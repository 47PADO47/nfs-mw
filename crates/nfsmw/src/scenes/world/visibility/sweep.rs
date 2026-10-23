//! One view's pass over the placed objects: the per-object rules, and the per-tile shortcut that skips the objects of
//! a tile the view cannot reach.

use blackbox_gfx::Instance;
use blackbox_gfx::reflection::distance::within;
use blackbox_scene::{Aabb, Frustum};
use blackbox_scenery::INSTANCE_TURNABLE;
use blackbox_scenery::LodView;
use blackbox_scenery::layout::LodRules;
use glam::{Mat4, Vec3, Vec4};

use super::{Camera, NOT_A_PROP, current_pose, near_caster, reaches_view, turned_pose};
use crate::scenes::world::detail::DetailState;
use crate::scenes::world::props::PropWorld;
use crate::scenes::world::resident::{CullEntry, CullTile, Placed};
use crate::scenes::world::scenery_groups::SceneryGroups;

/// How a tile's static objects stand to the view.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum TileView {
    /// None of them can be drawn or cast into the view.
    Outside,
    /// Some may be: each is tested.
    Partial,
    /// All are inside every frustum plane: none is tested against the frustum.
    Inside,
}

/// The state of one view's pass. Build it with [`Sweep::new`], feed it objects or tiles, drop it.
pub(super) struct Sweep<'a> {
    frustum: Frustum,
    planes: [Vec4; 6],
    camera: [f32; 3],
    caster_ray: Vec3,
    shadow_distance: f32,
    view: LodView,
    rules: &'a LodRules,
    props: &'a PropWorld,
    groups: Option<&'a SceneryGroups>,
    /// Drops every instance whose origin is farther than this from the camera (the rear-view mirror's far plane).
    far: Option<f32>,
    detail: Option<&'a mut DetailState>,
    out: &'a mut Vec<Instance>,
}

impl<'a> Sweep<'a> {
    /// A pass for `camera` into `out`, which it clears. `groups` decides which grouped instances are drawn (`None`
    /// draws none of them). `far` is applied to the origin of every instance this pass adds.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        camera: &Camera,
        rules: &'a LodRules,
        props: &'a PropWorld,
        groups: Option<&'a SceneryGroups>,
        (height, shadow_distance): (f32, f32),
        far: Option<f32>,
        detail: Option<&'a mut DetailState>,
        out: &'a mut Vec<Instance>,
    ) -> Self {
        out.clear();
        Self {
            frustum: Frustum::from_view_proj(&camera.view_proj),
            planes: planes_of(&camera.view_proj),
            camera: camera.position,
            caster_ray: Vec3::from_array(camera.light_dir).normalize_or_zero() * 80.0,
            shadow_distance,
            view: LodView {
                position: camera.position,
                forward: camera.forward,
                pixel_scale: LodView::pixel_scale_for(camera.fov_y_radians, height),
            },
            rules,
            props,
            groups,
            far,
            detail,
            out,
        }
    }

    /// Sweeps `tile`, skipping the objects it can tell are out of reach from the tile's box. `original` lists the
    /// authored shadow overlays too.
    pub fn visit_tile(&mut self, tile: &CullTile<'_>, original: bool) {
        let (placed, cull) = (tile.placed, tile.cull);
        let listed = |group: u16, original_shadow: bool| (original || !original_shadow) && tile.group_shown(group);
        if cull.entries.len() != placed.len() {
            // The list changed after its cull data was built: no shortcut.
            for p in placed.iter().filter(|p| listed(p.group, p.original_shadow)) {
                self.visit(p);
            }
            return;
        }
        let view = self.classify(cull.bounds.as_ref(), cull.origins.as_ref());
        if view == TileView::Outside {
            for &place in &cull.dynamic {
                let (p, entry) = (&placed[place as usize], &cull.entries[place as usize]);
                if listed(entry.group, entry.original_shadow) {
                    self.visit(p);
                }
            }
            return;
        }
        for (entry, p) in cull.entries.iter().zip(placed) {
            if !listed(entry.group, entry.original_shadow) {
                continue;
            }
            match entry.dynamic {
                true => self.visit(p),
                false => self.visit_static(entry, p, view == TileView::Inside),
            }
        }
    }

    /// How a tile whose static objects lie in `bounds`, with their origins in `origins`, stands to the view. A tile
    /// with no static object is `Outside`.
    pub fn classify(&self, bounds: Option<&Aabb>, origins: Option<&Aabb>) -> TileView {
        let (Some(bounds), Some(origins)) = (bounds, origins) else {
            return TileView::Outside;
        };
        if self.far.is_some() {
            let point = Vec3::from_array(self.camera);
            let nearest = point.clamp(origins.min, origins.max);
            if !within(point, nearest, self.far) {
                return TileView::Outside;
            }
        }
        if self.contains(bounds) {
            return TileView::Inside;
        }
        if self.frustum.intersects(bounds) || self.casts(bounds) {
            return TileView::Partial;
        }
        TileView::Outside
    }

    /// One object that never moves, from its cull data: only an object that survives reads its `Placed`.
    fn visit_static(&mut self, entry: &CullEntry, p: &Placed, in_frustum: bool) {
        if !self.admits(entry.group, entry.flags) || self.beyond_far(entry.position) {
            return;
        }
        let turned = entry.flags & INSTANCE_TURNABLE != 0 && self.groups.is_some_and(|g| g.turned(entry.group));
        if turned {
            self.visit(p);
            return;
        }
        let bounds = entry.bounds();
        let in_view = in_frustum || self.frustum.intersects(&bounds);
        if !in_view && !self.casts(&bounds) {
            return;
        }
        self.emit(p, p.transform, &bounds, in_view);
    }

    /// One object by its full record: the rules for any object, including props the prop world moves.
    pub fn visit(&mut self, p: &Placed) {
        if !self.admits(p.group, p.flags) {
            return;
        }
        let is_prop = p.prop_id != NOT_A_PROP;
        if !is_prop && self.beyond_far(p.transform.w_axis.truncate()) {
            return;
        }
        let Some((transform, bounds)) = current_pose(p, self.props) else {
            return;
        };
        // A turned race wall is mirrored along its own length (`docs/specs/race-barriers.md` section 3).
        let turned = p.flags & INSTANCE_TURNABLE != 0 && self.groups.is_some_and(|g| g.turned(p.group));
        let (transform, bounds) = match turned {
            true => turned_pose(transform, &bounds),
            false => (transform, bounds),
        };
        let in_view = self.frustum.intersects(&bounds);
        if !in_view && !self.casts(&bounds) {
            return;
        }
        self.emit(p, transform, &bounds, in_view);
    }

    /// The group and the detail setting both let an object with `group` and `flags` be drawn.
    fn admits(&self, group: u16, flags: u32) -> bool {
        if group != 0 && !self.groups.is_some_and(|groups| groups.admits(group)) {
            return false;
        }
        self.detail.as_ref().is_none_or(|d| d.settings.world.admits(flags))
    }

    fn beyond_far(&self, origin: Vec3) -> bool {
        self.far.is_some() && !within(Vec3::from_array(self.camera), origin, self.far)
    }

    /// A margin along the light ray admits casters beyond the last receiver cascade.
    fn casts(&self, bounds: &Aabb) -> bool {
        near_caster(bounds, self.camera, self.shadow_distance) && reaches_view(bounds, &self.frustum, self.caster_ray)
    }

    /// Whether `bounds` lies inside all six planes.
    fn contains(&self, bounds: &Aabb) -> bool {
        self.planes.iter().all(|plane| {
            let normal = plane.truncate();
            // The box corner furthest against the plane normal.
            let corner = Vec3::select(normal.cmpge(Vec3::ZERO), bounds.min, bounds.max);
            normal.dot(corner) + plane.w >= 0.0
        })
    }

    /// Picks the LOD of an object that passed the cull and adds it. `in_view` is whether `bounds` meets the frustum;
    /// when it does not, the object is a shadow caster.
    fn emit(&mut self, p: &Placed, transform: Mat4, bounds: &Aabb, in_view: bool) {
        let slots = p.lods.map(|m| m.is_some());
        let position = transform.w_axis.truncate().to_array();
        // Keep the player's projected-size LOD for visible geometry. A shadow must not change it.
        let visible_lod = match in_view {
            false => None,
            true => match self.detail.as_deref_mut() {
                Some(detail) => detail.choose(p, position, &self.view, self.rules),
                None => self.rules.choose(&self.view, position, p.radius, p.flags, slots, p.detailed),
            },
        };
        if in_view && self.detail.is_some() && visible_lod.is_none() {
            return;
        }
        let slot = match visible_lod {
            Some(slot) => slot,
            None => {
                if in_view && !self.casts(bounds) {
                    return;
                }
                // Off-camera LOD must not disappear merely because the object is behind the player.
                let shadow_view = LodView {
                    forward: (Vec3::from_array(position) - Vec3::from_array(self.camera))
                        .normalize_or_zero()
                        .to_array(),
                    ..self.view
                };
                let Some(slot) = self.rules.choose(&shadow_view, position, p.radius, p.flags, slots, p.detailed) else {
                    return;
                };
                slot
            }
        };
        let start = self.out.len();
        let is_prop = p.prop_id != NOT_A_PROP;
        if is_prop && self.props.model_instances(p.prop_id, transform, self.out) {
            self.limit_from(start);
            return;
        }
        if let Some(mesh) = p.lods[slot] {
            self.out.push(Instance::keyed(mesh, transform, p.key).with_flipped_winding(p.flip_winding()));
        }
        if is_prop {
            self.limit_from(start);
        }
    }

    /// Drops the instances from `start` on whose origin is beyond the far limit, keeping the order of the rest. A
    /// prop's pose and fragments are known only when they are added, so they are checked then.
    fn limit_from(&mut self, start: usize) {
        if self.far.is_none() {
            return;
        }
        let camera = Vec3::from_array(self.camera);
        let mut kept = start;
        for i in start..self.out.len() {
            if !within(camera, self.out[i].transform.w_axis.truncate(), self.far) {
                continue;
            }
            self.out.swap(kept, i);
            kept += 1;
        }
        self.out.truncate(kept);
    }
}

/// The six planes of `view_proj`, as `Frustum::from_view_proj` makes them (`n · p + d >= 0` inside).
fn planes_of(view_proj: &Mat4) -> [Vec4; 6] {
    let rows = [view_proj.row(0), view_proj.row(1), view_proj.row(2), view_proj.row(3)];
    let normalize = |p: Vec4| p / p.truncate().length().max(f32::EPSILON);
    [
        normalize(rows[3] + rows[0]),
        normalize(rows[3] - rows[0]),
        normalize(rows[3] + rows[1]),
        normalize(rows[3] - rows[1]),
        normalize(rows[2]),
        normalize(rows[3] - rows[2]),
    ]
}

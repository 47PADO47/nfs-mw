//! The road image has its own authored models, visibility and local reflection transform.
use super::super::{
    props::PropWorld,
    resident::{ReflectionPlaced, SectionResources},
    visibility::{Camera, NOT_A_PROP},
};
use super::cull::RoadCull;
use blackbox_gfx::Instance;
use blackbox_scene::{Aabb, Frustum};
use blackbox_scenery::LodView;
use glam::Mat4;

/// Above this capture height the road view keeps small geometry and culls the reflected mesh bounds.
const ENHANCED_HEIGHT: f32 = 240.0;

/// One capture's selection rules.
struct Pass<'a> {
    frustum: Frustum,
    view: LodView,
    props: &'a PropWorld,
    enhanced: bool,
}

impl<'a> Pass<'a> {
    fn new(camera: &Camera, props: &'a PropWorld, height: f32) -> Self {
        Self {
            frustum: Frustum::from_view_proj(&camera.view_proj),
            view: LodView {
                position: camera.position,
                forward: camera.forward,
                pixel_scale: LodView::pixel_scale_for(camera.fov_y_radians, height),
            },
            props,
            enhanced: height > ENHANCED_HEIGHT,
        }
    }

    /// One placement by its full record, including the props the prop world moves or hides.
    fn visit(&self, p: &ReflectionPlaced, out: &mut Vec<Instance>) {
        // A placement that is not a prop has no entry in the prop world, so it skips the lookups.
        if p.prop_id == NOT_A_PROP {
            self.emit(p, p.transform, p.bounds, None, out);
            return;
        }
        if self.props.hidden(p.prop_id) {
            return;
        }
        let (transform, bounds) = self.props.pose(p.prop_id).unwrap_or((p.transform, p.bounds));
        self.emit(p, transform, bounds, None, out);
    }

    /// Picks the mesh of `p` posed at `transform` with `bounds` and adds it. `reflected` is the bounds of its authored
    /// mesh under the reflection matrix when the cull data already has them.
    fn emit(
        &self,
        p: &ReflectionPlaced,
        transform: Mat4,
        bounds: Aabb,
        reflected: Option<Aabb>,
        out: &mut Vec<Instance>,
    ) {
        let flipped = scenery_transform(transform, p.flags);
        let selected = match self.enhanced {
            true => {
                let slot = authored_slot(p.flags);
                let bounds = reflected
                    .unwrap_or_else(|| p.local_bounds[slot].map(|b| b.transformed(&flipped)).unwrap_or(bounds));
                if !self.frustum.intersects(&bounds) {
                    return;
                }
                let radius = (bounds.max - bounds.min).length() * 0.5;
                if self.view.projected_size(bounds.center().to_array(), radius) < 2 {
                    return;
                }
                slot
            }
            false => {
                if !self.frustum.intersects(&bounds) {
                    return;
                }
                let size = self.view.projected_size(transform.w_axis.truncate().to_array(), p.radius + 6.0);
                let Some(slot) = slot(p.flags, size) else { return };
                slot
            }
        };
        let Some(mesh) = p.lods[selected] else { return };
        out.push(Instance::keyed(mesh, flipped, p.key).with_flipped_winding(p.flip_winding()));
    }

    /// One set of placements: skipped but for its props when the view cannot reach it, else swept with the per-entry
    /// shortcut.
    fn visit_set(&self, set: &SectionResources, out: &mut Vec<Instance>) {
        let placed = &set.reflection_placed;
        let cull = set.reflection_cull.get_or_init(|| RoadCull::build(placed));
        if cull.entries.len() != placed.len() {
            // The list changed after its cull data was built: no shortcut.
            for p in placed {
                self.visit(p, out);
            }
            return;
        }
        if !cull.bounds.as_ref().is_some_and(|bounds| self.frustum.intersects(bounds)) {
            for &place in &cull.dynamic {
                self.visit(&placed[place as usize], out);
            }
            return;
        }
        for (entry, p) in cull.entries.iter().zip(placed) {
            if entry.dynamic {
                self.visit(p, out);
                continue;
            }
            let seen = match self.enhanced {
                true => &entry.reflected,
                false => &entry.bounds,
            };
            if !self.frustum.intersects(seen) {
                continue;
            }
            self.emit(p, p.transform, p.bounds, Some(entry.reflected), out);
        }
    }
}

/// The road view of the resident sets (`Residency::reflection_sets`), unsorted: the caller appends more and sorts once.
pub(super) fn scenery_sets<'a>(
    sets: impl Iterator<Item = &'a SectionResources>,
    camera: &Camera,
    props: &PropWorld,
    height: f32,
    out: &mut Vec<Instance>,
) {
    let pass = Pass::new(camera, props, height);
    out.clear();
    for set in sets {
        pass.visit_set(set, out);
    }
}

/// The per-object path over a plain list, the reference the set path must match. The result is sorted by mesh.
#[cfg(test)]
pub(super) fn scenery<'a>(
    placed: impl Iterator<Item = &'a ReflectionPlaced>,
    camera: &Camera,
    props: &PropWorld,
    height: f32,
    out: &mut Vec<Instance>,
) {
    let pass = Pass::new(camera, props, height);
    out.clear();
    for p in placed {
        pass.visit(p, out);
    }
    super::super::visibility::sort_by_mesh(out);
}

fn slot(flags: u32, raw_size: i32) -> Option<usize> {
    if raw_size < 2 {
        return None;
    }
    let size = raw_size.saturating_add(if flags & 0x0200_0000 != 0 { 10 } else { 0 });
    if size < 32 {
        return None;
    }
    Some(authored_slot(flags))
}

pub(super) fn authored_slot(flags: u32) -> usize {
    if flags & 0x80 != 0 {
        return 2;
    }
    if flags & 0x0100_0100 != 0 {
        return 0;
    }
    3
}

pub(super) fn scenery_transform(mut transform: Mat4, flags: u32) -> Mat4 {
    if flags & 0x200 != 0 {
        return Mat4::IDENTITY;
    }
    transform.z_axis.z = -transform.z_axis.z;
    transform
}

/// The body callback keeps its own origin and flips its complete local Z axis.
/// Call on a body/root pose; child/wheel composition remains the vehicle adapter's job.
pub(super) fn vehicle_root(mut transform: Mat4) -> Mat4 {
    transform.z_axis = -transform.z_axis;
    transform
}

#[cfg(test)]
mod tests;

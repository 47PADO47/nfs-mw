//! Choose the current reflection views of the faces being drawn in one scenery traversal.
//!
//! The traversal goes tile by tile like the main and rear views (`visibility.rs`): a tile whose box no drawn face can
//! see costs one test, and an object a face cannot see costs one read of its compact cull entry. The large `Placed`
//! record is read only for an object that survives.
use blackbox_gfx::Instance;
use blackbox_gfx::reflection::distance::within;
use blackbox_scene::{Aabb, Frustum};
use blackbox_scenery::{LodView, layout::LodRules};
use glam::Vec3;

use super::{
    props::PropWorld,
    resident::{CullEntry, CullTile, Placed},
    visibility,
};

/// Fractured prop instances keep their allocation between captures.
#[derive(Default)]
pub(super) struct Scratch {
    parts: Vec<Instance>,
}

/// Which objects the capture lists at all, before any face looks at them.
#[derive(Clone, Copy)]
pub(super) struct Listing {
    /// The authored shadow overlays are listed too.
    pub original_shadows: bool,
    /// The Original PC capture keeps its documented origin-distance selection: only objects whose origin is within
    /// this distance of the capture origin are listed. `None` lists them all.
    pub origin_far: Option<f32>,
}

/// One capture's selection rules: what the drawn faces see and which LOD each picks.
struct Pass<'a> {
    frustums: [Frustum; 6],
    views: [LodView; 6],
    drawn: u8,
    rules: &'a LodRules,
    props: &'a PropWorld,
    far: Option<f32>,
    listing: Listing,
    origin: Vec3,
}

impl<'a> Pass<'a> {
    fn new(
        cameras: &[visibility::Camera; 6],
        rules: &'a LodRules,
        props: &'a PropWorld,
        height: f32,
        far: Option<f32>,
        listing: Listing,
        drawn: u8,
    ) -> Self {
        Self {
            frustums: cameras.each_ref().map(|c| Frustum::from_view_proj(&c.view_proj)),
            views: cameras.each_ref().map(|c| LodView {
                position: c.position,
                forward: c.forward,
                pixel_scale: LodView::pixel_scale_for(c.fov_y_radians, height),
            }),
            drawn,
            rules,
            props,
            far,
            listing,
            origin: cameras[0].position.into(),
        }
    }

    fn draws(&self, face: usize) -> bool {
        (self.drawn >> face) & 1 != 0
    }

    /// The objects the capture lists: the overlay rule and the Original PC origin distance.
    fn lists(&self, original_shadow: bool, position: Vec3) -> bool {
        (self.listing.original_shadows || !original_shadow) && within(self.origin, position, self.listing.origin_far)
    }

    /// Whether the far limit lets an object with `bounds` in.
    fn near_enough(&self, bounds: &Aabb) -> bool {
        !self.far.is_some_and(|far| bounds.distance_to(self.origin) > far)
    }

    /// Whether some drawn face's frustum meets `bounds`.
    fn seen(&self, bounds: &Aabb) -> bool {
        (0..6).any(|face| self.draws(face) && self.frustums[face].intersects(bounds))
    }

    /// Whether any static object of a tile with these boxes can reach a drawn face. A tile with no static object
    /// cannot.
    fn reaches(&self, bounds: Option<&Aabb>, origins: Option<&Aabb>) -> bool {
        let (Some(bounds), Some(origins)) = (bounds, origins) else {
            return false;
        };
        if self.listing.origin_far.is_some() {
            let nearest = self.origin.clamp(origins.min, origins.max);
            if !within(self.origin, nearest, self.listing.origin_far) {
                return false;
            }
        }
        self.near_enough(bounds) && self.seen(bounds)
    }

    /// Whether a static object can still be selected, from its cull entry alone.
    fn admits(&self, entry: &CullEntry) -> bool {
        // Reflection views have always omitted inactive scenery groups.
        if entry.group != 0 || !self.lists(entry.original_shadow, entry.position) {
            return false;
        }
        let bounds = entry.bounds();
        self.near_enough(&bounds) && self.seen(&bounds)
    }

    /// One object by its full record: the rules for any object, including props the prop world moves.
    fn visit(&self, p: &Placed, faces: &mut [Vec<Instance>; 6], parts: &mut Vec<Instance>) {
        // Reflection views have always omitted inactive scenery groups.
        if p.group != 0 || !self.lists(p.original_shadow, p.transform.w_axis.truncate()) {
            return;
        }
        let Some((transform, bounds)) = visibility::current_pose(p, self.props) else { return };
        if !self.near_enough(&bounds) {
            return;
        }
        let position = transform.w_axis.truncate().to_array();
        let slots = p.lods.map(|mesh| mesh.is_some());
        let selected: [Option<usize>; 6] = std::array::from_fn(|face| {
            if !self.draws(face) || !self.frustums[face].intersects(&bounds) {
                return None;
            }
            self.rules.choose(&self.views[face], position, p.radius, p.flags, slots, p.detailed)
        });
        if selected.iter().all(Option::is_none) {
            return;
        }
        parts.clear();
        let replaced = p.prop_id != visibility::NOT_A_PROP && self.props.model_instances(p.prop_id, transform, parts);
        for (face, slot) in faces.iter_mut().zip(selected) {
            let Some(slot) = slot else { continue };
            if replaced {
                face.extend_from_slice(parts);
                continue;
            }
            if let Some(mesh) = p.lods[slot] {
                face.push(Instance::keyed(mesh, transform, p.key).with_flipped_winding(p.flip_winding()));
            }
        }
    }

    /// One tile: skipped but for its props when no drawn face can reach it, else swept with the per-entry shortcut.
    fn visit_tile(&self, tile: &CullTile<'_>, faces: &mut [Vec<Instance>; 6], parts: &mut Vec<Instance>) {
        let (placed, cull) = (tile.placed, tile.cull);
        if cull.entries.len() != placed.len() {
            // The list changed after its cull data was built: no shortcut.
            for p in placed {
                self.visit(p, faces, parts);
            }
            return;
        }
        if !self.reaches(cull.bounds.as_ref(), cull.origins.as_ref()) {
            for &place in &cull.dynamic {
                self.visit(&placed[place as usize], faces, parts);
            }
            return;
        }
        for (entry, p) in cull.entries.iter().zip(placed) {
            if !entry.dynamic && !self.admits(entry) {
                continue;
            }
            self.visit(p, faces, parts);
        }
    }

    /// Empties the lists of the faces being drawn; the others keep theirs.
    fn clear(&self, faces: &mut [Vec<Instance>; 6]) {
        for (index, face) in faces.iter_mut().enumerate() {
            if self.draws(index) {
                face.clear();
            }
        }
    }
}

/// The tiles of the residency (`Residency::cull_tiles`), selected for each face. The authored-overlay and Original PC
/// origin rules come in `listing`. Each face still chooses its own LOD and frustum membership. Only the faces in the
/// `drawn` mask are selected and replaced; the others keep their lists. A finite `far` also drops scenery whose bounds
/// lie beyond that distance from the capture origin.
#[allow(clippy::too_many_arguments)]
pub(super) fn collect_tiles<'a>(
    tiles: impl Iterator<Item = CullTile<'a>>,
    listing: Listing,
    cameras: &[visibility::Camera; 6],
    rules: &LodRules,
    props: &PropWorld,
    height: f32,
    far: Option<f32>,
    drawn: u8,
    faces: &mut [Vec<Instance>; 6],
    scratch: &mut Scratch,
) {
    let _trace = blackbox_trace::scope("Reflection visibility: scenery traversal");
    let pass = Pass::new(cameras, rules, props, height, far, listing, drawn);
    pass.clear(faces);
    for tile in tiles {
        pass.visit_tile(&tile, faces, &mut scratch.parts);
    }
}

/// The per-object path over a plain list of objects the caller has already filtered, the reference the tile path must
/// match.
#[cfg(test)]
#[allow(clippy::too_many_arguments)]
pub(super) fn collect<'a>(
    placed: impl Iterator<Item = &'a Placed>,
    cameras: &[visibility::Camera; 6],
    rules: &LodRules,
    props: &PropWorld,
    height: f32,
    far: Option<f32>,
    drawn: u8,
    faces: &mut [Vec<Instance>; 6],
    scratch: &mut Scratch,
) {
    let listing = Listing { original_shadows: true, origin_far: None };
    let pass = Pass::new(cameras, rules, props, height, far, listing, drawn);
    pass.clear(faces);
    for p in placed {
        pass.visit(p, faces, &mut scratch.parts);
    }
}

#[cfg(test)]
#[path = "reflections_visibility_tests.rs"]
mod tests;

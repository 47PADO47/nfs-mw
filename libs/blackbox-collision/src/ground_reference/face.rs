//! Independent selected-face history over an externally prepared candidate list.
use super::{GroundFace, GroundPlane, GroundReferenceSettings, geometry};
use crate::math::{Vec3, dot, sub};

#[derive(Clone, Debug, Default)]
pub struct GroundFaceCache {
    face: Option<GroundFace>,
    plane: GroundPlane,
}

impl GroundFaceCache {
    pub fn update_global(
        &mut self,
        world: &crate::CollisionWorld,
        position: Vec3,
        settings: &super::PointQuerySettings,
        group_enabled: impl FnMut(u16) -> bool,
    ) -> GroundPlane {
        super::global::update(&mut self.face, &mut self.plane, world, position, settings, group_enabled)
    }
    pub fn clear(&mut self) {
        *self = Self::default();
    }
    pub fn plane(&self) -> GroundPlane {
        self.plane
    }
    pub fn update(
        &mut self,
        position: Vec3,
        candidates: &[GroundFace],
        settings: &GroundReferenceSettings,
    ) -> GroundPlane {
        select(&mut self.face, &mut self.plane, candidates, position, settings);
        self.plane
    }
}

pub(super) fn select(
    face: &mut Option<GroundFace>,
    plane: &mut GroundPlane,
    candidates: &[GroundFace],
    position: Vec3,
    settings: &GroundReferenceSettings,
) {
    let was_valid = plane.valid;
    let same = was_valid && face.is_some_and(|face| geometry::contains(face.triangle.pts, position));
    if !same || !settings.reuse_face {
        let query = [position[0], position[1] + settings.vertical_tolerance, position[2]];
        let mut selected = None;
        let mut best = f32::INFINITY;
        for &face in candidates {
            if !geometry::contains(face.triangle.pts, query) {
                continue;
            }
            let normal = geometry::normal(face.triangle.pts, settings.maximum_normal_y);
            let height = geometry::height(face.triangle.pts[0], normal, query);
            let distance = query[1] - height;
            if distance <= settings.minimum_vertical_separation || distance >= best {
                continue;
            }
            best = distance;
            selected = Some(face);
            if face.triangle.flags & settings.continuation_mask == 0 {
                break;
            }
        }
        plane.valid = selected.is_some();
        plane.surface_hash = selected.map_or(0, |face| face.surface_hash);
        if let Some(selected_face) = selected {
            *face = Some(selected_face);
        }
        if !plane.valid && settings.preserve_face && was_valid {
            plane.valid = true;
        }
    }
    if !plane.valid {
        plane.separation = -1.0;
        return;
    }
    let Some(face) = *face else { return };
    // The search returns whether the point left its previous face, even when forced.
    if !same {
        plane.normal = geometry::normal(face.triangle.pts, settings.maximum_normal_y);
    }
    plane.separation = dot(sub(face.triangle.pts[0], position), plane.normal);
}

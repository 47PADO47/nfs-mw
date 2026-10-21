//! Native ordered compound car contacts; `docs/specs/vehicle-pair-dispatch.md`.
use super::CarSim;
use super::pair::CarHit;
use crate::scenes::world::object_geometry::{BodyGeometryFrame, object_shape_at, object_shape_count};

impl CarSim {
    /// Emit each successful primitive reaction immediately, in native visitation order.
    pub fn collide_with_each(&mut self, other: &mut CarSim, mut emit: impl FnMut(CarHit, &CarSim, &CarSim)) -> bool {
        let radius = self.collision_radius() + other.collision_radius();
        if self.body().position.distance_squared(other.body().position) > (radius * radius).max(1.0) {
            return false;
        }
        // Both hooks run for this admitted body pair, before narrow phase.
        let admits_a = self.reset_collision.admit();
        let admits_b = other.reset_collision.admit();
        if !admits_a || !admits_b {
            return false;
        }
        let geometry_a = self.geometry.clone();
        let geometry_b = other.geometry.clone();
        let frame_a =
            geometry_a.as_ref().map(|g| BodyGeometryFrame { geometry: &g.body, previous: self.previous_contact_pose });
        let frame_b =
            geometry_b.as_ref().map(|g| BodyGeometryFrame { geometry: &g.body, previous: other.previous_contact_pose });
        let mut reacted = false;
        for i in 0..object_shape_count(frame_a) {
            let Some(a) = object_shape_at(self.body(), frame_a, i) else { continue };
            for j in 0..object_shape_count(frame_b) {
                let Some(b) = object_shape_at(other.body(), frame_b, j) else { continue };
                let Some(contact) = a.contact(&b) else { continue };
                let Some(hit) = self.resolve_pair_contact(other, contact, a.surface, b.surface) else { continue };
                reacted = true;
                emit(hit, self, other);
            }
        }
        reacted
    }

    /// Tests may inspect a strongest-hit summary; every physical callback still executes.
    #[cfg(test)]
    pub fn collide_with(&mut self, other: &mut CarSim) -> Option<CarHit> {
        let mut strongest: Option<CarHit> = None;
        self.collide_with_each(other, |hit, _, _| {
            if strongest.as_ref().is_none_or(|old| hit.impulse > old.impulse) {
                strongest = Some(hit);
            }
        });
        strongest
    }
}

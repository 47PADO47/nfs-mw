//! The towing side of a car: how a hitched trailer lengthens it for the other cars, and its parts.

use blackbox_render::Instance;
use blackbox_roads::Body;
use glam::{Vec2, Vec3};

use super::super::trailer::TrailerCar;
use super::AiCar;
use crate::scenes::world::drive::CarSim;

/// The body other cars avoid for a tractor with a hitched trailer: one box from the tractor's nose to the
/// trailer's tail. `trailer_forward` and `trailer_half` (width, length) are the trailer's.
fn rig_body(tractor: &Body, trailer_at: Vec3, trailer_forward: Vec2, trailer_half: Vec2) -> Body {
    let flat = |v: Vec3| Vec2::new(v.x, v.z);
    let nose = flat(tractor.position) + tractor.forward * tractor.half_length;
    let tail = flat(trailer_at) - trailer_forward * trailer_half.y;
    let middle = (nose + tail) * 0.5;
    let along = (nose - tail).try_normalize().unwrap_or(tractor.forward);
    Body {
        position: Vec3::new(middle.x, tractor.position.y, middle.y),
        velocity: tractor.velocity,
        forward: along,
        half_width: tractor.half_width.max(trailer_half.x),
        half_length: nose.distance(tail) * 0.5,
    }
}

impl AiCar {
    /// The car as the other cars' trails avoid it: with its trailer when that is hitched.
    pub fn avoidable(&self) -> Body {
        let body = self.body();
        let Some(trailer) = self.trailer.as_ref().filter(|t| t.is_hitched()) else { return body };
        let half = trailer.half_dimensions();
        rig_body(&body, trailer.physics_position(), trailer.forward(), Vec2::new(half.x, half.z))
    }

    /// The body of a trailer that came loose, which is an obstacle of its own.
    pub fn loose_trailer_body(&self) -> Option<Body> {
        let trailer = self.trailer.as_ref().filter(|t| !t.is_hitched())?;
        let half = trailer.half_dimensions();
        Some(Body {
            position: trailer.physics_position(),
            velocity: trailer.velocity(),
            forward: trailer.forward(),
            half_width: half.x,
            half_length: half.z,
        })
    }

    /// The length the trailer adds to the car, metres (0 without one).
    pub fn trailer_length(&self) -> f32 {
        self.trailer.as_ref().map_or(0.0, |t| 2.0 * t.half_dimensions().z)
    }

    /// Whether a hitched trailer is part of the car: the two do not collide with each other.
    pub fn is_coupled(&self) -> bool {
        self.trailer.as_ref().is_some_and(|t| t.is_hitched())
    }

    /// The physics of the car and of its trailer, for hits between cars.
    pub(in crate::scenes::world::ai) fn sims_mut(&mut self) -> (&mut CarSim, Option<&mut CarSim>) {
        (&mut self.sim, self.trailer.as_mut().map(|t| t.sim_mut()))
    }

    /// Appends the instances of the car and of its trailer.
    pub fn instances(&self, alpha: f32, out: &mut Vec<Instance>) {
        self.rig.instances(&self.pose(alpha), out);
        if let Some(trailer) = &self.trailer {
            trailer.rig().instances(&trailer.pose(alpha), out);
        }
    }

    /// Follows which part of the car the player can see: `in_view` is asked for the position (render space)
    /// of the car and of its trailer.
    pub fn note_views(&mut self, dt: f32, in_view: impl Fn(Vec3) -> bool) {
        self.note_view(in_view(self.current.position), dt);
        if let Some(trailer) = self.trailer.as_mut() {
            trailer.note_view(in_view(trailer.position()), dt);
        }
    }

    /// A trailer that came loose, if the car has one.
    pub fn loose_trailer(&self) -> Option<&TrailerCar> {
        self.trailer.as_ref().filter(|t| !t.is_hitched())
    }

    /// Takes a loose trailer off the road.
    pub fn drop_trailer(&mut self) {
        self.trailer = None;
    }

    /// The car's trailer, hitched or loose.
    pub fn trailer(&self) -> Option<&TrailerCar> {
        self.trailer.as_ref()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tractor() -> Body {
        Body {
            position: Vec3::new(10.0, 1.0, 0.0),
            velocity: Vec3::new(0.0, 0.0, 20.0),
            forward: Vec2::Y,
            half_width: 1.3,
            half_length: 3.6,
        }
    }

    #[test]
    fn a_straight_rig_is_one_box_from_the_nose_to_the_tail() {
        // Tractor centre at z 0, nose at 3.6; trailer centre 9.2 m behind, half length 5.5, so the tail is at -14.7.
        let rig = rig_body(&tractor(), Vec3::new(10.0, 1.0, -9.2), Vec2::Y, Vec2::new(1.4, 5.5));
        assert!((rig.half_length - (3.6 + 14.7) / 2.0).abs() < 1e-4, "{}", rig.half_length);
        assert!((rig.position.z - (3.6 - 14.7) / 2.0).abs() < 1e-4 && (rig.position.x - 10.0).abs() < 1e-4);
        assert_eq!(rig.forward, Vec2::Y);
        assert_eq!(rig.half_width, 1.4);
        assert_eq!(rig.velocity, tractor().velocity);
    }

    #[test]
    fn a_bent_rig_still_covers_the_nose_and_the_tail() {
        // The trailer swings 30 degrees to the left of the tractor's line.
        let trailer_forward = Vec2::new(-0.5, 0.866);
        let trailer_at = Vec3::new(10.0, 1.0, -9.2);
        let rig = rig_body(&tractor(), trailer_at, trailer_forward, Vec2::new(1.4, 5.5));
        let tail = Vec2::new(trailer_at.x, trailer_at.z) - trailer_forward * 5.5;
        let nose = Vec2::new(10.0, 3.6);
        let ends = [nose, tail];
        let centre = Vec2::new(rig.position.x, rig.position.z);
        for end in ends {
            let along = (end - centre).dot(rig.forward);
            assert!((along.abs() - rig.half_length).abs() < 1e-3, "{along} vs {}", rig.half_length);
        }
    }
}

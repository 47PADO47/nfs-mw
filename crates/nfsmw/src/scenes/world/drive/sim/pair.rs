//! Two cars hitting each other.
//! Spec: `docs/specs/vehicle-rigid-body.md` (§6).

use blackbox_vehicle::rigid_body::{BodyContact, BodyHit, Obb, obb_contact, react_bodies, separate};
use glam::Vec3;

use super::CarSim;

/// Restitution of a car against a car. The traffic rigid body spec has 0.1 for objects.
const RESTITUTION: f32 = 0.15;
/// Friction between two cars: static and kinetic.
const FRICTION: (f32, f32) = (0.6, 0.5);

/// What a hit did.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CarHit {
    /// Normal impulse, N s.
    pub impulse: f32,
    /// Where the cars touched (physics space).
    pub point: Vec3,
}

impl CarSim {
    /// The collision box in physics space.
    pub fn collision_box(&self) -> Obb {
        let v = &self.vehicle;
        Obb { centre: v.position(), axes: v.rotation(), half: v.spec().dimension }
    }

    /// Tests this car against `other` and, when their boxes overlap, pushes them apart and changes their
    /// velocities. Returns the hit, if the cars were approaching.
    pub fn collide_with(&mut self, other: &mut CarSim) -> Option<CarHit> {
        let contact = obb_contact(&self.collision_box(), &other.collision_box())?;
        let (a, b) = (self.vehicle.body_mut(), other.vehicle.body_mut());
        separate(a, b, contact.normal, contact.overlap);
        let hit = BodyHit { restitution: RESTITUTION, inertia_scale: Vec3::ONE };
        let body = BodyContact {
            point: contact.point,
            normal: contact.normal,
            overlap: contact.overlap,
            friction_static: FRICTION.0,
            friction_kinetic: FRICTION.1,
        };
        let reaction = react_bodies(a, b, &body, &hit, &hit)?;
        self.vehicle.notify_collision(reaction.impulse / 1000.0);
        other.vehicle.notify_collision(reaction.impulse / 1000.0);
        Some(CarHit { impulse: reaction.impulse, point: contact.point })
    }
}

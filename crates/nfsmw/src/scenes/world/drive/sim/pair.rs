//! Two cars hitting each other.
//! Spec: `docs/specs/vehicle-rigid-body.md` (§6).

use crate::scenes::world::object_geometry::{BodyGeometryFrame, object_shapes};
use blackbox_collision::bounds_flags;
use blackbox_vehicle::rigid_body::{BodyContact, BodyHit, react_bodies, separate};
use blackbox_vehicle::steering::SteeringDevice;
use glam::Vec3;

use super::CarSim;

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
    #[cfg(test)]
    pub fn collision_box(&self) -> blackbox_vehicle::rigid_body::Obb {
        let v = &self.vehicle;
        blackbox_vehicle::rigid_body::Obb { centre: v.position(), axes: v.rotation(), half: v.spec().dimension }
    }

    /// Encloses padded child shapes, which can extend past the original root box.
    pub fn collision_radius(&self) -> f32 {
        let Some(geometry) = &self.geometry else { return self.vehicle.body().radius() };
        geometry
            .body
            .primitives
            .iter()
            .filter(|p| p.collides(bounds_flags::PRIM_VS_OBJECTS))
            .map(|p| p.centre.length() + p.half.length())
            .fold(0.0_f32, f32::max)
    }

    pub fn collision_sweep_radius(&self) -> f32 {
        self.collision_radius() + self.vehicle.position().distance(self.previous_contact_pose.0)
    }

    fn geometry_frame(&self) -> Option<BodyGeometryFrame<'_>> {
        Some(BodyGeometryFrame { geometry: &self.geometry.as_ref()?.body, previous: self.previous_contact_pose })
    }

    /// Tests this car against `other` and, when their boxes overlap, pushes them apart and changes their
    /// velocities. Returns the hit, if the cars were approaching.
    pub fn collide_with(&mut self, other: &mut CarSim) -> Option<CarHit> {
        let a = object_shapes(self.body(), self.geometry_frame());
        let b = object_shapes(other.body(), other.geometry_frame());
        let contact = a
            .iter()
            .flat_map(|a| b.iter().filter_map(move |b| a.contact(b)))
            .max_by(|a, b| a.overlap.total_cmp(&b.overlap))?;
        let material_a = self.vehicle.body().spec().objects;
        let material_b = other.vehicle.body().spec().objects;
        let [friction_static, friction_kinetic] = material_a.friction_with(&material_b);
        let grounded = self.vehicle.wheels_on_ground() > 2 && other.vehicle.wheels_on_ground() > 2;
        let hit_a = self.object_hit(contact.point, contact.normal, grounded);
        let hit_b = other.object_hit(contact.point, -contact.normal, grounded);
        let (a, b) = (self.vehicle.body_mut(), other.vehicle.body_mut());
        separate(a, b, contact.normal, contact.overlap);
        let body = BodyContact {
            point: contact.point,
            normal: contact.normal,
            overlap: contact.overlap,
            friction_static,
            friction_kinetic,
        };
        let reaction = react_bodies(a, b, &body, &hit_a, &hit_b)?;
        self.vehicle.notify_collision(reaction.impulse / 1000.0);
        other.vehicle.notify_collision(reaction.impulse / 1000.0);
        Some(CarHit { impulse: reaction.impulse, point: contact.point })
    }

    /// Temporary parameters selected for this contact, including the grounded player's roll centre.
    fn object_hit(&self, point: Vec3, normal: Vec3, grounded: bool) -> BodyHit {
        let body = self.vehicle.body();
        let material = body.spec().objects;
        let mut hit = BodyHit {
            restitution: material.restitution(body.rotation().transpose() * normal),
            inertia_scale: material.inertia_scale,
            mass_scale: material.mass_scale,
            ..Default::default()
        };
        if grounded && self.vehicle.config.steering_device != SteeringDevice::Ai {
            let mut cog = body.cog();
            cog.y = point.y - body.position.y - 0.125;
            hit.centre = Some(body.position + body.rotation() * cog);
        }
        hit
    }
}

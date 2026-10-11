//! Body ground queries and response; geometry supplies its own ordered support points.

use super::{BodyState, ContactParams, PlaneContact, RigidBody};

#[cfg(test)]
mod tests;
use crate::ground::Ground;
use glam::Vec3;

pub const BOX_CORNERS: [Vec3; 8] = [
    Vec3::new(-1.0, -1.0, 1.0),
    Vec3::new(1.0, -1.0, 1.0),
    Vec3::new(1.0, -1.0, -1.0),
    Vec3::new(-1.0, -1.0, -1.0),
    Vec3::new(-1.0, 1.0, 1.0),
    Vec3::new(1.0, 1.0, 1.0),
    Vec3::new(1.0, 1.0, -1.0),
    Vec3::new(-1.0, 1.0, -1.0),
];

#[derive(Clone, Copy, Debug)]
pub struct GroundContact {
    pub point: Vec3,
    pub normal: Vec3,
    pub depth: f32,
    pub friction: f32,
}

impl RigidBody {
    pub fn ground_contact(&self, point: Vec3, ground: &dyn Ground, dt: f32) -> Option<GroundContact> {
        if self.spec().no_ground_collisions || self.state != BodyState::Awake {
            return None;
        }
        let extent = (self.rotation().transpose() * Vec3::Y * self.dimension()).length();
        let arm = point - self.world_cog();
        let downward = self.angular_velocity.x * arm.z - self.angular_velocity.z * arm.x - self.linear_velocity.y;
        let planar = self.linear_velocity.x.hypot(self.linear_velocity.z);
        let ceiling = self.position.y + extent;
        let lift = (planar * dt + downward.max(0.0) * dt + extent).max(0.25).min(ceiling - point.y);
        if !lift.is_finite() || lift <= 0.0 {
            return None;
        }
        let hit = ground.hit(point + Vec3::Y * lift, Vec3::NEG_Y, lift)?;
        let normal = hit.normal * hit.normal.y.signum();
        let depth = (lift - hit.distance) * normal.y;
        if !normal.is_finite() || !depth.is_finite() || depth <= 0.000001 {
            return None;
        }
        Some(GroundContact { point, normal, depth, friction: ground.body_friction(hit.surface.tag) })
    }

    /// The first successful contact changes velocities; the deepest retained penetration moves the body.
    pub fn resolve_ground(&mut self, contacts: &mut [GroundContact]) {
        contacts.sort_by(|a, b| b.depth.total_cmp(&a.depth));
        let spec = *self.spec();
        for c in contacts.iter() {
            let local = self.rotation().transpose() * c.normal;
            let params = ContactParams {
                restitution: (local * spec.ground_elasticity).length(),
                inertia_scale: spec.ground_moment_scale,
            };
            let plane = PlaneContact {
                point: c.point,
                normal: c.normal,
                friction_static: spec.ground_friction[0] * c.friction,
                friction_kinetic: spec.ground_friction[1].min(spec.ground_friction[0]) * c.friction,
            };
            if self.react_plane(&plane, &params).is_some() {
                break;
            }
        }
        if let Some(c) = contacts.first() {
            self.position += c.normal * c.depth;
        }
    }
}

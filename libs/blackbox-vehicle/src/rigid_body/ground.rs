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

#[derive(Clone, Copy, Debug, Default)]
pub struct GroundContact {
    pub point: Vec3,
    pub normal: Vec3,
    pub depth: f32,
    pub friction: f32,
    pub surface: u32,
    pub velocity: Vec3,
    /// Original local contact vertex, before penetration correction moves the body.
    pub local_arm: Vec3,
    /// Incoming normal speed of the successful reaction, excluding sliding.
    pub closing_speed: f32,
    /// Body linear velocity immediately before the successful reaction.
    pub before_velocity: Vec3,
    pub impulse_delta_v: f32,
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
        let lift = (planar * dt + downward.max(0.0) * dt + extent).min(ceiling - point.y).max(0.25);
        if !lift.is_finite() || lift <= 0.0 {
            return None;
        }
        let hit = ground.hit(point + Vec3::Y * lift, Vec3::NEG_Y, lift)?;
        let normal = hit.normal * hit.normal.y.signum();
        let depth = (lift - hit.distance) * normal.y;
        if !normal.is_finite() || !depth.is_finite() || depth <= 0.000001 {
            return None;
        }
        Some(GroundContact {
            point,
            normal,
            depth,
            friction: ground.body_friction(hit.surface.tag),
            surface: hit.surface.tag,
            velocity: self.point_velocity(point),
            local_arm: self.rotation().transpose() * (point - self.position),
            closing_speed: 0.0,
            before_velocity: Vec3::ZERO,
            impulse_delta_v: 0.0,
        })
    }

    /// The first successful contact changes velocities; the deepest retained penetration moves the body.
    pub fn resolve_ground(&mut self, contacts: &mut [GroundContact]) {
        contacts.sort_by(|a, b| b.depth.total_cmp(&a.depth));
        let spec = *self.spec();
        for c in contacts.iter_mut() {
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
            let before_velocity = self.linear_velocity;
            if let Some(reaction) = self.react_plane(&plane, &params) {
                c.closing_speed = reaction.closing_speed.abs();
                c.before_velocity = before_velocity;
                c.impulse_delta_v = reaction.impulse / self.mass();
                break;
            }
        }
        if let Some(c) = contacts.first() {
            self.position += c.normal * c.depth;
        }
    }
}

//! Two bodies against each other: separating them and the impulse that changes their velocities.
//! Spec: `docs/specs/vehicle-rigid-body.md` (§5, §6). The original's solver is not specified; this is the
//! textbook rule with Coulomb friction, like [`RigidBody::react_plane`].

use glam::Vec3;

use super::body::{BodyState, RigidBody};
use super::contact::{FrictionState, Reaction};

/// What differs per body in a hit: how bouncy it is here and how hard it is to turn.
#[derive(Clone, Copy, Debug)]
pub struct BodyHit {
    pub restitution: f32,
    pub inertia_scale: Vec3,
}

/// A contact between two bodies.
#[derive(Clone, Copy, Debug)]
pub struct BodyContact {
    pub point: Vec3,
    /// Unit normal from the second body to the first.
    pub normal: Vec3,
    pub overlap: f32,
    pub friction_static: f32,
    pub friction_kinetic: f32,
}

/// Pushes two overlapping bodies apart along the normal, the lighter one further (by the mass ratio).
pub fn separate(a: &mut RigidBody, b: &mut RigidBody, normal: Vec3, overlap: f32) {
    let total = a.mass + b.mass;
    let move_b = a.mass / total;
    a.position += normal * (overlap * (1.0 - move_b));
    b.position -= normal * (overlap * move_b);
}

/// Resolves the contact: a normal impulse with the mean restitution of the two bodies, then friction.
/// `a` is the first body of the contact (the normal points at it). Returns `None` when the bodies are not
/// approaching.
pub fn react_bodies(
    a: &mut RigidBody,
    b: &mut RigidBody,
    contact: &BodyContact,
    hit_a: &BodyHit,
    hit_b: &BodyHit,
) -> Option<Reaction> {
    if a.state == BodyState::Frozen || b.state == BodyState::Frozen {
        return None;
    }
    let n = contact.normal;
    let (tensor_a, tensor_b) =
        (a.scaled_inv_world_tensor(hit_a.inertia_scale), b.scaled_inv_world_tensor(hit_b.inertia_scale));
    let (ra, rb) = (contact.point - a.world_cog(), contact.point - b.world_cog());
    let relative = |a: &RigidBody, b: &RigidBody| {
        (a.linear_velocity + a.angular_velocity.cross(ra)) - (b.linear_velocity + b.angular_velocity.cross(rb))
    };
    let vn = relative(a, b).dot(n);
    if vn >= 0.0 {
        return None;
    }
    a.wake();
    b.wake();
    let inverse_mass = |dir: Vec3| {
        a.inv_mass
            + b.inv_mass
            + dir.dot((tensor_a * ra.cross(dir)).cross(ra))
            + dir.dot((tensor_b * rb.cross(dir)).cross(rb))
    };
    let e = (0.5 * (hit_a.restitution + hit_b.restitution)).clamp(0.0, 1.0);
    let jn = -(1.0 + e) * vn / inverse_mass(n);
    a.linear_velocity += n * (jn * a.inv_mass);
    a.angular_velocity += tensor_a * ra.cross(n * jn);
    b.linear_velocity -= n * (jn * b.inv_mass);
    b.angular_velocity -= tensor_b * rb.cross(n * jn);

    let vp = relative(a, b);
    let vt = vp - n * vp.dot(n);
    let sliding = vt.length();
    let mut friction = FrictionState::None;
    if sliding > 1e-6 {
        let t = vt / sliding;
        let jt_stop = sliding / inverse_mass(t);
        let (jt, state) = match jt_stop <= contact.friction_static * jn {
            true => (jt_stop, FrictionState::Static),
            false => (contact.friction_kinetic * jn, FrictionState::Dynamic),
        };
        a.linear_velocity -= t * (jt * a.inv_mass);
        a.angular_velocity -= tensor_a * ra.cross(t * jt);
        b.linear_velocity += t * (jt * b.inv_mass);
        b.angular_velocity += tensor_b * rb.cross(t * jt);
        friction = state;
    }
    Some(Reaction { impulse: jn, closing_speed: -vn, sliding_speed: sliding, friction })
}

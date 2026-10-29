//! The ground against the segments of the ragdoll. Each segment is a capsule; its two end spheres are tested against the ground and
//! pushed out of it (position level), and once the sub-step's velocities are known Coulomb friction acts on the contact points
//! (velocity level): the impulse that would stop the slide, no larger than `mu` times the normal impulse the push amounted to. The
//! two-step shape is that of extended position-based dynamics (Mueller et al., "Detailed Rigid Body Simulation with Extended
//! Position Based Dynamics", 2020, sections 3.5 and 3.6). Restitution is zero.

use glam::Vec3;

use super::body::Body;
use crate::ground::Ground;

/// How far above a sphere's centre the ground is looked for (m): the ray starts there and reaches to just under the sphere.
const LOOK_ABOVE: f32 = 0.4;

/// A sphere that touched the ground in this sub-step.
#[derive(Clone, Copy, Debug)]
pub(super) struct Contact {
    pub body: usize,
    /// The contact point in the body's frame.
    pub local: Vec3,
    pub normal: Vec3,
    /// The normal impulse (position level: mass x distance) the push amounted to.
    pub impulse: f32,
    pub mu: f32,
}

/// Push every end sphere that is under the ground out of it and list the contacts. `friction` is the coefficient of the
/// segments (the ground's own multiplier is applied to it).
pub(super) fn resolve(bodies: &mut [Body], ground: &dyn Ground, friction: f32, out: &mut Vec<Contact>) {
    out.clear();
    for (index, body) in bodies.iter_mut().enumerate() {
        for end in [-1.0_f32, 1.0] {
            let local = Vec3::Y * (end * body.half_length);
            let centre = body.point(local);
            let Some((normal, separation, tag)) = probe(ground, centre, body.radius) else { continue };
            let depth = body.radius - separation;
            if depth <= 0.0 {
                continue;
            }
            let arm = centre - body.x - normal * body.radius;
            let share = body.inverse_mass_at(arm, normal);
            body.push_at(arm, normal * (depth / share));
            out.push(Contact {
                body: index,
                local: body.q.inverse() * (centre - body.x - normal * body.radius),
                normal,
                impulse: depth / share,
                mu: friction * ground.body_friction(tag),
            });
        }
    }
}

/// Where the ground is below the sphere at `centre`: the unit normal (up), the signed distance of the centre above the surface
/// along it, and the surface's tag. `None` when the ground is out of reach.
fn probe(ground: &dyn Ground, centre: Vec3, radius: f32) -> Option<(Vec3, f32, u32)> {
    let reach = LOOK_ABOVE + radius + 0.05;
    let hit = ground.hit(centre + Vec3::Y * LOOK_ABOVE, Vec3::NEG_Y, reach)?;
    let normal = hit.normal * hit.normal.y.signum();
    if !normal.is_finite() || normal.y <= 0.0 {
        return None;
    }
    let point = centre + Vec3::Y * LOOK_ABOVE - Vec3::Y * hit.distance;
    Some((normal, (centre - point).dot(normal), hit.surface.tag))
}

/// Friction on the contacts of a sub-step of length `h`, on the velocities the sub-step ended with: the impulse that would stop
/// the contact point's slide, held to `mu` times the normal impulse (`impulse / h`).
pub(super) fn friction(bodies: &mut [Body], contacts: &[Contact], h: f32) {
    for contact in contacts {
        let body = &mut bodies[contact.body];
        let arm = body.q * contact.local;
        let velocity = body.v + body.w.cross(arm);
        let sliding = velocity - contact.normal * velocity.dot(contact.normal);
        let Some(direction) = sliding.try_normalize() else { continue };
        let needed = sliding.length() / body.inverse_mass_at(arm, direction);
        let impulse = needed.min(contact.mu * contact.impulse / h);
        let spin = body.inv_world() * arm.cross(direction) * impulse;
        body.v -= direction * (impulse * body.inv_mass);
        body.w -= spin;
    }
}

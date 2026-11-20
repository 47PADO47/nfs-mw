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
/// ... and when that finds nothing for a sphere that is down at the level of the ground it last touched (it has sunk further than the
/// ray starts, into a kerb, a ramp or a step in the road that its last move carried it under), the ray starts this much higher.
const DEEP_LOOK: f32 = 2.5;
/// How much higher than the last ground a body touched a surface may be for the deep ray to take it for the ground (m): a kerb or a
/// ramp, not the deck of a bridge above a body that is lying under it.
const DEEP_STEP: f32 = 1.2;
/// The deepest the deep ray is used below the level of the last ground (m); a sphere further up than this is in the air.
const DEEP_LEVEL: f32 = 0.5;
/// How much of an overlap one sub-step recovers beyond what the body's own motion into the surface needs (m): a body that starts
/// 40 cm into the road comes out of it in a few steps, without a jolt.
pub(super) const MAX_RECOVER: f32 = 0.02;

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

/// What a sphere found under it.
struct Found {
    normal: Vec3,
    /// The signed distance of the centre above the surface along the normal (m).
    separation: f32,
    tag: u32,
    /// The height of the surface where the ray met it (m).
    height: f32,
}

/// Push every end sphere that is under the ground out of it and list the contacts. `friction` is the coefficient of the
/// segments (the ground's own multiplier is applied to it). `h` is the sub-step, `floor` the height of the highest ground the body
/// has touched lately (`None` before the first touch).
///
/// The push is what the body's own motion into the ground needs (so that it stops at it) and, beyond that, at most [`MAX_RECOVER`]
/// of the overlap it was already in: that part is recovery, not motion, and is kept out of the velocity ([`Body::recover`]).
pub(super) fn resolve(
    bodies: &mut [Body],
    ground: &dyn Ground,
    (friction, h): (f32, f32),
    floor: &mut Option<f32>,
    out: &mut Vec<Contact>,
) {
    out.clear();
    let mut touched: Option<f32> = None;
    for (index, body) in bodies.iter_mut().enumerate() {
        for end in [-1.0_f32, 1.0] {
            let local = Vec3::Y * (end * body.half_length);
            let centre = body.point(local);
            let Some(found) = probe(ground, centre, body.radius, *floor) else { continue };
            touched = Some(touched.map_or(found.height, |t| t.max(found.height)));
            let depth = body.radius - found.separation;
            if depth <= 0.0 {
                continue;
            }
            let normal = found.normal;
            let arm = centre - body.x - normal * body.radius;
            let share = body.inverse_mass_at(arm, normal);
            // What the motion of this sub-step carried into the surface; the rest of the overlap is recovered gently.
            let closing = -(body.v + body.w.cross(arm)).dot(normal);
            let own = closing.max(0.0) * h;
            let push = depth.min(own + MAX_RECOVER);
            let recovered = (push - own).max(0.0);
            body.push_at(arm, normal * (push / share));
            body.recover += normal * (recovered * body.inv_mass / share);
            out.push(Contact {
                body: index,
                local: body.q.inverse() * (centre - body.x - normal * body.radius),
                normal,
                impulse: push / share,
                mu: friction * ground.body_friction(found.tag),
            });
        }
    }
    *floor = touched.or(*floor);
}

/// Where the ground is below the sphere at `centre`: the unit normal (up), the signed distance of the centre above the surface
/// along it, and the surface's tag. `None` when the ground is out of reach.
fn probe(ground: &dyn Ground, centre: Vec3, radius: f32, floor: Option<f32>) -> Option<Found> {
    let reach = LOOK_ABOVE + radius + 0.05;
    let near = ground.hit(centre + Vec3::Y * LOOK_ABOVE, Vec3::NEG_Y, reach).map(|hit| (hit, LOOK_ABOVE));
    let hit = near.or_else(|| deep(ground, centre, radius, floor))?;
    let (hit, above) = hit;
    let normal = hit.normal * hit.normal.y.signum();
    if !normal.is_finite() || normal.y <= 0.0 {
        return None;
    }
    let point = centre + Vec3::Y * above - Vec3::Y * hit.distance;
    Some(Found { normal, separation: (centre - point).dot(normal), tag: hit.surface.tag, height: point.y })
}

/// The ray of a sphere that is down at the level of the ground its body last touched and found nothing: it starts [`DEEP_LOOK`]
/// above the sphere, and what it meets is taken for the ground only if it is not [`DEEP_STEP`] above that level.
fn deep(ground: &dyn Ground, centre: Vec3, radius: f32, floor: Option<f32>) -> Option<(crate::ground::GroundHit, f32)> {
    let floor = floor.filter(|floor| centre.y < floor + DEEP_LEVEL)?;
    let hit = ground.hit(centre + Vec3::Y * DEEP_LOOK, Vec3::NEG_Y, DEEP_LOOK + radius + 0.05)?;
    let height = centre.y + DEEP_LOOK - hit.distance;
    (height <= floor + DEEP_STEP).then_some((hit, DEEP_LOOK))
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

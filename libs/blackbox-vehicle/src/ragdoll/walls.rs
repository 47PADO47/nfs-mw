//! The walls of the world against the segments of the ragdoll.
//!
//! The road is not all a body meets: the edge of a road is a wall of the collision packs, 3 m or more tall (`docs/specs/ride6-feel-round10.md`),
//! and a rider thrown at one at bike speed must stop at it, not fly through it and fall into the void on its far side. Once per step, before
//! the sub-steps, every end sphere that is moving casts a segment along its motion (a step's worth, and its radius) against the walls
//! ([`Ground::wall`]); a sphere that would reach the wall within the step is stopped at it by an impulse along the wall's normal (no
//! bounce: the rider's material has no restitution) with Coulomb friction along it, and one that is already inside is moved out of it. This is
//! a velocity-level stop with a position fix, not a substitute for the sub-steps' own contacts: a body that is pressed on a wall keeps being
//! stopped step after step.

use glam::Vec3;

use super::body::Body;
use crate::ground::Ground;

/// A sphere slower than this (m/s) is not tested: it cannot cross a wall in a step, and one resting on a wall is already against it.
const MIN_SPEED: f32 = 0.5;
/// How far past the sphere's reach in a step the segment extends (m).
const SKIN: f32 = 0.05;
/// The most an overlap with a wall is undone in one step (m).
const MAX_FIX: f32 = 0.3;

/// Stop every end sphere that would cross a wall in the step of `dt` seconds. `friction` is the coefficient of the segments (the wall's own
/// multiplier, [`Ground::world_friction`], is applied to it).
pub(super) fn collide(bodies: &mut [Body], ground: &dyn Ground, dt: f32, friction: f32) {
    for body in bodies.iter_mut() {
        for end in [-1.0_f32, 1.0] {
            let local = Vec3::Y * (end * body.half_length);
            let centre = body.point(local);
            let arm = centre - body.x;
            let velocity = body.v + body.w.cross(arm);
            let speed = velocity.length();
            if speed < MIN_SPEED {
                continue;
            }
            let direction = velocity / speed;
            let reach = speed * dt + body.radius + SKIN;
            let Some(wall) = ground.wall(centre, centre + direction * reach) else { continue };
            let normal = wall.normal;
            if !normal.is_finite() || normal.dot(direction) >= -1e-3 {
                continue;
            }
            let gap = (centre - wall.point).dot(normal) - body.radius;
            if gap < 0.0 {
                body.x += normal * (-gap).min(MAX_FIX);
            }
            let approach = -velocity.dot(normal);
            let allowed = gap.max(0.0) / dt;
            if approach <= allowed {
                continue;
            }
            let share = body.inverse_mass_at(arm, normal);
            let impulse = (approach - allowed) / share;
            body.v += normal * (impulse * body.inv_mass);
            body.w += body.inv_world() * arm.cross(normal * impulse);
            slide(body, arm, normal, impulse * friction * ground.world_friction(wall.surface));
        }
    }
}

/// Coulomb friction along the wall for the sphere at `arm`: the impulse that would stop its slide, held to `limit`.
fn slide(body: &mut Body, arm: Vec3, normal: Vec3, limit: f32) {
    let velocity = body.v + body.w.cross(arm);
    let along = velocity - normal * velocity.dot(normal);
    let Some(direction) = along.try_normalize() else { return };
    let impulse = (along.length() / body.inverse_mass_at(arm, direction)).min(limit);
    body.v -= direction * (impulse * body.inv_mass);
    body.w -= body.inv_world() * arm.cross(direction * impulse);
}

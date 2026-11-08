//! The walls of the world against the segments of the ragdoll.
//!
//! The road is not all a body meets: the edge of a road is a wall of the collision packs, 3 m or more tall (`docs/specs/ride6-feel-round10.md`),
//! and a rider thrown at one at bike speed must stop at it, not fly through it and fall into the void on its far side. Once per step, before
//! the sub-steps, every end sphere that is moving casts a segment along its motion (a step's worth, and its radius) against the walls
//! ([`Ground::wall`]); a sphere that would reach the wall within the step is stopped at it by an impulse along the wall's normal (no
//! bounce: the rider's material has no restitution) with Coulomb friction along it, and one that is already inside is moved out of it. This is
//! a velocity-level stop with a position fix. The wall a sphere met is remembered ([`Plane`]) until the sphere leaves it, and the sub-steps
//! keep the sphere out of it ([`hold`]): the joints of a chain that is still moving drag the stopped end into the wall, and a velocity
//! stop alone let a rider through at 30 m/s (a pelvis and a trunk behind a head that had stopped).

use glam::Vec3;

use super::body::Body;
use crate::ground::Ground;

/// A sphere slower than this (m/s) is not tested: it cannot cross a wall in a step, and one resting on a wall is already against it.
const MIN_SPEED: f32 = 0.5;
/// How far past the sphere's reach in a step the segment extends (m).
const SKIN: f32 = 0.05;
/// The most an overlap with a wall is undone in one step (m), and in one sub-step by [`hold`].
const MAX_FIX: f32 = 0.3;
const MAX_HOLD: f32 = 0.1;
/// A sphere this far from the wall it met has left it (m).
const LEFT: f32 = 1.0;

/// A wall a sphere met: a point of it and its normal (facing the sphere).
#[derive(Clone, Copy, Debug)]
pub(super) struct Plane {
    point: Vec3,
    normal: Vec3,
}

/// The walls of the two end spheres of each body.
pub(super) type Planes = Vec<[Option<Plane>; 2]>;

/// Stop every end sphere that would cross a wall in the step of `dt` seconds. `friction` is the coefficient of the segments (the wall's own
/// multiplier, [`Ground::world_friction`], is applied to it).
pub(super) fn collide(bodies: &mut [Body], planes: &mut Planes, ground: &dyn Ground, dt: f32, friction: f32) {
    planes.resize(bodies.len(), [None; 2]);
    for (body, planes) in bodies.iter_mut().zip(planes.iter_mut()) {
        for (slot, end) in planes.iter_mut().zip([-1.0_f32, 1.0]) {
            let local = Vec3::Y * (end * body.half_length);
            let centre = body.point(local);
            let arm = centre - body.x;
            let velocity = body.v + body.w.cross(arm);
            let speed = velocity.length();
            if speed < MIN_SPEED {
                // Still: the wall it leans on stays, unless it has been left behind.
                *slot = slot.filter(|plane| (centre - plane.point).dot(plane.normal) < body.radius + LEFT);
                continue;
            }
            let direction = velocity / speed;
            let reach = speed * dt + body.radius + SKIN;
            let Some(wall) = ground.wall(centre, centre + direction * reach) else {
                *slot = slot.filter(|plane| (centre - plane.point).dot(plane.normal) < body.radius + LEFT);
                continue;
            };
            let normal = wall.normal;
            if !normal.is_finite() || normal.dot(direction) >= -1e-3 {
                continue;
            }
            *slot = Some(Plane { point: wall.point, normal });
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

/// Keep the end spheres out of the walls they met, in a sub-step: a sphere that is past its wall (the joints dragged it there) is moved
/// back out of it, by at most [`MAX_HOLD`], as a recovery that gives it no velocity.
pub(super) fn hold(bodies: &mut [Body], planes: &Planes) {
    for (body, planes) in bodies.iter_mut().zip(planes) {
        for (plane, end) in planes.iter().zip([-1.0_f32, 1.0]) {
            let Some(plane) = plane else { continue };
            let centre = body.point(Vec3::Y * (end * body.half_length));
            let gap = (centre - plane.point).dot(plane.normal) - body.radius;
            if gap >= 0.0 {
                continue;
            }
            let push = (-gap).min(MAX_HOLD);
            let arm = centre - body.x - plane.normal * body.radius;
            let share = body.inverse_mass_at(arm, plane.normal);
            body.push_at(arm, plane.normal * (push / share));
            body.recover += plane.normal * (push * body.inv_mass / share);
        }
    }
}

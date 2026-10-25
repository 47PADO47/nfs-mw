//! What a traffic car decides every think: how far ahead to keep its cursor and how fast to go.
//! Spec: `docs/specs/ai-traffic.md` (§3).

use blackbox_driver::ramp;
use blackbox_roads::{
    Body, Occlusion, RandomSource, RoadNav, RoadNetwork, trail_curvature, update_occluded_position, zone,
};
use glam::Vec3;

use super::stop::StopLine;

/// Ten ticks on the shared physics clock (6 Hz at 60 Hz). The original variable task's rate 0.1
/// counts rendered frames, not seconds; this adapter keeps decisions independent of render rate.
pub const THINK_STEPS: u32 = 10;
/// Seconds between thinks.
pub const THINK_PERIOD: f32 = THINK_STEPS as f32 * crate::scenes::world::drive::STEP;

/// Posted speeds in metres per second: 35 mph on streets, 55 mph where there are four or more traffic
/// lanes.
pub const STREET_SPEED: f32 = 35.0 * MPH;
pub const HIGHWAY_SPEED: f32 = 55.0 * MPH;
/// Metres per second in one mile per hour.
pub const MPH: f32 = 0.447_04;
/// Traffic lane zones (both directions) that make a road a highway.
const HIGHWAY_LANES: usize = 4;
/// Lateral acceleration (in g) traffic and cops take bends at.
const TRAFFIC_LATERAL_G: f32 = 0.6;
const COP_LATERAL_G: f32 = 1.6;
const GRAVITY: f32 = 9.8;
/// The speed a car on a free road may always ask for, so that a stopped car can pull away again. The
/// original has no such floor and a car that has come to a halt stays on the brake.
const CREEP_SPEED: f32 = 1.0;
/// A traffic car never asks for more than this much above its current speed per second.
const ACCELERATION_CAP: f32 = 2.0;

/// The distance ahead of the car the cursor is kept at: 10 m when slow, 30 m when fast, plus a think
/// of travel and the car's radius.
pub fn look_ahead(speed: f32, think: f32, radius: f32) -> f32 {
    let base = 10.0 + 20.0 * ramp(speed, 0.0, 25.0);
    base + speed * think + radius
}

/// How a car cruises: its speed on streets and on roads with four or more traffic lanes, and how hard it
/// takes bends. Traffic gets the numbers of the pattern it spawned in, patrol cops the search-mode speeds.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Cruise {
    pub street: f32,
    pub highway: f32,
    pub lateral_g: f32,
    /// A cop may accelerate as hard as it likes; traffic never asks for much more than it has.
    pub limit_acceleration: bool,
}

impl Cruise {
    pub fn traffic(street: f32, highway: f32) -> Self {
        Self { street, highway, lateral_g: TRAFFIC_LATERAL_G, limit_acceleration: true }
    }

    pub fn patrol(street: f32, highway: f32) -> Self {
        Self { street, highway, lateral_g: COP_LATERAL_G, limit_acceleration: false }
    }
}

impl Default for Cruise {
    fn default() -> Self {
        Self::traffic(STREET_SPEED, HIGHWAY_SPEED)
    }
}

/// The speed limit posted on the segment the cursor is on.
pub fn posted_speed(net: &RoadNetwork, nav: &RoadNav, cruise: &Cruise) -> f32 {
    let seg = net.segment(nav.segment);
    let profile = net.profile_at(nav.segment, seg.nodes[0]);
    let lanes = profile.zones.iter().filter(|z| z.kind == zone::TRAFFIC).count();
    match lanes >= HIGHWAY_LANES {
        true => cruise.highway,
        false => cruise.street,
    }
}

/// Signed distance of the cursor ahead of the car along the cursor's forward vector.
pub fn cursor_ahead(nav: &RoadNav, car: Vec3) -> f32 {
    nav.forward.dot(nav.position - car)
}

/// What a traffic car knows of itself when it thinks.
#[derive(Debug, Clone, Copy)]
pub struct CarState {
    pub body: Body,
    /// Signed speed along the car, m/s.
    pub forward_speed: f32,
    /// Bounding radius, metres.
    pub radius: f32,
    /// Physical body mass; the following-distance calculation separately accounts for tractors.
    pub mass: f32,
    pub tractor: bool,
    /// The next stop line of a signalled junction ahead, when lights are on and the car obeys them.
    pub stop: Option<StopLine>,
}

/// The result of a think.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Think {
    /// The point to steer at (physics space).
    pub target: Vec3,
    pub speed: f32,
    /// Another car blocks the view along the trail.
    pub blocked: bool,
}

/// The stopping distance behind a car ahead: 3 m when slow up to 50 m at the top of the table.
pub(super) fn stopping_distance(speed: f32, mass: f32) -> f32 {
    3.0 + 47.0 * (speed * (mass * 0.0005).max(1.0) / 80.0).clamp(0.0, 1.0)
}

/// The effective mass used only when slowing for a vehicle ahead, not by the rigid body.
fn following_mass(car: &CarState) -> f32 {
    match car.tractor {
        true => car.mass * 2.0,
        false => car.mass,
    }
}

/// The other cars as a think reads them: every body of the step's snapshot (`simulation.rs`) except the thinking car
/// `me`, in snapshot order, then the player. Nothing is copied; the snapshot is read in place.
pub fn others<'a>(
    bodies: &'a [(Body, bool)],
    me: usize,
    player: Option<Body>,
) -> impl Iterator<Item = Body> + Clone + 'a {
    bodies.iter().enumerate().filter(move |&(i, _)| i != me).map(|(_, b)| b.0).chain(player)
}

/// The traffic think: keep the cursor ahead of the car, find the steering point past other cars and pick
/// a speed. `was_blocked` is the result of the previous think (a blocked car looks further ahead). `others` are
/// the other cars as bodies (see [`others`]).
#[cfg(test)]
pub fn think(
    net: &RoadNetwork,
    nav: &mut RoadNav,
    car: &CarState,
    others: impl IntoIterator<Item = Body>,
    was_blocked: bool,
    rng: &mut impl RandomSource,
    cruise: &Cruise,
) -> Think {
    think_with_delta(THINK_PERIOD, net, nav, car, others, was_blocked, rng, cruise)
}

#[allow(clippy::too_many_arguments)]
pub fn think_with_delta(
    dt: f32,
    net: &RoadNetwork,
    nav: &mut RoadNav,
    car: &CarState,
    others: impl IntoIterator<Item = Body>,
    was_blocked: bool,
    rng: &mut impl RandomSource,
    cruise: &Cruise,
) -> Think {
    let speed = car.forward_speed.abs();
    let look = match was_blocked {
        true => 30.0 + speed * dt + car.radius,
        false => look_ahead(speed, dt, car.radius),
    };
    let ahead = cursor_ahead(nav, car.body.position);
    if !nav.dead_end && ahead < look {
        nav.advance_with_lookahead(net, look - ahead, Vec3::ZERO, look, rng);
    }
    // The other cars within the avoidable radius.
    let reach = look.max(2.0 * speed);
    let position = car.body.position;
    let near = others.into_iter().filter(move |o| o.position.distance(position) < reach);
    let occlusion = nav
        .trail()
        .and_then(|trail| update_occluded_position(trail, &car.body, nav.position, near, true, nav.half_width));
    let target = occlusion.as_ref().map_or(nav.position, |o| o.position);
    let blocked = occlusion.as_ref().is_some_and(|o| o.avoidable != 0);
    Think { target, speed: compute_speed(net, nav, car, occlusion.as_ref(), cruise, dt), blocked }
}

/// The wanted speed: the posted one, limited by the bends ahead, by the car in front and by a light that
/// is red (or amber and still stoppable), nothing at a dead end, and never far above the current speed.
fn compute_speed(
    net: &RoadNetwork,
    nav: &RoadNav,
    car: &CarState,
    occlusion: Option<&Occlusion>,
    cruise: &Cruise,
    dt: f32,
) -> f32 {
    if nav.dead_end {
        return 0.0;
    }
    let posted = posted_speed(net, nav, cruise);
    let current = car.forward_speed;
    let mut desired = posted;
    if let (Some(o), Some(trail)) = (occlusion, nav.trail()) {
        let k = trail_curvature(trail.cookies(), o.current_index, car.body.position, o, nav.position, current);
        let a = cruise.lateral_g * GRAVITY;
        desired = desired.min((a / (a / (posted * posted)).max(k.abs())).sqrt());
        if o.avoidable != 0 && !o.from_behind {
            let length = 2.0 * car.radius;
            let distance = (o.apex.distance(car.body.position) - length).max(0.0);
            let stop = stopping_distance(current, following_mass(car));
            if distance < stop {
                desired = (o.trail_speed * distance / stop).clamp(0.0, desired).min(current);
            }
        }
    }
    // A light that holds the car is an obstacle that stands at the stop line.
    let light = car.stop.and_then(|stop| stop.speed_limit(current.max(0.0), car.mass));
    desired = desired.min(light.unwrap_or(desired));
    if !cruise.limit_acceleration {
        return desired;
    }
    // The cap on how far the target may exceed the current speed would pin a stopped car to the brake (the
    // pedals treat anything under 0.5 m/s as "stop"), so a free road always allows a walking pace. Not
    // beyond what the light allows: at the line the car has to stay on the brake.
    let floor = CREEP_SPEED.min(light.unwrap_or(CREEP_SPEED));
    desired.min((current.max(0.0) + ACCELERATION_CAP * dt).max(floor))
}

#[cfg(test)]
mod tests;

//! What a traffic car decides every think: how far ahead to keep its cursor and how fast to go.
//! Spec: `docs/specs/ai-traffic.md` (§3).

use blackbox_driver::ramp;
use blackbox_roads::{RoadNav, RoadNetwork, zone};
use glam::Vec3;

/// The traffic AI thinks at 10 Hz: every this many physics steps.
pub const THINK_STEPS: u32 = 10;
/// Seconds between thinks.
pub const THINK_PERIOD: f32 = THINK_STEPS as f32 / 60.0;

/// Posted speeds in metres per second: 35 mph on streets, 55 mph where there are four or more traffic
/// lanes.
pub const STREET_SPEED: f32 = 35.0 * 0.447_04;
pub const HIGHWAY_SPEED: f32 = 55.0 * 0.447_04;
/// Traffic lane zones (both directions) that make a road a highway.
const HIGHWAY_LANES: usize = 4;
/// A traffic car never asks for more than this much above its current speed per second.
const ACCELERATION_CAP: f32 = 2.0;

/// The distance ahead of the car the cursor is kept at: 10 m when slow, 30 m when fast, plus a think
/// of travel and the car's radius.
pub fn look_ahead(speed: f32, think: f32, radius: f32) -> f32 {
    let base = 10.0 + 20.0 * ramp(speed, 0.0, 25.0);
    base + speed * think + radius
}

/// The speed limit posted on the segment the cursor is on.
pub fn posted_speed(net: &RoadNetwork, nav: &RoadNav) -> f32 {
    let seg = net.segment(nav.segment);
    let profile = net.profile_at(nav.segment, seg.nodes[0]);
    let lanes = profile.zones.iter().filter(|z| z.kind == zone::TRAFFIC).count();
    match lanes >= HIGHWAY_LANES {
        true => HIGHWAY_SPEED,
        false => STREET_SPEED,
    }
}

/// The wanted speed: the posted one, nothing at a dead end, and never far above the current speed.
pub fn wanted_speed(net: &RoadNetwork, nav: &RoadNav, current: f32, think: f32) -> f32 {
    if nav.dead_end {
        return 0.0;
    }
    posted_speed(net, nav).min(current.max(0.0) + ACCELERATION_CAP * think)
}

/// Signed distance of the cursor ahead of the car along the cursor's forward vector.
pub fn cursor_ahead(nav: &RoadNav, car: Vec3) -> f32 {
    nav.forward.dot(nav.position - car)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_look_ahead_grows_with_speed() {
        let radius = 2.5;
        assert!((look_ahead(0.0, THINK_PERIOD, radius) - 12.5).abs() < 1e-4);
        // Fast: 30 m, plus a think of travel, plus the radius.
        assert!((look_ahead(25.0, THINK_PERIOD, radius) - (30.0 + 25.0 * THINK_PERIOD + radius)).abs() < 1e-3);
        assert!(look_ahead(12.5, THINK_PERIOD, radius) > look_ahead(5.0, THINK_PERIOD, radius));
    }

    #[test]
    fn a_standing_car_may_only_ask_for_a_little_more() {
        use blackbox_roads::{Road, RoadNetwork, RoadNode, RoadProfile, RoadSegment, Zone};
        let lane = |offset| Zone { kind: zone::TRAFFIC, width: 4.0, offset };
        let net = RoadNetwork {
            nodes: vec![
                RoadNode { position: Vec3::ZERO, profile: 0, segments: vec![0] },
                RoadNode { position: Vec3::new(0.0, 0.0, 100.0), profile: 0, segments: vec![0] },
            ],
            segments: vec![RoadSegment {
                nodes: [0, 1],
                length: 100.0,
                road: Some(0),
                flags: 0,
                start_handle: Vec3::ZERO,
                end_handle: Vec3::ZERO,
            }],
            profiles: vec![RoadProfile { middle: 1, zones: vec![lane(2.0), lane(2.0)] }],
            roads: vec![Road { scale: 1.0, speech_id: 0 }],
        };
        let mut nav = RoadNav::traffic();
        nav.init_in_lane(&net, 0, 1, 0.5);
        assert_eq!(posted_speed(&net, &nav), STREET_SPEED);
        // The desired speed is the posted one, but never more than 2 m/s above the current speed per second.
        assert!((wanted_speed(&net, &nav, 10.0, THINK_PERIOD) - (10.0 + 2.0 * THINK_PERIOD)).abs() < 1e-5);
        assert_eq!(wanted_speed(&net, &nav, STREET_SPEED, THINK_PERIOD), STREET_SPEED);
        nav.dead_end = true;
        assert_eq!(wanted_speed(&net, &nav, 10.0, THINK_PERIOD), 0.0);
    }
}

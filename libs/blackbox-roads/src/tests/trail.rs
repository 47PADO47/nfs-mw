use glam::{Vec2, Vec3};

use super::synth::*;
use crate::{Body, CUT, RoadNav, SplitMix, update_occluded_position};

fn body(position: Vec3, forward: Vec2, speed: f32) -> Body {
    Body {
        position,
        velocity: Vec3::new(forward.x, 0.0, forward.y) * speed,
        forward,
        half_width: 0.9,
        half_length: 2.2,
    }
}

/// A traffic cursor in the first forward lane of the 100 m road, 30 m ahead of a car at the start.
fn cursor_ahead() -> (crate::RoadNetwork, RoadNav) {
    let net = straight_road();
    let mut nav = RoadNav::traffic();
    nav.init_in_lane(&net, 0, 3, 0.0);
    nav.enable_trail(&net);
    nav.advance_with_lookahead(&net, 30.0, Vec3::ZERO, 0.0, &mut SplitMix(1));
    (net, nav)
}

#[test]
fn the_trail_records_a_cookie_every_few_metres() {
    let (_, nav) = cursor_ahead();
    let trail = nav.trail().unwrap();
    assert!((9..=12).contains(&trail.len()), "{} cookies", trail.len());
    let gaps: Vec<f32> = trail.cookies().iter().skip(1).map(|c| c.length).collect();
    assert!(gaps.iter().all(|&g| (3.0..=3.4).contains(&g)), "{gaps:?}");
    // Along +z the corridor is as wide as the car's half width, centred on the lane.
    let c = trail.cookies()[3];
    assert!((c.right_offset - 0.5).abs() < 1e-3 && (c.left_offset + 0.5).abs() < 1e-3, "{c:?}");
    assert!(c.forward.distance(Vec2::Y) < 1e-3);
}

#[test]
fn the_trail_is_a_ring() {
    let net = chain_road();
    let mut nav = RoadNav::traffic();
    nav.init_in_lane(&net, 0, 3, 0.0);
    nav.enable_trail(&net);
    nav.advance_with_lookahead(&net, 190.0, Vec3::ZERO, 0.0, &mut SplitMix(1));
    assert_eq!(nav.trail().unwrap().len(), crate::TRAIL_CAPACITY);
}

#[test]
fn nothing_in_the_way_means_steering_at_the_cursor() {
    let (_, nav) = cursor_ahead();
    let car = body(Vec3::new(2.0, 0.0, 0.0), Vec2::Y, 10.0);
    let o = update_occluded_position(nav.trail().unwrap(), &car, nav.position, &[], true, 0.9).unwrap();
    assert!(!o.occluded());
    assert!(o.position.distance(nav.position) < 1e-4);
    assert!((o.out_of_bounds - (0.9 - 0.5)).abs() < 0.2, "centred in the lane: {}", o.out_of_bounds);
}

#[test]
fn a_car_off_the_corridor_reports_it() {
    let (_, nav) = cursor_ahead();
    let car = body(Vec3::new(5.0, 0.0, 0.0), Vec2::Y, 10.0);
    let o = update_occluded_position(nav.trail().unwrap(), &car, nav.position, &[], true, 0.9).unwrap();
    assert!(o.out_of_bounds > 3.0, "{}", o.out_of_bounds);
}

#[test]
fn a_car_ahead_cuts_the_corridor_and_names_its_speed() {
    let (_, nav) = cursor_ahead();
    let me = body(Vec3::new(2.0, 0.0, 0.0), Vec2::Y, 10.0);
    // Another car stopped in the lane 15 m ahead.
    let other = body(Vec3::new(2.0, 0.0, 15.0), Vec2::Y, 0.0);
    let o = update_occluded_position(nav.trail().unwrap(), &me, nav.position, &[other], true, 0.9).unwrap();
    assert_ne!(o.avoidable, 0, "the stopped car blocks the view: {o:?}");
    assert!(!o.from_behind);
    assert_eq!(o.trail_speed, 0.0);
    assert!(o.closing_speed > 9.0, "closing at {}", o.closing_speed);
    assert!(o.apex.z > 5.0 && o.apex.z < 25.0, "{:?}", o.apex);
    // The trail itself is untouched: the cut works on a copy.
    assert!(nav.trail().unwrap().cookies().iter().all(|c| c.flags & CUT == 0));
}

#[test]
fn a_car_far_to_the_side_is_ignored() {
    let (_, nav) = cursor_ahead();
    let me = body(Vec3::new(2.0, 0.0, 0.0), Vec2::Y, 10.0);
    let other = body(Vec3::new(40.0, 0.0, 15.0), Vec2::Y, 0.0);
    let o = update_occluded_position(nav.trail().unwrap(), &me, nav.position, &[other], true, 0.9).unwrap();
    assert!(!o.occluded());
}

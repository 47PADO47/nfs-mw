use nfsmw_sim::{EntityKind, EntityView};

use super::*;
use crate::geom;

const RADIUS: f32 = 30.0;
const SPEED: f32 = 20.0;

fn view(id: u16, kind: EntityKind, position: [f32; 3], velocity: [f32; 3]) -> EntityView {
    EntityView {
        id,
        kind,
        owner: None,
        position,
        orientation: geom::IDENTITY,
        linear_velocity: velocity,
        angular_velocity: [0.0; 3],
        type_id: 0,
        flags: 0,
    }
}

/// A car on a circle of 30 m at 20 m/s, at fractional tick `t`.
fn on_circle(t: f64) -> EntityView {
    let omega = SPEED / RADIUS;
    let theta = omega * (t / 60.0) as f32;
    let (s, c) = theta.sin_cos();
    let mut v = view(1, EntityKind::PlayerCar, [RADIUS * c, 0.0, RADIUS * s], [-SPEED * s, 0.0, SPEED * c]);
    // Facing along the velocity: +z turned about +y by the heading.
    v.orientation = geom::from_rotation_vector([0.0, (-s).atan2(c), 0.0]);
    v.angular_velocity = [0.0, omega, 0.0];
    v
}

/// sync-model.md section 10, test 2: a car on a circle sampled at 20 Hz is drawn within 5 cm of the true circle.
#[test]
fn a_circle_sampled_at_20_hz_is_drawn_within_5_cm() {
    let mut interp = Interpolator::default();
    let mut pushed = 0u32;
    let mut worst = 0.0f32;
    for step in 0..5000 {
        let r = 1.0 + f64::from(step) * 0.11;
        // Records arrive as the display time follows them, two snapshot intervals behind the newest.
        while f64::from(pushed) <= r + 6.0 {
            interp.push(pushed, &on_circle(f64::from(pushed)));
            pushed += 3;
        }
        let drawn = interp.sample(1, r).unwrap();
        assert_eq!(drawn.mode, DrawMode::Interpolated);
        worst = worst.max(geom::distance(drawn.view.position, on_circle(r).position));
    }
    assert!(worst < 0.05, "worst error {worst} m");
}

#[test]
fn orientation_is_slerped_along_the_shorter_arc() {
    let mut interp = Interpolator::default();
    let mut a = view(1, EntityKind::PlayerCar, [0.0; 3], [0.0; 3]);
    let mut b = a;
    a.orientation = geom::from_rotation_vector([0.0, 3.0, 0.0]);
    let q = geom::from_rotation_vector([0.0, -3.0, 0.0]);
    b.orientation = [-q[0], -q[1], -q[2], -q[3]];
    interp.push(0, &a);
    interp.push(6, &b);
    let drawn = interp.sample(1, 3.0).unwrap();
    // Halfway between +3 and -3 rad the short way round is π.
    let expected = geom::from_rotation_vector([0.0, std::f32::consts::PI, 0.0]);
    assert!(geom::angle_between(drawn.view.orientation, expected) < 1.0e-3);
}

#[test]
fn discrete_fields_switch_halfway() {
    let mut interp = Interpolator::default();
    let mut a = view(1, EntityKind::PoliceCar, [0.0; 3], [0.0; 3]);
    let mut b = a;
    a.flags = 1;
    b.flags = 8;
    interp.push(0, &a);
    interp.push(6, &b);
    assert_eq!(interp.sample(1, 2.9).unwrap().view.flags, 1);
    assert_eq!(interp.sample(1, 3.1).unwrap().view.flags, 8);
}

#[test]
fn extrapolation_is_bounded_and_then_lags() {
    let config = InterpConfig::default();
    let mut interp = Interpolator::new(config);
    let v = 30.0;
    interp.push(0, &view(1, EntityKind::TrafficCar, [0.0; 3], [v, 0.0, 0.0]));
    interp.push(3, &view(1, EntityKind::TrafficCar, [1.5, 0.0, 0.0], [v, 0.0, 0.0]));
    // Full velocity for 100 ms, then a linear decay to zero at 250 ms: at most v × 0.175 s past the newest record.
    let bound = 1.5 + v * 0.175;
    let mut last_x = 0.0f32;
    for step in 0..200 {
        let r = 3.0 + f64::from(step) * 0.5;
        let drawn = interp.sample(1, r).unwrap();
        let x = drawn.view.position[0];
        assert!(x <= bound + 1.0e-3, "x {x} at r {r}");
        assert!(x >= last_x - 1.0e-4, "never moves back");
        last_x = x;
        let late = (r - 3.0) * 1000.0 / 60.0;
        let expected_mode = match late {
            l if l <= 0.0 => DrawMode::Interpolated,
            l if l < 250.0 => DrawMode::Extrapolated,
            _ => DrawMode::Lagging,
        };
        assert_eq!(drawn.mode, expected_mode, "at {late} ms");
        if late > 1250.0 {
            assert_eq!(drawn.fade, 0.0, "lagging traffic fades out after a second");
        }
    }
    assert!((last_x - bound).abs() < 1.0e-3, "held at {last_x}");
}

#[test]
fn the_pose_blends_back_after_extrapolating() {
    let mut interp = Interpolator::default();
    interp.push(0, &view(1, EntityKind::PlayerCar, [0.0; 3], [20.0, 0.0, 0.0]));
    interp.push(3, &view(1, EntityKind::PlayerCar, [1.0, 0.0, 0.0], [20.0, 0.0, 0.0]));
    let mut last = interp.sample(1, 3.0).unwrap().view.position;
    let mut r = 3.0;
    let mut largest_step = 0.0f32;
    for frame in 0..60 {
        r += 1.0;
        if frame == 10 {
            // The late snapshot arrives, and it says the car had turned off its straight line.
            interp.push(9, &view(1, EntityKind::PlayerCar, [2.0, 0.0, 1.0], [10.0, 0.0, 10.0]));
            interp.push(18, &view(1, EntityKind::PlayerCar, [3.5, 0.0, 2.5], [10.0, 0.0, 10.0]));
        }
        let drawn = interp.sample(1, r).unwrap();
        largest_step = largest_step.max(geom::distance(drawn.view.position, last));
        last = drawn.view.position;
    }
    // The records put the car more than 2 m from where it was extrapolated; drawn, the move is spread over 100 ms.
    assert!(largest_step < 0.7, "largest step {largest_step}");
}

#[test]
fn entities_appear_and_go_at_display_time() {
    let mut interp = Interpolator::default();
    interp.push(30, &view(7, EntityKind::TrafficCar, [1.0; 3], [0.0; 3]));
    assert!(interp.sample(7, 29.9).is_none());
    let first = interp.sample(7, 30.0).unwrap();
    assert_eq!(first.fade, 0.0, "traffic fades in");
    assert!((interp.sample(7, 45.0).unwrap().fade - 0.5).abs() < 1.0e-3);
    assert_eq!(interp.sample(7, 61.0).unwrap().fade, 1.0);
    interp.push(33, &view(7, EntityKind::TrafficCar, [1.0; 3], [0.0; 3]));
    interp.remove(7, 40);
    assert!(interp.sample(7, 39.0).is_some());
    assert!(interp.sample(7, 40.0).is_none());
    interp.prune(39.0);
    assert_eq!(interp.len(), 1);
    interp.prune(40.0);
    assert!(interp.is_empty());
}

#[test]
fn a_teleport_is_not_smoothed() {
    let mut interp = Interpolator::default();
    let a = view(1, EntityKind::PlayerCar, [0.0; 3], [10.0, 0.0, 0.0]);
    let b = view(1, EntityKind::PlayerCar, [200.0, 0.0, 0.0], [0.0; 3]);
    interp.push(0, &a);
    interp.push(3, &b);
    assert_eq!(interp.sample(1, 2.9).unwrap().view.position, a.position);
    assert_eq!(interp.sample(1, 3.0).unwrap().view.position, b.position);
}

#[test]
fn records_arriving_out_of_order_are_sorted() {
    let mut interp = Interpolator::default();
    interp.push(6, &view(1, EntityKind::PlayerCar, [2.0, 0.0, 0.0], [0.0; 3]));
    interp.push(0, &view(1, EntityKind::PlayerCar, [0.0; 3], [0.0; 3]));
    interp.push(3, &view(1, EntityKind::PlayerCar, [1.0, 0.0, 0.0], [0.0; 3]));
    let x = interp.sample(1, 4.5).unwrap().view.position[0];
    assert!((x - 1.5).abs() < 0.2, "{x}");
    assert_eq!(interp.newest_tick(1), Some(6));
}

#[test]
fn memory_is_bounded() {
    let config = InterpConfig { max_entities: 8, ..InterpConfig::default() };
    let mut interp = Interpolator::new(config);
    for tick in 0..10_000u32 {
        for id in 0..10u16 {
            interp.push(tick, &view(id, EntityKind::PoliceCar, [0.0; 3], [0.0; 3]));
        }
    }
    assert_eq!(interp.len(), 8);
    assert_eq!(interp.records(), 8 * config.buffer_len);
    assert!(interp.refused() > 0);
    // A silent entity is forgotten after the forget time.
    interp.prune(10_000.0 + config.forget_after_ms / 1000.0 * 60.0 + 1.0);
    assert!(interp.is_empty());
}

#[test]
fn the_delay_is_two_intervals_past_the_transit_without_jitter() {
    let mut delay = AdaptiveDelay::new(&InterpConfig::default());
    for _ in 0..40 {
        delay.on_snapshot(1.5);
        delay.advance(50.0);
    }
    assert!((delay.delay_ticks() - 7.5).abs() < 1.0e-9, "{}", delay.delay_ticks());
    assert_eq!(delay.allowance_ticks(), 0.0);
}

#[test]
fn jitter_raises_the_delay_by_up_to_an_interval_and_it_falls_slowly() {
    let mut delay = AdaptiveDelay::new(&InterpConfig::default());
    for i in 0..64 {
        // Every fourth snapshot is 4 ticks late.
        let age = match i % 4 {
            0 => 5.0,
            _ => 1.0,
        };
        delay.on_snapshot(age);
        delay.advance(50.0);
    }
    assert!((delay.allowance_ticks() - 3.0).abs() < 1.0e-9, "capped at one interval");
    let before = delay.delay_ticks();
    assert!((before - 10.0).abs() < 0.2, "{before}");
    // The jitter stops: the allowance falls by one tick per second, never in one step.
    let mut last = before;
    for _ in 0..200 {
        delay.on_snapshot(1.0);
        delay.advance(50.0);
        assert!(last - delay.delay_ticks() <= 3.0 * 0.05 + 1.0e-9, "slewed");
        last = delay.delay_ticks();
    }
    assert!((delay.delay_ticks() - 7.0).abs() < 1.0e-6, "{}", delay.delay_ticks());
}

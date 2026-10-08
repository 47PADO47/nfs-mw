use super::tables::input_remap;
use super::*;

const DT: f32 = crate::FIXED_STEP;

fn input(steer: f32, speed: f32) -> SteeringInput {
    SteeringInput {
        dt: DT,
        steer,
        gas: 1.0,
        brake: 0.0,
        ebrake: 0.0,
        forward_speed: speed,
        tire_steering: 1.0,
        tuning: 0.0,
        device: SteeringDevice::Pad,
        rear_slip_deg: [0.0, 0.0],
    }
}

fn settle(speed: f32, steer: f32) -> f32 {
    let mut s = Steering::default();
    let mut angle = 0.0;
    for _ in 0..120 {
        angle = s.update(&input(steer, speed));
    }
    angle
}

#[test]
fn ackermann_turns_the_inside_wheel_more() {
    let right = ackermann(20.0, 2.6, 1.6);
    assert!(right.right > right.left && right.left > 0.0);
    assert!((right.right - 20.0_f32.to_radians()).abs() < 1e-5);
    let left = ackermann(-20.0, 2.6, 1.6);
    assert!((left.left + right.right).abs() < 1e-6 && (left.right + right.left).abs() < 1e-6);
    assert_eq!(ackermann(0.0, 2.6, 1.6), WheelAngles::default());
}

#[test]
fn ackermann_matches_the_cotangent_rule_for_small_angles() {
    let a = ackermann(10.0, 2.7, 1.5);
    let exact = (1.0 / a.right.tan() + 1.5 / 2.7).recip().atan();
    assert!((a.left - exact).abs() < 2e-3, "{} vs {}", a.left, exact);
}

#[test]
fn full_lock_at_a_standstill_and_less_at_speed() {
    let slow = settle(0.0, 1.0);
    let mid = settle(20.0, 1.0);
    let fast = settle(60.0, 1.0);
    assert!(slow > 35.0 && slow <= 45.0, "slow {slow}");
    assert!(mid < slow && fast < mid, "{slow} {mid} {fast}");
    assert!(fast < 8.0);
}

#[test]
fn steering_is_symmetric() {
    assert!((settle(15.0, 1.0) + settle(15.0, -1.0)).abs() < 1e-4);
}

#[test]
fn braking_widens_the_angle() {
    let mut coast = Steering::default();
    let mut brake = Steering::default();
    let mut a = 0.0;
    let mut b = 0.0;
    for _ in 0..120 {
        let mut i = input(1.0, 30.0);
        i.gas = 0.0;
        a = coast.update(&i);
        i.brake = 1.0;
        b = brake.update(&i);
    }
    assert!(b > a, "{a} {b}");
}

#[test]
fn the_pad_rate_is_limited() {
    let mut s = Steering::default();
    let first = s.update(&input(1.0, 0.0));
    // One 60 Hz step at 180 deg/s is 3 degrees; a sudden full deflection raises that by the input-speed
    // and input coefficients (at most 3.1 * 1.4).
    assert!(first > 3.0 && first <= 3.0 * 3.1 * 1.4 + 1e-3, "{first}");
    assert!(first < 40.0);
    let mut wheel = Steering::default();
    let mut i = input(1.0, 0.0);
    i.device = SteeringDevice::WheelFixed;
    assert_eq!(wheel.update(&i), 45.0);
}

#[test]
fn ai_steering_is_the_plain_product() {
    let mut s = Steering::default();
    let mut i = input(0.5, 40.0);
    i.device = SteeringDevice::Ai;
    i.tire_steering = 0.8;
    assert!((s.update(&i) - 18.0).abs() < 1e-5);
}

#[test]
fn counter_steer_allows_the_slip_angle_at_speed() {
    let plain = settle(50.0, 1.0);
    let mut s = Steering::default();
    let mut angle = 0.0;
    for _ in 0..180 {
        let mut i = input(1.0, 50.0);
        i.rear_slip_deg = [0.0, 20.0];
        angle = s.update(&i);
    }
    assert!(angle > plain + 5.0 && angle <= 20.0 + 1e-3, "{plain} -> {angle}");
}

#[test]
fn a_hard_hit_limits_the_angle_then_recovers() {
    let mut s = Steering::default();
    for _ in 0..60 {
        s.update(&input(0.3, 20.0));
    }
    let before = s.angle();
    s.notify_collision(60.0);
    let during = s.update(&input(0.3, 20.0));
    assert!(during < before, "{during} < {before}");
    for _ in 0..90 {
        s.update(&input(0.3, 20.0));
    }
    assert!((s.angle() - before).abs() < 0.5);
}

#[test]
fn remap_table_is_odd_and_flat_near_the_centre() {
    let t = input_remap();
    assert_eq!(t[10], 0.0);
    for i in 0..21 {
        assert!((t[i] + t[20 - i]).abs() < 1e-6);
    }
    assert!(t[9].abs() < 0.1 * 0.5);
}

#[test]
fn output_is_always_bounded() {
    let mut s = Steering::default();
    for k in 0..1000 {
        let u = ((k as f32) * 0.37).sin() * 3.0;
        let a = s.update(&input(u, (k % 80) as f32 * 2.0 - 20.0));
        assert!(a.is_finite() && a.abs() <= 45.0 + 1e-3);
    }
}

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
fn human_range_matches_original_pc_full_lock_vectors() {
    // Isolated original-PC maximum-steering outputs; see player-driving-parity.md.
    for (speed, expected) in [(10.0, 38.8125), (40.0, 11.981_25), (80.0, 5.231_25)] {
        for direction in [-1.0, 1.0] {
            let angle = settle(speed, direction);
            assert!((angle - direction * expected).abs() < 1e-4, "speed {speed}: {angle}");
        }
    }
}

#[test]
fn small_pad_input_stays_on_original_flat_range_segment() {
    // Raw 0.2 records 0.061 in the history: the original range multiplier is still one.
    for direction in [-1.0, 1.0] {
        let angle = settle(40.0, direction * 0.2);
        assert!((angle - direction * 1.775).abs() < 1e-4, "angle {angle}");
    }
}

#[test]
fn speed_sensitive_wheel_matches_original_partial_history_range() {
    let mut steering = Steering::default();
    let mut i = input(0.8, 40.0);
    i.device = SteeringDevice::WheelSpeedSensitive;
    for _ in 0..120 {
        steering.update(&i);
    }
    // Original maximum 12.8125 degrees, multiplied by raw input 0.8.
    assert!((steering.angle() - 10.25).abs() < 1e-4, "angle {}", steering.angle());
}

#[test]
fn human_range_keeps_original_tuning_endpoint_multipliers() {
    // Original full-history range at 40 m/s, with the verified 0.8/1.2 tuning factors.
    for (tuning, expected) in [(-1.0, 9.585), (0.0, 11.981_25), (1.0, 14.377_5)] {
        for direction in [-1.0, 1.0] {
            let mut steering = Steering::default();
            let mut i = input(direction, 40.0);
            i.tuning = tuning;
            for _ in 0..120 {
                steering.update(&i);
            }
            assert!((steering.angle() - direction * expected).abs() < 1e-4, "tuning {tuning}");
        }
    }
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
fn partial_pad_history_matches_original_pc_remap_vectors() {
    // Outputs of the selected PC lookup routine, not values derived from the Rust table.
    for (raw, expected) in [
        (-1.1, -1.0),
        (-0.95, -0.856),
        (-0.8, -0.453),
        (-0.45, -0.132),
        (-0.2, -0.061),
        (0.0, 0.0),
        (0.15, 0.0475),
        (0.4, 0.116),
        (0.8, 0.453),
        (1.1, 1.0),
    ] {
        let mut steering = Steering::default();
        steering.update(&input(raw, 0.0));
        assert!((steering.average_input() - expected).abs() < 1e-6, "raw {raw}");
    }
}

#[test]
fn partial_pad_transients_match_original_at_normal_and_slowed_tick_deltas() {
    // Complete original human-steering routine with fixed 40 m/s, neutral tuning and no rear slip.
    // The smaller dt values are observed at 144 Hz, normally and at full Speedbreaker.
    let cases = [
        (1.0 / 60.0, [3.55, 3.55, 3.55, 3.55, 3.55, -3.55, -3.55, -3.55, -3.55, -3.55, 0.0, 0.0]),
        (
            1.0 / 144.0,
            [3.448_75, 3.55, 3.55, 3.55, 3.55, 0.001_236_09, -3.514_19, -3.55, -3.55, -3.55, -0.101_249_75, 0.0],
        ),
        (
            1.0 / 576.0,
            [
                0.862_187_5,
                1.749_378_4,
                2.636_569_3,
                3.523_76,
                3.55,
                2.662_809,
                1.783_952_4,
                0.911_049_07,
                0.042_610_66,
                -0.822_355_03,
                0.0,
                0.0,
            ],
        ),
    ];
    for (dt, expected) in cases {
        let mut steering = Steering::default();
        for (step, expected_angle) in expected.into_iter().enumerate() {
            let raw = match step {
                0..5 => 0.4,
                5..10 => -0.4,
                _ => 0.0,
            };
            let mut i = input(raw, 40.0);
            i.dt = dt;
            let angle = steering.update(&i);
            assert!((angle - expected_angle).abs() < 1e-4, "dt {dt}, step {step}: {angle}");
        }
    }
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

#[test]
fn partial_input_uses_previous_remap_and_averages_rate_coefficients() {
    let mut s = Steering::default();
    // First derivative is 0.4 / dt = 24: coefficient 3.1, previous magnitude average 0.
    let first = s.update(&input(0.4, 0.0));
    assert!((first - 9.3).abs() < 1e-4, "first {first}");
    // Original PC: last remapped input 0.116, raw 0.2 gives 5.04/s, coefficient 1.308.
    // The recorded mean is (3.1 + 1.308)/2, rather than lookup(mean derivatives).
    let mut i = input(0.2, 0.0);
    i.tire_steering = 0.1;
    let second = s.update(&i);
    // Independent original rate helper: 408.224884 degrees/s before tire steering.
    assert!((second - 8.619_625).abs() < 1e-4, "second {second}");
}

#[test]
fn fixed_racing_wheel_bypasses_range_changes() {
    let mut s = Steering::default();
    s.notify_collision(40.0);
    let mut i = input(0.5, 100.0);
    i.device = SteeringDevice::WheelFixed;
    i.tuning = -1.0;
    assert!((s.update(&i) - 22.5).abs() < 1e-5);
}

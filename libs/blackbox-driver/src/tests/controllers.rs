use glam::Vec3;

use crate::*;

fn view(speed: f32) -> VehicleView {
    VehicleView {
        position: Vec3::ZERO,
        forward: Vec3::Z,
        forward_speed: speed,
        planar_speed: speed.abs(),
        gear_is_reverse: false,
        max_steer: 45f32.to_radians(),
        in_shock: false,
        staging: false,
    }
}

fn request(target: Vec3, speed: f32, flags: DriveFlags) -> DriveRequest {
    DriveRequest { target, speed, flags }
}

#[test]
fn table_and_graph_clamp_and_interpolate() {
    let table = Table { data: [0.0, 10.0, 20.0], min: 0.0, max: 2.0 };
    assert_eq!(table.lookup(-5.0), 0.0);
    assert!((table.lookup(0.5) - 5.0).abs() < 1e-5);
    assert_eq!(table.lookup(9.0), 20.0);
    let graph = Graph { points: [(0.0, 0.0), (10.0, -10.0)] };
    assert!((graph.lookup(5.0) + 5.0).abs() < 1e-5);
    assert_eq!(graph.lookup(99.0), -10.0);
    assert_eq!(ramp(5.0, 0.0, 10.0), 0.5);
}

#[test]
fn pid_error_matches_hand_computed_values() {
    let mut e = PidError::new(4, 4, 30.0);
    for _ in 0..4 {
        e.record(2.0, 1.0 / 60.0);
    }
    // Slopes: the first sample jumps 0 -> 2, the rest are flat.
    assert!((e.derivative() - (2.0 * 60.0) / 4.0).abs() < 1e-3);
    assert_eq!(e.error(), 2.0);
    // With a constant step the integral is about the sum of the last errors over the frequency.
    assert!(e.integral() > 0.0);
}

#[test]
fn simple_steering_points_at_the_target() {
    let (steer, behind) = simple_steering(Vec3::ZERO, Vec3::Z, Vec3::new(10.0, 0.0, 10.0), 45f32.to_radians(), false);
    assert!((steer - 1.0).abs() < 1e-3, "45 degrees right is full lock: {steer}");
    assert!(!behind);
    let (left, _) = simple_steering(Vec3::ZERO, Vec3::Z, Vec3::new(-2.0, 0.0, 20.0), 45f32.to_radians(), false);
    assert!(left < 0.0 && left > -0.5);
    let (back, behind) = simple_steering(Vec3::ZERO, Vec3::Z, Vec3::new(1.0, 0.0, -10.0), 45f32.to_radians(), false);
    assert!(behind && back == 1.0);
}

#[test]
fn simple_pedals_are_bang_bang() {
    let p = |speed, want| simple_pedals(speed, want, false, false, false);
    assert_eq!(p(10.0, 15.0).gas, 1.0);
    assert_eq!(p(15.0, 10.0).brake, 1.0);
    // Within 2.5 m/s above the wanted speed the car coasts.
    assert_eq!(p(11.5, 10.0), Pedals::default());
    // Asked to stop: brake.
    assert_eq!(p(3.0, 0.0).brake, 1.0);
    // A handbrake turn when the target is behind.
    let turn = simple_pedals(5.0, 10.0, false, true, false);
    assert_eq!((turn.gas, turn.handbrake), (1.0, 1.0));
}

#[test]
fn throttle_pid_reaches_full_gas_then_eases_off() {
    let mut pid = ThrottlePid::default();
    let mut speed = 20.0f32;
    let mut peak: f32 = 0.0;
    for _ in 0..600 {
        let out = pid.step(
            &ThrottleInput {
                speed,
                want: 30.0,
                reversing_gear: false,
                staging: false,
                steering_behind: false,
                reversing_speed: false,
            },
            1.0 / 60.0,
        );
        peak = peak.max(out.gas);
        // A crude car: 6 m/s^2 at full gas, 8 m/s^2 of braking.
        speed += (out.gas * 6.0 - out.brake * 8.0 - 0.02 * speed) / 60.0;
    }
    assert_eq!(peak, 1.0);
    assert!((speed - 30.0).abs() < 3.0, "settled at {speed}");
}

#[test]
fn steering_pid_turns_towards_the_target_and_calms_down() {
    let mut pid = SteeringPid::new(AdaptiveSettings::RACING);
    let mut heading = 0.0f32; // radians, positive towards the target at +x
    let target_angle = 15f32.to_radians();
    let mut peak: f32 = 0.0;
    for tick in 0..1200 {
        let error = target_angle - heading;
        let steer = pid.step(error, 25.0, 45f32.to_radians(), 1.0 / 60.0, tick as f32 / 60.0);
        peak = peak.max(steer.abs());
        // A crude car: yaw rate proportional to steering and speed.
        heading += steer * 0.35 / 60.0;
    }
    assert!(peak > 0.0);
    assert!(
        (target_angle - heading).abs() < 3f32.to_radians(),
        "heading error left: {}",
        (target_angle - heading).to_degrees()
    );
}

#[test]
fn the_driver_asks_for_reverse_when_the_target_is_behind() {
    let mut driver = Driver::new(ControllerKind::Pid, false);
    let req = request(Vec3::new(0.0, 0.0, -20.0), 8.0, DriveFlags::FULL);
    let out = driver.step(&view(0.5), &req, 1.0 / 60.0, 0.0);
    assert_eq!(out.gear, Some(GearRequest::Reverse));
    // Reversing and now facing the target: back to first.
    let mut v = view(-1.0);
    v.gear_is_reverse = true;
    v.forward = -Vec3::Z;
    let out = driver.step(&v, &req, 1.0 / 60.0, 0.1);
    assert_eq!(out.gear, Some(GearRequest::First));
}

#[test]
fn traffic_in_shock_coasts_in_neutral() {
    let mut driver = Driver::new(ControllerKind::Simple, false);
    let req = request(Vec3::new(0.0, 0.0, 30.0), 15.0, DriveFlags::SIMPLE);
    let mut v = view(10.0);
    v.in_shock = true;
    let out = driver.step(&v, &req, 0.1, 0.0);
    assert_eq!((out.gas, out.brake, out.gear), (0.0, 0.0, Some(GearRequest::Neutral)));
    v.in_shock = false;
    let out = driver.step(&v, &req, 0.1, 0.1);
    assert_eq!(out.gear, Some(GearRequest::First));
    assert_eq!(out.gas, 1.0);
}

#[test]
fn a_stuck_car_is_noticed_after_three_seconds() {
    let mut detector = StuckDetector::default();
    let mut stuck = false;
    for i in 0..40 {
        stuck |= detector.update(0.1, true, false, false, Vec3::new(0.0, 0.0, 0.01 * i as f32));
    }
    assert!(stuck);
    let mut moving = StuckDetector::default();
    let mut ever = false;
    for i in 0..40 {
        ever |= moving.update(0.1, true, false, false, Vec3::new(0.0, 0.0, i as f32));
    }
    assert!(!ever);
}

#[test]
fn a_reverse_override_runs_out_and_returns_to_first() {
    let mut driver = Driver::new(ControllerKind::Pid, false);
    assert_eq!(driver.start_reverse_override(0.2, false), GearRequest::Reverse);
    let req = request(Vec3::new(0.0, 0.0, 20.0), 15.0, DriveFlags::SIMPLE);
    let mut v = view(0.0);
    v.gear_is_reverse = true;
    let mut gear = None;
    for _ in 0..30 {
        gear = driver.step(&v, &req, 1.0 / 60.0, 0.0).gear.or(gear);
    }
    assert_eq!(gear, Some(GearRequest::First));
    assert_eq!(driver.reverse_override_left(), 0.0);
}

mod speed {
    use glam::Vec2;

    use crate::*;

    fn limits() -> CarLimits {
        CarLimits {
            top_speed: 80.0,
            start_grip: 1.0,
            end_grip: 1.3,
            top_speed_multiplier: 1.0,
            acceleration_multiplier: 1.0,
        }
    }

    #[test]
    fn a_straight_road_allows_the_top_speed_and_a_tight_bend_far_less() {
        assert!((speed_limit_for(0.0, 1.0, 0.002, 80.0) - 80.0).abs() < 1e-3);
        let bend = speed_limit_for(1.0 / 25.0, 1.0, 0.002, 80.0);
        // A 25 m radius at about 1 g of grip holds only a modest speed.
        assert!((8.0..20.0).contains(&bend), "{bend}");
        assert!(speed_limit_for(1.0 / 100.0, 1.0, 0.002, 80.0) > bend);
    }

    #[test]
    fn skilled_drivers_corner_faster() {
        let l = limits();
        let slow = potential_speed(&l, 0.02, 0.0, None);
        let fast = potential_speed(&l, 0.02, 1.0, None);
        assert!(fast > slow * 1.1, "{slow} {fast}");
    }

    #[test]
    fn the_governor_follows_the_acceleration_not_the_potential() {
        let mut g = SpeedGovernor::begin(0.0);
        let (mut speed, mut asked) = (0.0f32, Vec::new());
        for i in 0..120 {
            let wanted = g.update(speed, 60.0, 8.0, 0.5, 1.0 / 30.0);
            // A car that accelerates at 6 m/s^2 towards the request.
            speed += ((wanted - speed) * 4.0).clamp(-8.0, 6.0) / 30.0;
            if i % 30 == 29 {
                asked.push(wanted);
            }
        }
        assert!(asked[0] < 25.0, "the request does not leap to the top: {asked:?}");
        assert!(asked[3] > asked[0] + 10.0, "{asked:?}");
        assert!(g.speed_limit <= 60.0);
    }

    #[test]
    fn a_cop_far_behind_its_target_may_use_all_its_speed_and_near_it_follows_the_target() {
        let base = PursuitContext {
            distant: 70.0,
            offset: Vec2::new(0.0, -300.0),
            seek_dir: Vec2::Y,
            target_speed: 20.0,
            forward: Vec2::Y,
            steer_dir: Vec2::Y,
            target_steer_dir: Vec2::Y,
            race_running: false,
            jerk: false,
        };
        assert!((base.max_cop_speed() - 70.0).abs() < 1.0, "{}", base.max_cop_speed());
        let near = PursuitContext { offset: Vec2::new(0.0, -20.0), ..base };
        assert!(near.max_cop_speed() < 70.0, "{}", near.max_cop_speed());
        let l = limits();
        assert!(potential_speed(&l, 0.0, 0.5, Some(&near)) <= near.max_cop_speed());
    }
}

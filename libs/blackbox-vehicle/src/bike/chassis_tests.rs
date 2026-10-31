//! The fall tests and the day the bike rides in (`docs/specs/ride6-chassis-round6.md`).

use glam::{Quat, Vec3};

use super::super::{Bike, BikeInput, BikeSpec, Environment};
use crate::FIXED_STEP;
use crate::ground::FlatGround;

fn settled() -> (Bike, FlatGround) {
    let ground = FlatGround::new(0.0);
    let mut bike = Bike::new(BikeSpec::example());
    assert!(bike.place_on_ground(&ground, 0.0, 0.0, 2.0, 0.0));
    for _ in 0..(4.0 / FIXED_STEP) as usize {
        bike.step(FIXED_STEP, &BikeInput::default(), &ground);
    }
    (bike, ground)
}

#[test]
fn the_angular_speed_is_tested_at_any_speed_and_the_lean_is_not_a_fall_for_a_moment() {
    let (mut bike, ground) = settled();
    assert!(!bike.fallen());
    bike.body_mut().angular_velocity = Vec3::new(0.0, 5.0, 0.0);
    bike.step(FIXED_STEP, &BikeInput::default(), &ground);
    assert!(bike.fallen(), "a bike turning at 5 rad/s while standing has fallen");
}

#[test]
fn the_fall_lean_limit_follows_the_grip_and_the_lean_must_stay_over_it_for_the_duration() {
    let (mut bike, ground) = settled();
    let spec_limit = bike.spec().fall.lean;
    let limit = bike.chassis.fall_lean;
    // It follows the front tyre's grip angle times the wheel's `LateralGripLimit` (65 degrees here), not the 70 of its start.
    assert!(limit < spec_limit && limit > 0.5, "{limit} {spec_limit}");
    // Beyond the limit for one step is not yet a fall; for the duration (0.01 s: two steps) it is.
    bike.body_mut().set_orientation(Quat::from_rotation_z(limit + 0.1));
    bike.step(FIXED_STEP, &BikeInput::default(), &ground);
    assert!(!bike.fallen());
    bike.step(FIXED_STEP, &BikeInput::default(), &ground);
    bike.step(FIXED_STEP, &BikeInput::default(), &ground);
    assert!(bike.fallen());
}

#[test]
fn the_lean_limit_rises_slowly_and_falls_slower() {
    let mut state = super::chassis_state::ChassisState::new(&BikeSpec::example().fall);
    state.fall_lean = 1.0;
    state.follow_grip(&BikeSpec::example().fall, 0.5);
    assert!((state.fall_lean - 0.995).abs() < 1e-6, "{}", state.fall_lean);
    state.follow_grip(&BikeSpec::example().fall, 1.5);
    assert!((state.fall_lean - (0.995 + 0.505 * 0.025)).abs() < 1e-5, "{}", state.fall_lean);
}

#[test]
fn the_tyres_cool_towards_the_day_the_host_sets() {
    let ground = FlatGround::new(0.0);
    let temperature_after = |environment: Environment| {
        let mut bike = Bike::new(BikeSpec::example());
        bike.set_environment(environment);
        assert_eq!(bike.environment(), environment);
        assert!(bike.place_on_ground(&ground, 0.0, 0.0, 2.0, 0.0));
        for _ in 0..(20.0 / FIXED_STEP) as usize {
            bike.step(FIXED_STEP, &BikeInput::default(), &ground);
        }
        bike.tyre_temperature(0)
    };
    let hot = temperature_after(Environment { air: 40.0, asphalt: 55.0 });
    let cold = temperature_after(Environment { air: 5.0, asphalt: 5.0 });
    assert!(hot > cold + 1.0, "{hot} {cold}");
}

#[test]
fn a_held_lean_stays_up_and_sits_a_little_off_its_target_with_no_feed_forward() {
    // The reference closes the roll with the tyre and the lean help alone: the held lean settles under a degree and a half off its
    // target (`ride6-chassis-round7.md`, section 2). Held at the stick's 0.9 up to 20 m/s, 0.8 at 30, for six seconds.
    let ground = FlatGround::new(0.0);
    for (speed, stick) in [(10.0, 0.9), (20.0, 0.9), (30.0, 0.8)] {
        let mut bike = Bike::new(BikeSpec::example());
        assert!(bike.place_moving(&ground, 0.0, 0.0, 2.0, 0.0, speed));
        for _ in 0..(6.0 / FIXED_STEP) as usize {
            bike.step(FIXED_STEP, &BikeInput { throttle: 0.2, steer: stick, ..Default::default() }, &ground);
            assert!(!bike.fallen(), "fell at {speed} m/s, stick {stick}");
        }
        let error = bike.lean() - bike.target_lean();
        assert!(error > 0.0 && error < 0.04, "{speed} m/s: lean {} target {}", bike.lean(), bike.target_lean());
    }
}

#[test]
fn a_step_is_divided_into_sub_steps_and_the_bike_advances_exactly_the_step() {
    let ground = FlatGround::new(0.0);
    let run_for = |dt: f32, seconds: f32| {
        let mut bike = Bike::new(BikeSpec::example());
        assert!(bike.place_moving(&ground, 0.0, 0.0, 2.0, 0.0, 20.0));
        for _ in 0..(seconds / dt) as usize {
            bike.step(dt, &BikeInput { throttle: 0.2, ..Default::default() }, &ground);
        }
        bike
    };
    // One second at 60 Hz (four sub-steps each) and at a single sub-step's own rate go the same way, to a few centimetres.
    let (coarse, fine) = (run_for(FIXED_STEP, 1.0), run_for(crate::bike::choices::MAX_SUBSTEP, 1.0));
    assert!(coarse.position().distance(fine.position()) < 0.2, "{} {}", coarse.position(), fine.position());
    assert!((coarse.position().z - 20.0).abs() < 1.0, "{}", coarse.position());
}

/// A road tilted across the heading: lower on the right by `bank` (rise over run), `height(x) = -x bank`.
struct BankedGround {
    bank: f32,
}

impl crate::ground::Ground for BankedGround {
    fn hit(&self, origin: Vec3, dir: Vec3, max_distance: f32) -> Option<crate::ground::GroundHit> {
        let normal = Vec3::new(self.bank, 1.0, 0.0).normalize();
        let denom = normal.dot(dir);
        if denom >= -1e-6 {
            return None;
        }
        let distance = normal.dot(Vec3::ZERO - origin) / denom;
        (distance >= 0.0 && distance <= max_distance).then_some(crate::ground::GroundHit {
            distance,
            normal,
            surface: crate::ground::SurfaceGrip::DEFAULT,
        })
    }
}

#[test]
fn the_bank_is_positive_when_the_road_is_lower_on_the_right_and_zero_on_the_flat() {
    let (flat, _) = settled();
    assert!(flat.terrain_slope().abs() < 0.01, "{}", flat.terrain_slope());
    for (bank, sign) in [(0.1_f32, 1.0), (-0.1, -1.0)] {
        let ground = BankedGround { bank };
        let mut bike = Bike::new(BikeSpec::example());
        assert!(bike.place_on_ground(&ground, 0.0, 0.0, 2.0, 0.0));
        for _ in 0..30 {
            bike.step(FIXED_STEP, &BikeInput::default(), &ground);
        }
        let slope = bike.terrain_slope();
        assert!(slope * sign > 0.05 && slope * sign < 0.12, "{bank}: {slope}");
    }
}

#[test]
fn the_lean_limit_rises_leaning_into_a_rising_road_and_falls_leaning_off_it() {
    use super::super::lean::{LeanInput, lean_limit};
    let steering = BikeSpec::example().steering;
    let input = |stick: f32, terrain_slope: f32| LeanInput {
        stick,
        speed: 30.0,
        lean: 0.0,
        lean_rate: 0.0,
        lateral_grip: 1.3,
        terrain_slope,
        lateral_g: 0.0,
        heading_error: 0.0,
        pitch_cut: (0.0, 0.0),
        roll_inertia: 47.8,
        cg_height: 0.8,
        front_arm: 1.0,
        target_cap: f32::MAX,
        dt: FIXED_STEP,
    };
    let flat = lean_limit(&steering, &input(1.0, 0.0));
    // Turning right with the road lower on the right (a banked right-hander): the limit rises by two thirds of the bank.
    let banked = lean_limit(&steering, &input(1.0, 0.1));
    assert!((banked - flat - 0.1 * 0.666_667).abs() < 1e-4, "{banked} {flat}");
    // Turning right with the road higher on the right: it falls by the whole bank.
    assert!((lean_limit(&steering, &input(1.0, -0.1)) - (flat - 0.1)).abs() < 1e-4);
    // Turning left reverses both, and a bank of 20 degrees counts as 15.
    assert!((lean_limit(&steering, &input(-1.0, 0.1)) - (flat - 0.1)).abs() < 1e-4);
    let steep = lean_limit(&steering, &input(1.0, 0.35));
    assert!((steep - flat - 15.0_f32.to_radians() * 0.666_667).abs() < 1e-4, "{steep}");
    // At a crawl the speed curve has not brought it in.
    let slow = lean_limit(&steering, &LeanInput { speed: 1.0, ..input(1.0, 0.1) });
    assert!((slow - lean_limit(&steering, &LeanInput { speed: 1.0, ..input(1.0, 0.0) })).abs() < 1e-5);
}

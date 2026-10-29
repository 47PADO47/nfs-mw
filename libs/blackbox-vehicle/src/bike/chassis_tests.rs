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
    assert!(limit < spec_limit - 0.1 && limit > 0.5, "{limit} {spec_limit}");
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

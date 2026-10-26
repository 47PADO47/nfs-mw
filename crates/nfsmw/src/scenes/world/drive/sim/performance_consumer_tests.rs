use crate::input::ActionState;
use crate::scenes::world::drive::time_scale::SpeedbreakerClock;
use crate::scenes::world::drive::{FixedClock, STEP, observer_tests};

#[test]
fn frame_depletion_selects_one_meter_clock_and_vehicle_state_even_without_a_tick() {
    let (mut drive, mut renderer) = observer_tests::fixture();
    let mut tuning = drive.breaker.tuning();
    // Isolate real-time depletion and its consumers from wheel-speed eligibility.
    tuning.min_speed = 0.0;
    drive.set_speedbreaker_tuning(tuning);
    drive.breaker.meter = 0.001;
    drive.frame_input.speedbreaker = true;
    let mut clock = FixedClock::default();
    let mut ramp = SpeedbreakerClock::default();
    let active = drive.prepare_time(0.001);
    assert!(active && drive.breaker.active());
    assert!(clock.advance_real(0.001).is_empty(), "the meter must run before any physical tick is due");
    let vehicle = &drive.sim.as_ref().unwrap().vehicle;
    assert!(!vehicle.can_recharge_nos);
    assert_eq!(vehicle.maneuver.collision_mass_scale, Some(2.0));
    let entry_delta = ramp.advance_tick(active, STEP);
    assert!(entry_delta < STEP);
    let charge = drive.breaker.meter;
    drive.finish_visual_time(&ActionState::default(), 0.001, 0.001);
    assert_eq!(drive.breaker.meter, charge, "presentation cannot drain a second time");

    let active = drive.prepare_time(0.02);
    assert!(!active && !drive.breaker.active());
    assert_eq!(drive.breaker.meter, 0.0);
    let vehicle = &drive.sim.as_ref().unwrap().vehicle;
    assert!(vehicle.can_recharge_nos);
    assert_eq!(vehicle.maneuver.collision_mass_scale, None);
    assert_eq!(vehicle.maneuver.extra_downward_acceleration, 0.0);
    assert_eq!(vehicle.maneuver.drag_scale, 1.0);
    assert!(ramp.advance_tick(active, STEP) > entry_delta, "the clock exits when vehicle assists end");
    drive.rig.release(&mut renderer);
}

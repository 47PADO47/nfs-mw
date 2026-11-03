//! Round 10 (`docs/specs/ride6-feel-round10.md`): what a collision does to the controllers' help, and a rider thrown by the host.

use super::handling::chassis_state::CollisionHelp;
use super::{Bike, BikeInput, BikeSpec, FallKind, choices};
use crate::FIXED_STEP;
use crate::ground::FlatGround;

#[test]
fn a_collision_adds_to_the_help_and_the_addition_fades_at_the_rate_of_the_data() {
    let mut help = CollisionHelp::default();
    help.collide(false);
    assert_eq!((help.lean, help.stability), (2.0, 0.4));
    // The lean help's addition lasts 2 / 5 = 0.4 s, the stability help's 0.4 / 5 = 0.08 s.
    help.fade(0.1);
    assert!((help.lean - 1.5).abs() < 1e-6 && help.stability == 0.0, "{help:?}");
    help.fade(0.3);
    assert_eq!(help.lean, 0.0);
}

#[test]
fn a_frontal_collision_gives_the_stability_help_less_and_for_longer() {
    let mut help = CollisionHelp::default();
    help.collide(true);
    assert_eq!((help.lean, help.stability), (2.0, 0.2));
    help.fade(0.1);
    assert!((help.stability - 0.1).abs() < 1e-6, "fading at 1 per second: {help:?}");
    // A second collision does not lower what is left of the first.
    help.collide(false);
    assert_eq!(help.stability, 0.4);
}

#[test]
fn a_bike_tells_the_help_of_a_collision_and_it_fades_in_its_own_steps() {
    let mut bike = Bike::new(BikeSpec::example());
    let ground = FlatGround::new(0.0);
    assert!(bike.place_moving(&ground, 0.0, 0.0, 2.0, 0.0, 10.0));
    assert_eq!(bike.collision_help(), (0.0, 0.0));
    bike.notify_collision(false);
    assert_eq!(bike.collision_help(), (choices::COLLISION_LEAN_HELP.0, choices::COLLISION_STABILITY_HELP.0));
    for _ in 0..(0.5 / FIXED_STEP) as usize {
        bike.step(FIXED_STEP, &BikeInput::default(), &ground);
    }
    assert_eq!(bike.collision_help(), (0.0, 0.0));
    assert!(!bike.fallen());
}

#[test]
fn a_rider_thrown_by_the_host_is_released_once() {
    let mut bike = Bike::new(BikeSpec::example());
    assert!(bike.place_moving(&FlatGround::new(0.0), 0.0, 0.0, 2.0, 0.0, 20.0));
    bike.throw_rider(FallKind::Generic);
    assert!(bike.fallen() && bike.rider_off());
    assert_eq!(bike.fall_kind(), Some(FallKind::Generic));
    assert!(bike.take_rider_release().is_some());
    bike.throw_rider(FallKind::HighSide);
    assert_eq!(bike.fall_kind(), Some(FallKind::Generic), "a bike that has fallen has fallen");
    assert!(bike.take_rider_release().is_none());
}

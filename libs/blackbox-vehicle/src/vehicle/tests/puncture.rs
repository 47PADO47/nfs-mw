//! Spike-strip tire damage on the vehicle: a punctured tire is inert until its fuse runs out; a blown tire
//! brakes fully, releases the handbrake and keeps 30 % of its traction (`tires::damage`).

use super::*;
use crate::tires::{BLOWN_TRACTION, TireDamage};

/// The example car at speed, with `damage` set and one step taken with the brake on.
fn braked_with(damage: TireDamage) -> Vehicle {
    let mut v = parked();
    run(&mut v, &InputState { throttle: 1.0, ..Default::default() }, 2.0);
    v.tire_damage = damage;
    v.step(FIXED_STEP, &InputState { brake: 1.0, ..Default::default() }, &flat());
    v
}

fn blown(wheel: usize) -> TireDamage {
    let mut t = TireDamage::default();
    t.puncture(wheel);
    t.advance(1.0);
    assert!(t.is_blown(wheel));
    t
}

#[test]
fn a_blown_tire_takes_the_blown_brake_command() {
    let v = braked_with(blown(0));
    assert_eq!(v.tires[0].brake, 1.0, "the blown command");
    assert_eq!(v.tires[0].ebrake, 0.0);
    let sound = braked_with(TireDamage::default());
    assert_eq!(v.tires[1].brake, sound.tires[1].brake, "a sound tire keeps the normal command");
}

#[test]
fn a_blown_tire_grips_at_thirty_percent_of_a_sound_one() {
    let sound = braked_with(TireDamage::default());
    let flat = braked_with(blown(3));
    let ratio = flat.tires[3].traction_boost / sound.tires[3].traction_boost;
    assert!((ratio - BLOWN_TRACTION).abs() < 1e-4, "ratio {ratio}");
    assert!((flat.tires[0].traction_boost - sound.tires[0].traction_boost).abs() < 1e-6, "other tires match");
}

#[test]
fn a_punctured_tire_has_no_effect_before_it_blows() {
    let mut punctured = TireDamage::default();
    punctured.puncture(0);
    let v = braked_with(punctured);
    let sound = braked_with(TireDamage::default());
    assert_eq!(v.tires[0].brake, sound.tires[0].brake);
    assert_eq!(v.tires[0].traction_boost, sound.tires[0].traction_boost);
}

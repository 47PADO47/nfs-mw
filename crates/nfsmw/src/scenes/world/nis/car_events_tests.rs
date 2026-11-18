use super::*;

#[test]
fn pulses_end_at_rest_and_channels_do_not_depend_on_event_order() {
    assert_eq!(slot_of(blackbox_attrib::vlt_hash("car2")), Some(1));
    assert_eq!(slot_of(blackbox_attrib::vlt_hash("cop3")), Some(10));
    assert_eq!(slot_of(blackbox_attrib::vlt_hash("camera")), None);
    let mut motion = CarMotion::default();
    motion.apply(CarEvent::Pitch { dip: 0.1, length: 2.0 }, 1.0);
    assert_eq!(motion.rotation(1.0, 2.5, 1.6), Quat::IDENTITY);
    assert!(motion.rotation(2.0, 2.5, 1.6).angle_between(Quat::IDENTITY) > 0.01);
    assert_eq!(motion.rotation(3.0, 2.5, 1.6), Quat::IDENTITY);
    assert_eq!(motion.rotation(30.0, 2.5, 1.6), Quat::IDENTITY);
}

#[test]
fn a_shake_ramps_in_whole_cycles_peaks_at_each_cycle_start_and_stops_at_its_end() {
    let shake = Shake { dip: 0.01, rate: 0.5, ramp: 1.0, start: 0.0, length: 20.0 };
    let full = ai_angle(0.01, 1.8);
    // Ramp: 1 / 0.5 + 0.99 = 2.99, so 2 cycles; cycle 0 is silent, cycle 1 at half.
    assert_eq!(shake.angle(0.1, 1.8), 0.0);
    assert!((shake.angle(0.5, 1.8) - full * 0.5).abs() < 1e-6, "cycle 1 starts at half amplitude");
    assert!((shake.angle(1.0, 1.8) - full).abs() < 1e-6, "full from cycle 2");
    assert!((shake.angle(1.25, 1.8) + full).abs() < 1e-5, "half a cycle on, the cosine is at -1");
    // 40 cycles in all: the last two ramp down.
    assert!((shake.angle(19.5, 1.8) - full * 0.5).abs() < 1e-6);
    assert_eq!(shake.angle(20.0, 1.8), 0.0, "over at its length");
    let none = Shake { rate: 0.0, ..shake };
    assert_eq!(none.angle(5.0, 1.8), 0.0, "a zero rate does nothing");
}

#[test]
fn detach_and_reattach_toggle_the_channel_and_repairs_count() {
    let mut motion = CarMotion::default();
    assert!(motion.attached);
    motion.apply(CarEvent::Detach, 1.0);
    assert!(!motion.attached);
    motion.apply(CarEvent::Reattach { repair: false }, 2.0);
    assert!(motion.attached);
    assert_eq!(motion.repairs, 0);
    motion.apply(CarEvent::Reattach { repair: true }, 3.0);
    motion.apply(CarEvent::DamageReset, 3.0);
    assert_eq!(motion.repairs, 2);
}

#[test]
fn locks_lights_nitro_and_revs_are_kept_until_changed() {
    let mut motion = CarMotion::default();
    motion.apply(CarEvent::BrakeLock { front: false, rear: true }, 0.0);
    assert!(!motion.locked(false) && motion.locked(true));
    motion.apply(CarEvent::Lights { cop: true, head: false }, 0.0);
    assert_eq!(motion.lights, Some((true, false)));
    motion.apply(CarEvent::Nitro(true), 0.0);
    assert!(motion.nitro);
    motion.apply(CarEvent::NeutralRev { on: true, throttle: 0.8, speed: 2.0 }, 0.0);
    assert_eq!(motion.neutral_rev, Some((0.8, 2.0)));
    motion.apply(CarEvent::NeutralRev { on: false, throttle: 0.8, speed: 2.0 }, 0.0);
    assert_eq!(motion.neutral_rev, None);
    assert_eq!(slot_name(0), "car1");
    assert_eq!(slot_name(15), "cop8");
}

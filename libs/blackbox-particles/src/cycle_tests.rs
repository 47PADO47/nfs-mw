//! The on and off cycles of an emitter: births stop in the off phase and start again with the next on phase.

use crate::{Emitter, EmitterSpec, Frame};

#[test]
fn an_on_off_emitter_spawns_only_during_its_on_phases() {
    let spec =
        EmitterSpec { rate: 10.0, life: 100.0, size: [0.2; 4], on_cycle: 1.0, off_cycle: 1.0, ..Default::default() };
    let mut emitter = Emitter::new(spec, 7);
    let frame = Frame { intensity: 1.0, ..Frame::default() };
    let dt = 0.05;
    let mut counts = Vec::new();
    for _ in 0..60 {
        emitter.step(dt, &frame);
        counts.push(emitter.len());
    }
    // Index i is the count at time (i + 1) * dt.
    let (first_on, off_end, second_on) = (counts[19], counts[37], counts[49]);
    assert!(first_on > 0, "the first on-cycle spawns");
    assert_eq!(first_on, off_end, "the off-cycle spawns nothing");
    assert!(second_on > off_end, "the next on-cycle spawns again");
}

#[test]
fn a_one_shot_emitter_stops_after_its_first_on_cycle() {
    let spec = EmitterSpec {
        rate: 10.0,
        life: 100.0,
        size: [0.2; 4],
        on_cycle: 1.0,
        off_cycle: 1.0,
        one_shot: true,
        ..Default::default()
    };
    let mut emitter = Emitter::new(spec, 7);
    let frame = Frame { intensity: 1.0, ..Frame::default() };
    for _ in 0..60 {
        emitter.step(0.05, &frame);
    }
    let once = emitter.len();
    assert!(once > 0);
    for _ in 0..40 {
        emitter.step(0.05, &frame);
    }
    assert_eq!(emitter.len(), once, "no second cycle");
}

#[test]
fn a_start_delay_holds_back_the_first_birth() {
    let spec =
        EmitterSpec { rate: 10.0, life: 100.0, size: [0.2; 4], on_cycle: 1.0, start_delay: 1.0, ..Default::default() };
    let mut emitter = Emitter::new(spec, 7);
    let frame = Frame { intensity: 1.0, ..Frame::default() };
    for _ in 0..19 {
        emitter.step(0.05, &frame);
    }
    assert_eq!(emitter.len(), 0, "still delayed");
    for _ in 0..10 {
        emitter.step(0.05, &frame);
    }
    assert!(emitter.len() > 0, "after the delay");
}

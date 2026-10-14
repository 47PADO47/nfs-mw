use super::*;
use crate::{EnvelopeKind, Event, InputKey};

/// An event of `kind` fading by `swing` with attack and release of 6 frames (100 ms) on a linear-down curve,
/// triggered by object input 0; its dB output feeds a sub channel so a test can read it.
fn event_state(kind: u32, swing: i32, sustain_frames: u32) -> State {
    let p = 0x8000 | 6; // shape 8, 6 frames
    State {
        events: vec![Event {
            id: SourceId(0xA000_3000 | (kind << 24)),
            swing,
            trigger: object_input(0, 3, 0),
            params: [p, sustain_frames, p],
            scales: vec![],
        }],
        subs: vec![sub(vec![event_ref(0, 0)], -10_000, 10_000)],
        ..State::default()
    }
}

fn level(m: &Mixer) -> i32 {
    m.graph.sub[0]
}

fn trigger(m: &mut Mixer, on: i32) {
    m.set_input(InputKey::object(0, 0, 3, 0), on);
}

#[test]
fn an_event_at_rest_reads_zero() {
    let mut m = mixer(vec![(0, event_state(0, -1000, 0))], &[1]);
    run(&mut m, 5);
    assert_eq!(level(&m), 0);
    assert_eq!(m.element_counts()[1], 1);
}

#[test]
fn shared_event_output_keeps_one_clock_and_new_instances_inherit_its_phase() {
    let state = event_state(0x2000_0000, -1200, 0);
    let mut owner = mixer(vec![(0, state.clone())], &[1]);
    trigger(&mut owner, 32767);
    owner.process(0.1);
    let snapshot = owner.event_output(0, 0, 0).unwrap();
    assert!(snapshot.level < 0);
    for dt in [0.0, 0.001, 0.25, 4.0] {
        let mut local = mixer(vec![(0, state.clone())], &[1]);
        assert!(local.override_event(0, 0, 0, Some(snapshot)));
        local.process(dt);
        assert_eq!(local.event_output(0, 0, 0), Some(snapshot));
        assert_eq!(level(&local), snapshot.level);
        assert!(!local.override_event(0, 0, 99, Some(snapshot)));
        assert!(!local.override_event(0, 0, 0, Some(crate::EventOutput { position: -1, level: 0 })));
        assert!(local.override_event(0, 0, 0, None));
        local.process(1.0);
        assert_eq!(local.event_output(0, 0, 0).unwrap().level, 0);
    }
}

#[test]
fn the_kinds_come_from_the_id() {
    assert_eq!(
        Event { id: SourceId(0xA200_3000), swing: 0, trigger: SourceId(0), params: [0; 3], scales: vec![] }.kind(),
        EnvelopeKind::Atr
    );
}

#[test]
fn an_ar_envelope_attacks_and_releases_on_its_own() {
    let mut m = mixer(vec![(0, event_state(0, -1000, 0))], &[1]);
    trigger(&mut m, 1);
    // 3 frames = 50 ms into a 100 ms attack: halfway down the fade.
    run(&mut m, 3);
    let halfway = level(&m);
    assert!((-560..=-440).contains(&halfway), "{halfway}");
    // After the attack the release walks back; after both it has finished.
    run(&mut m, 4);
    let peak = level(&m);
    assert!(peak < -800, "{peak}");
    trigger(&mut m, 0);
    run(&mut m, 10);
    assert_eq!(level(&m), 0, "the envelope ran out");
}

#[test]
fn an_asr_envelope_holds_for_its_sustain() {
    // 30 frames of sustain = 500 ms.
    let mut m = mixer(vec![(0, event_state(1, -800, 30))], &[1]);
    trigger(&mut m, 1);
    run(&mut m, 12); // past the 6-frame attack
    let held = level(&m);
    assert!(held <= -790, "{held}");
    run(&mut m, 12);
    assert_eq!(level(&m), held, "still sustaining");
    trigger(&mut m, 0);
    run(&mut m, 40);
    assert_eq!(level(&m), 0, "released and over");
}

#[test]
fn an_atr_envelope_holds_while_the_trigger_is_on_and_releases_when_it_drops() {
    let mut m = mixer(vec![(0, event_state(2, -1000, 0))], &[1]);
    trigger(&mut m, 0x7FFF);
    run(&mut m, 20);
    assert!(level(&m) <= -990, "{}", level(&m));
    trigger(&mut m, 0);
    run(&mut m, 3);
    let releasing = level(&m);
    assert!(releasing > -990 && releasing < 0, "{releasing}");
    run(&mut m, 30);
    assert_eq!(level(&m), 0);
}

#[test]
fn a_positive_swing_rises_from_zero() {
    let mut m = mixer(vec![(0, event_state(2, 700, 0))], &[1]);
    trigger(&mut m, 0x7FFF);
    run(&mut m, 20);
    assert!((690..=700).contains(&level(&m)), "{}", level(&m));
}

#[test]
fn a_zero_time_counts_as_one_frame() {
    let mut state = event_state(0, -1000, 0);
    state.events[0].params = [0x8000, 0, 0x8000];
    let mut m = mixer(vec![(0, state)], &[1]);
    trigger(&mut m, 1);
    run(&mut m, 1);
    trigger(&mut m, 0);
    run(&mut m, 3);
    assert_eq!(level(&m), 0);
}

#[test]
fn an_event_scale_multiplies_its_output() {
    // The scale is control 0 (an up-linear curve of controller input 0): 0 at rest, 0x7FFF at full input.
    let mut state = event_state(2, -1000, 0);
    state.controls = vec![control(controller(9, 0, 0, 0), 0xD8F0, vec![])];
    state.events[0].scales = vec![control_ref(0, 0)];
    let mut m = mixer(vec![(0, state)], &[1]);
    trigger(&mut m, 0x7FFF);
    run(&mut m, 20);
    assert_eq!(level(&m), 0, "scaled by a closed control");
    m.set_input(InputKey::controller(0, 0, 0, 0), 0x7FFF);
    run(&mut m, 2);
    assert!((-1002..=-1000).contains(&level(&m)), "an open scale leaves the level: {}", level(&m));
}

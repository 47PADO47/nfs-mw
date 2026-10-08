use super::*;
use crate::{InputKey, db_from_q15, pitch_ratio, q15_from_db};

const FULL_CUT: u32 = 0xD8F0; // swing: bit 15 set, depth -10000

fn input(index: u8) -> InputKey {
    InputKey::controller(0, 0, 0, index)
}

/// One control from controller input 0 (linear down curve, so the input 0 is "loud") into a volume slot.
fn one_control_state(swing: u32) -> State {
    State {
        controls: vec![control(controller(8, 0, 0, 0), swing, vec![])],
        masters: vec![master(1, OutputKind::Volume, vec![control_ref(0, 0)], vec![word(2)])],
        ..State::default()
    }
}

#[test]
fn a_control_cuts_by_its_depth_when_its_curve_is_zero() {
    let mut m = mixer(vec![(0, one_control_state(FULL_CUT))], &[1]);
    // Input 0: the down-linear curve gives 0x7FFF, no cut: only the rounding of dB(0x7FFF) = -1.
    run(&mut m, 1);
    assert_eq!(m.raw_slot(obj(0, 0, 1), 2), Some(q15_from_db(-1) as u16));
    // Full input: the curve gives 0, the full -10000 dB cut, silence.
    m.set_input(input(0), 0x7FFF);
    run(&mut m, 1);
    assert_eq!(m.volume(obj(0, 0, 1), 2), Some(0.0));
}

#[test]
fn a_control_follows_its_curve_between_the_ends() {
    let mut m = mixer(vec![(0, one_control_state(0xFC18))], &[1]); // depth -1000 (-10 dB)
    m.set_input(input(0), 0x7FFF);
    run(&mut m, 1);
    let gain = m.volume(obj(0, 0, 1), 2).unwrap();
    assert!((gain - 10f32.powf(-0.5)).abs() < 0.01, "{gain}");
    m.set_input(input(0), 0x3FFF);
    run(&mut m, 1);
    let half = m.volume(obj(0, 0, 1), 2).unwrap();
    assert!(half > gain && half < 1.0, "{half}");
}

#[test]
fn a_positive_swing_is_an_offset_that_a_pitch_slot_reads_in_cents() {
    let state = State {
        controls: vec![control(controller(9, 0, 0, 0), 1200, vec![])],
        masters: vec![master(1, OutputKind::Pitch, vec![control_ref(0, 0)], vec![word(4)])],
        ..State::default()
    };
    let mut m = mixer(vec![(0, state)], &[1]);
    run(&mut m, 1);
    // Input 0 on an up-linear curve: nothing, the control is at its "off" end: db(0x7FFF)... zero cents.
    let rest = m.pitch(obj(0, 0, 1), 4).unwrap();
    assert!((rest - 1.0).abs() < 0.01, "{rest}");
    m.set_input(input(0), 0x7FFF);
    run(&mut m, 1);
    // Full input: +1200 cents, an octave up.
    let up = m.pitch(obj(0, 0, 1), 4).unwrap();
    assert!((up - 2.0).abs() < 0.05, "{up}");
    assert!((pitch_ratio(1200) - 2.0).abs() < 1e-4);
}

#[test]
fn scales_multiply_the_level() {
    // Control 1 scales control 0 by its curve output: a down-linear curve of input 1.
    let state = State {
        controls: vec![
            control(controller(9, 0, 0, 0), FULL_CUT, vec![control_ref(0, 1)]),
            control(controller(8, 0, 0, 1), FULL_CUT, vec![]),
        ],
        masters: vec![master(1, OutputKind::Volume, vec![control_ref(0, 0)], vec![word(2)])],
        ..State::default()
    };
    let mut m = mixer(vec![(0, state)], &[1]);
    m.set_input(input(0), 0x7FFF);
    m.set_input(input(1), 0); // scale curve 0x7FFF: unity
    run(&mut m, 1);
    let full = m.volume(obj(0, 0, 1), 2).unwrap();
    m.set_input(input(1), 0x4000); // scale about half: the level (dB, negative) halves, so the gain rises
    run(&mut m, 1);
    let scaled = m.volume(obj(0, 0, 1), 2).unwrap();
    assert!(full > 0.9, "{full}");
    assert!((scaled - full).abs() < 0.2, "{scaled} {full}");
}

#[test]
fn sub_channels_add_and_clamp() {
    let state = State {
        controls: vec![
            control(controller(8, 0, 0, 0), 0x0100, vec![]), // +256 at full curve
            control(controller(8, 0, 0, 0), 0x0100, vec![]),
        ],
        subs: vec![sub(vec![control_ref(0, 0), control_ref(0, 1)], -100, 300), sub(vec![sub_ref(0, 0)], -5, 5)],
        masters: vec![master(1, OutputKind::Volume, vec![sub_ref(0, 0)], vec![word(2)])],
        ..State::default()
    };
    let mut m = mixer(vec![(0, state)], &[1]);
    run(&mut m, 1);
    // Both controls read -1 + 256 = 255 each: 510 clamps to the upper limit 300.
    assert_eq!(m.graph.sub, vec![300, 5]);
}

#[test]
fn a_master_adds_its_base_and_converts_the_sum() {
    let mut state = one_control_state(FULL_CUT);
    state.masters[0].base = -600;
    let mut m = mixer(vec![(0, state)], &[1]);
    run(&mut m, 1);
    let level = -600 + db_from_q15(0x7FFF);
    assert_eq!(m.graph.master, vec![level]);
    assert_eq!(m.raw_slot(obj(0, 0, 1), 2), Some(q15_from_db(level) as u16));
}

#[test]
fn an_object_that_is_not_attached_is_silent() {
    let mut m = mixer(vec![(0, one_control_state(FULL_CUT))], &[1]);
    m.attach(obj(0, 0, 1), false);
    run(&mut m, 1);
    assert_eq!(m.graph.master, vec![-10_000]);
    assert_eq!(m.volume(obj(0, 0, 1), 2), Some(0.0));
    m.attach(obj(0, 0, 1), true);
    run(&mut m, 1);
    assert!(m.volume(obj(0, 0, 1), 2).unwrap() > 0.99);
}

#[test]
fn a_filter_output_is_open_at_zero_and_closes_with_the_level() {
    let state = State {
        controls: vec![control(controller(9, 0, 0, 0), 0xFC18, vec![])],
        masters: vec![master(1, OutputKind::Filter, vec![control_ref(0, 0)], vec![word(5)])],
        ..State::default()
    };
    let mut m = mixer(vec![(0, state)], &[1]);
    run(&mut m, 1);
    let closed = m.filter(obj(0, 0, 1), 5).unwrap();
    m.set_input(input(0), 0x7FFF);
    run(&mut m, 1);
    let open = m.filter(obj(0, 0, 1), 5).unwrap();
    assert!(open > 24_000.0 && closed < open, "{closed} {open}");
}

#[test]
fn a_state_with_several_instances_feeds_a_state_that_reads_all_of_them() {
    // State 1 has one control reading controller 1 of its own instance; state 0's sub sums it over all instances.
    let producer = State { controls: vec![control(controller(9, 1, 0, 0), 0x0010, vec![])], ..State::default() };
    let consumer = State { subs: vec![sub(vec![control_ref(1, 0)], -10_000, 10_000)], ..State::default() };
    let mut m = mixer(vec![(0, consumer.clone()), (1, producer.clone())], &[1, 3]);
    for i in 0..3 {
        m.set_input(InputKey::controller(1, i, 0, 0), 0x7FFF);
    }
    run(&mut m, 1);
    // 3 x (+16 - 1), each cut to 14 by the original's (0x7FFF * level) >> 15 scaling of an unscaled control.
    assert_eq!(m.graph.sub, vec![42]);
    let mut m = mixer(vec![(0, consumer), (1, producer)], &[1, 0]);
    run(&mut m, 1);
    assert_eq!(m.graph.sub, vec![0], "no instances of the state, nothing to add");
    assert_eq!(m.element_counts()[0], 0);
}

#[test]
fn inputs_nobody_reads_are_ignored_and_the_ones_set_can_be_read_back() {
    let mut m = mixer(vec![(0, one_control_state(FULL_CUT))], &[1]);
    m.set_input(InputKey::controller(0, 0, 9, 9), 5);
    assert_eq!(m.input(InputKey::controller(0, 0, 9, 9)), 0);
    m.set_input(input(0), 1234);
    assert_eq!(m.input(input(0)), 1234);
}

#[test]
fn equal_inputs_give_equal_outputs_and_odd_numbers_do_not_poison_them() {
    let build = || mixer(vec![(0, one_control_state(FULL_CUT))], &[1]);
    let (mut a, mut b) = (build(), build());
    for (i, v) in [0, 100, 32767, -5, i32::MAX, i32::MIN].into_iter().enumerate() {
        a.set_input(input(0), v);
        b.set_input(input(0), v);
        a.process(1.0 / 60.0);
        b.process(1.0 / 60.0);
        assert_eq!(a.raw_slot(obj(0, 0, 1), 2), b.raw_slot(obj(0, 0, 1), 2), "step {i}");
    }
    a.process(f32::NAN);
    a.process(-1.0);
    a.process(f32::INFINITY);
    assert!(a.volume(obj(0, 0, 1), 2).unwrap().is_finite());
}

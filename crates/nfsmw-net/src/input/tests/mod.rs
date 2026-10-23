mod garbage;

use blackbox_vehicle::InputState;

use super::*;

pub fn code(throttle: u8, steer: u16) -> InputCode {
    InputCode { throttle, steer, ..InputCode::NEUTRAL }
}

fn encoded(frame: &InputFrame) -> Vec<u8> {
    let mut buf = [0u8; MAX_INPUT_PAYLOAD];
    let len = frame.encode(&mut buf).unwrap();
    buf[..len].to_vec()
}

#[test]
fn quantise_maps_ranges_and_cleans_up() {
    let input = InputState {
        throttle: 1.0,
        brake: 0.5,
        steer: -1.0,
        handbrake: 1.0,
        nos: true,
        shift_up: true,
        shift_down: true,
        gear_select: Some(40),
        clutch: f32::NAN,
    };
    let code = InputCode::quantise(&input);
    assert_eq!(code.throttle, PEDAL_MAX);
    assert_eq!(code.brake, 64);
    assert_eq!(code.steer, 0);
    assert_eq!(code.handbrake, HANDBRAKE_MAX);
    assert!(code.nos && !code.shift_up && !code.shift_down);
    assert_eq!(code.gear_select, None);
    assert_eq!(code.clutch, 0);
    assert!(code.is_valid());
    let wild = InputState { throttle: 7.0, steer: 3.0, brake: -2.0, ..InputState::default() };
    let code = InputCode::quantise(&wild);
    assert_eq!((code.throttle, code.steer, code.brake), (PEDAL_MAX, STEER_MAX, 0));
}

#[test]
fn quantisation_error_is_within_half_a_step() {
    for i in 0..=1000 {
        let x = i as f32 / 1000.0;
        let s = x * 2.0 - 1.0;
        let input = InputState { throttle: x, brake: x, steer: s, handbrake: x, clutch: x, ..InputState::default() };
        let back = InputCode::quantise(&input).to_input();
        assert!((back.throttle - x).abs() <= 0.5 / 127.0 + 1e-6);
        assert!((back.steer - s).abs() <= 0.5 / 255.0 + 1e-6);
        assert!((back.handbrake - x).abs() <= 0.5 / 15.0 + 1e-6);
        assert!((back.clutch - x).abs() <= 0.5 / 15.0 + 1e-6);
    }
    assert_eq!(InputCode::NEUTRAL.to_input().steer, 0.0);
    assert_eq!(InputCode::quantise(&InputState { gear_select: Some(3), ..InputState::default() }).gear_select, Some(3));
}

#[test]
fn frames_round_trip_and_use_same() {
    let mut frame = InputFrame::new(0xFFFE, false);
    for code in [code(10, 255), code(10, 255), code(10, 255), code(90, 300)] {
        frame.push(code).unwrap();
    }
    let bytes = encoded(&frame);
    // 24 header bits, two full inputs of 31 bits and two `same` bits, padded to the byte.
    assert_eq!(bytes.len(), (24 + 31 + 1 + 1 + 31_usize).div_ceil(8));
    assert_eq!(InputFrame::decode(&bytes).unwrap(), frame);
}

#[test]
fn clutch_and_gear_travel_only_when_flagged() {
    let mut with_clutch = InputFrame::new(5, true);
    with_clutch.push(InputCode { clutch: 9, gear_select: Some(2), ..code(1, 2) }).unwrap();
    with_clutch.push(InputCode { clutch: 3, ..code(1, 2) }).unwrap();
    let back = InputFrame::decode(&encoded(&with_clutch)).unwrap();
    assert_eq!(back, with_clutch);
    assert_eq!(back.inputs()[0].clutch, 9);
    assert_eq!(back.inputs()[0].gear_select, Some(2));
    let mut no_clutch = InputFrame::new(5, false);
    no_clutch.push(InputCode { clutch: 9, ..code(1, 2) }).unwrap();
    assert_eq!(no_clutch.inputs()[0].clutch, 0);
    assert_eq!(encoded(&no_clutch).len(), (24 + 31_usize).div_ceil(8));
}

#[test]
fn eight_inputs_at_most() {
    let mut frame = InputFrame::new(0, true);
    for i in 0..MAX_FRAME_TICKS {
        frame.push(InputCode { gear_select: Some(i as u8), clutch: 15, ..code(i as u8, 510) }).unwrap();
    }
    assert_eq!(frame.push(InputCode::NEUTRAL), Err(InputError::Full));
    let bytes = encoded(&frame);
    assert_eq!(bytes.len(), MAX_INPUT_PAYLOAD);
    assert_eq!(InputFrame::decode(&bytes).unwrap(), frame);
    assert_eq!(InputFrame::new(0, false).encode(&mut [0; 8]), Err(InputError::Empty));
    assert_eq!(frame.encode(&mut [0; 8]), Err(InputError::BufferTooSmall));
    assert_eq!(frame.push(InputCode { steer: 511, ..InputCode::NEUTRAL }), Err(InputError::Full));
    assert_eq!(
        InputFrame::new(0, false).push(InputCode { steer: 511, ..InputCode::NEUTRAL }),
        Err(InputError::BadValue)
    );
}

#[test]
fn ticks_expand_around_the_reference() {
    assert_eq!(expand_tick(10, 100), 10);
    assert_eq!(expand_tick(0xFFFF, 0x1_0002), 0xFFFF);
    assert_eq!(expand_tick(3, 0x1_FFFE), 0x2_0003);
    assert_eq!(expand_tick(0x8000, 0), 0xFFFF_8000);
    assert_eq!(expand_tick(5, 0), 5);
}

#[test]
fn history_frames_repeat_unacknowledged_ticks() {
    let mut history = InputHistory::new(false);
    for tick in 100..120u32 {
        history.record(tick, code(tick as u8, 255));
    }
    let frame = history.frame(None, 8).unwrap();
    assert_eq!(frame.first_tick(), 112);
    assert_eq!(frame.len(), 8);
    let frame = history.frame(Some(116), 8).unwrap();
    assert_eq!((frame.first_tick(), frame.len()), (117, 3));
    assert!(history.frame(Some(119), 8).is_none());
    assert_eq!(history.get(105), Some(code(105, 255)));
    history.record(105 + HISTORY_TICKS as u32, code(1, 1));
    assert_eq!(history.get(105), None);
}

#[test]
fn history_frames_stop_at_a_gap() {
    let mut history = InputHistory::new(false);
    for tick in [10u32, 11, 13, 14] {
        history.record(tick, InputCode::NEUTRAL);
    }
    let frame = history.frame(None, 8).unwrap();
    assert_eq!((frame.first_tick(), frame.len()), (13, 2));
}

#[test]
fn window_sorts_inputs_into_fresh_late_early_and_duplicate() {
    let mut window = InputWindow::new(100);
    let mut frame = InputFrame::new(98, false);
    for i in 0..5 {
        frame.push(code(i, 255)).unwrap();
    }
    let accepted = window.accept(&frame, 100);
    assert_eq!(accepted, Accepted { fresh: 3, duplicate: 0, late: 2, early: 0 });
    assert_eq!(window.accept(&frame, 100), Accepted { fresh: 0, duplicate: 3, late: 2, early: 0 });
    let mut ahead = InputFrame::new(100 + MAX_AHEAD_TICKS as u16, false);
    ahead.push(InputCode::NEUTRAL).unwrap();
    ahead.push(code(1, 1)).unwrap();
    assert_eq!(window.accept(&ahead, 100), Accepted { fresh: 1, duplicate: 0, late: 0, early: 1 });
}

#[test]
fn window_holds_then_falls_back() {
    let mut window = InputWindow::new(0);
    let mut frame = InputFrame::new(0, false);
    frame.push(InputCode { shift_up: true, ..code(100, 300) }).unwrap();
    window.accept(&frame, 0);
    let (got, source) = window.input_for(0);
    assert_eq!(source, InputSource::Received);
    assert!(got.shift_up);
    for tick in 1..=HOLD_TICKS {
        let (held, source) = window.input_for(tick);
        assert_eq!(source, InputSource::Held);
        assert_eq!(held, InputCode { shift_up: false, ..got });
    }
    let (fallback, source) = window.input_for(HOLD_TICKS + 1);
    assert_eq!(source, InputSource::Fallback);
    assert_eq!(fallback.throttle, 0);
    assert!(fallback.brake > 0);
    assert_eq!(window.accept(&frame, HOLD_TICKS + 2).late, 1);
    assert_eq!(window.next_tick(), HOLD_TICKS + 2);
}

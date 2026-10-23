use blackbox_vehicle::InputState;

use super::*;
use crate::input::{FRAME_INPUT, InputCode, InputFrame};
use crate::message::StartAt;

/// A synced, placed session: one full snapshot with the own block.
fn placed() -> Harness {
    let mut h = Harness::synced();
    let tick = server_tick(h.now) as Tick;
    let own = car_view(ENTITY, Some(SLOT), 0.0);
    let body = body(&h.schema, tick, 0, &[Entry::Full(&own)]);
    h.snapshot(tick, tick, Some(&own_block(0.0)), &body);
    h.frame();
    h.answer_pings();
    assert!(h.session.own_pose().is_some());
    h
}

fn input_frames(sent: Vec<Sent>) -> Vec<InputFrame> {
    sent.into_iter()
        .filter_map(|sent| match sent {
            Sent::Unreliable { frame_type: FRAME_INPUT, bytes, .. } => Some(InputFrame::decode(&bytes).unwrap()),
            _ => None,
        })
        .collect()
}

#[test]
fn driving_steps_the_car_and_sends_redundant_input_frames() {
    let mut h = placed();
    let throttle = InputState { throttle: 1.0, ..InputState::default() };
    let mut frames = Vec::new();
    for _ in 0..30 {
        h.frame();
        h.session.drive(|_| throttle);
        frames.extend(input_frames(h.answer_pings()));
    }
    assert!(frames.len() >= 25, "one INPUT frame per frame that stepped, got {}", frames.len());
    assert!(frames.iter().any(|frame| frame.len() > 1), "unacknowledged inputs repeat");
    assert!(frames.iter().all(|frame| frame.len() <= 8));
    let pose = h.session.own_pose().unwrap();
    assert!(pose.position[2] > 0.1, "the car drove forward: {pose:?}");
    assert_eq!(h.session.counters().inputs_sent, frames.len() as u64);
}

#[test]
fn input_before_the_start_tick_is_neutral() {
    let mut h = placed();
    let start = server_tick(h.now) as u32 + 600;
    h.reliable(StartAt { start_tick: start }.into());
    h.frame();
    let throttle = InputState { throttle: 1.0, ..InputState::default() };
    let mut frames = Vec::new();
    for _ in 0..10 {
        h.frame();
        h.session.drive(|_| throttle);
        frames.extend(input_frames(h.answer_pings()));
    }
    assert!(!frames.is_empty());
    for frame in &frames {
        assert!(frame.inputs().iter().all(|code| *code == InputCode::NEUTRAL));
    }
}

#[test]
fn only_the_next_due_tick_is_taken() {
    let mut h = placed();
    h.frame();
    let due = h.session.due_ticks().expect("ticks are due");
    let first = *due.start();
    assert!(!h.session.submit_input(first + 1, &InputState::default()), "a tick ahead is refused");
    assert!(h.session.submit_input(first, &InputState::default()));
    assert!(!h.session.submit_input(first, &InputState::default()), "a tick already stepped is refused");
}

#[test]
fn nothing_is_due_before_the_first_own_block() {
    let mut h = Harness::synced();
    assert_eq!(h.session.due_ticks(), None);
    assert!(!h.session.submit_input(700, &InputState::default()));
}

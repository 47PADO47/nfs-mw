//! Pad input: messages for the current button and the package, repeat times and navigation.

use super::*;
use crate::hash::fe_hash_upper;
use crate::ids::*;
use crate::package::flags;
use crate::package::synth::*;

const TO_GAME: u32 = 0xFFFF_FFFF;
const FRAME: f32 = 1.0 / 60.0;

fn button(guid: u32, x: f32, extra_flags: u32) -> Obj {
    let mut o = Obj::image(guid, fe_hash_upper(&format!("BTN{guid}")));
    o.position = [x, 0.0, 10.0];
    o.flags = flags::IS_BUTTON | extra_flags;
    o
}

/// Three buttons in a row (GUIDs 1, 2, 3), the package forwards the directions, accept and back to the game.
fn row(extra: &[(usize, u32)]) -> (Runtime, PackageId) {
    let mut objects = vec![button(1, -70.0, 0), button(2, 0.0, 0), button(3, 70.0, 0)];
    for &(i, f) in extra {
        objects[i].flags |= f;
    }
    // Button 2 answers BUTTON_PRESSED with message 0x1234 for the game, and BUTTON_RELEASED with 0x1235.
    objects[1].responses =
        responses(&[(BUTTON_PRESSED, vec![(2, 0x1234, TO_GAME)]), (BUTTON_RELEASED, vec![(2, 0x1235, TO_GAME)])]);
    let package_responses: Vec<(u32, Vec<Resp>)> =
        [PAD_LEFT, PAD_RIGHT, PAD_UP, PAD_ACCEPT, PAD_BACK].iter().map(|&m| (m, vec![(2, m, TO_GAME)])).collect();
    let bytes = package("Row.fng", &[], &objects, &responses(&package_responses), &[]);
    let mut rt = Runtime::new();
    let id = rt.load(Package::parse(&bytes).unwrap());
    rt.set_focus(id, 1);
    rt.update(FRAME);
    rt.take_outgoing();
    (rt, id)
}

fn step(rt: &mut Runtime, mask: u32, frames: usize) {
    for _ in 0..frames {
        rt.set_pad_mask(mask);
        rt.update(FRAME);
    }
}

fn game_messages(rt: &mut Runtime) -> Vec<u32> {
    rt.take_outgoing()
        .into_iter()
        .filter_map(|o| if let Outgoing::Game { message, .. } = o { Some(message) } else { None })
        .collect()
}

#[test]
fn a_direction_moves_the_focus_and_tells_the_game() {
    let (mut rt, id) = row(&[]);
    step(&mut rt, pad::RIGHT, 1);
    assert_eq!(rt.focus(id), Some(2));
    assert_eq!(game_messages(&mut rt), vec![PAD_RIGHT]);
    step(&mut rt, 0, 1);
    step(&mut rt, pad::RIGHT, 1);
    assert_eq!(rt.focus(id), Some(3));
    step(&mut rt, 0, 1);
    step(&mut rt, pad::RIGHT, 1);
    assert_eq!(rt.focus(id), Some(3), "nothing further right");
    step(&mut rt, 0, 1);
    step(&mut rt, pad::LEFT, 1);
    assert_eq!(rt.focus(id), Some(2));
}

#[test]
fn a_held_direction_repeats_after_20_frames_and_then_fast() {
    let (mut rt, _) = row(&[]);
    step(&mut rt, pad::UP, 1);
    assert_eq!(game_messages(&mut rt).len(), 1, "the press");
    // 16 ticks per frame: the held count is 320 on the 21st frame.
    step(&mut rt, pad::UP, 19);
    assert!(game_messages(&mut rt).is_empty(), "not yet");
    step(&mut rt, pad::UP, 1);
    assert_eq!(game_messages(&mut rt).len(), 1, "first repeat");
    step(&mut rt, pad::UP, 7);
    assert!(game_messages(&mut rt).is_empty(), "7 frames are 112 ticks");
    step(&mut rt, pad::UP, 1);
    assert_eq!(game_messages(&mut rt).len(), 1, "then every 120 ticks or more");
    step(&mut rt, 0, 1);
    step(&mut rt, pad::UP, 1);
    assert_eq!(game_messages(&mut rt).len(), 1, "a new press starts over");
    step(&mut rt, pad::UP, 19);
    assert!(game_messages(&mut rt).is_empty(), "with the slow first repeat again");
}

#[test]
fn accept_goes_to_the_current_buttons_response_or_else_the_package() {
    let (mut rt, id) = row(&[]);
    // Button 1 has no BUTTON_PRESSED response: the package's PAD_ACCEPT response answers.
    step(&mut rt, pad::ACCEPT, 1);
    assert_eq!(game_messages(&mut rt), vec![PAD_ACCEPT]);
    step(&mut rt, 0, 1);
    rt.set_focus(id, 2);
    rt.update(FRAME);
    rt.take_outgoing();
    step(&mut rt, pad::ACCEPT, 1);
    assert_eq!(game_messages(&mut rt), vec![0x1234], "button 2 answers BUTTON_PRESSED");
    step(&mut rt, 0, 1);
    assert_eq!(game_messages(&mut rt), vec![0x1235], "and BUTTON_RELEASED when accept is let go");
}

#[test]
fn a_button_that_does_not_navigate_keeps_the_focus() {
    let (mut rt, id) = row(&[(0, flags::DONT_NAVIGATE)]);
    step(&mut rt, pad::RIGHT, 1);
    assert_eq!(rt.focus(id), Some(1));
    assert_eq!(game_messages(&mut rt), vec![PAD_RIGHT], "the package still hears it");
}

#[test]
fn a_package_without_control_hears_nothing() {
    let (mut rt, id) = row(&[]);
    rt.set_control(id, false);
    step(&mut rt, pad::RIGHT | pad::ACCEPT, 2);
    assert_eq!(rt.focus(id), Some(1));
    assert!(game_messages(&mut rt).is_empty());
    rt.set_control(id, true);
    step(&mut rt, 0, 1);
    step(&mut rt, pad::BACK, 1);
    assert_eq!(game_messages(&mut rt), vec![PAD_BACK]);
}

#[test]
fn start_can_stand_in_for_accept() {
    let (mut rt, id) = row(&[]);
    step(&mut rt, pad::START, 1);
    assert!(game_messages(&mut rt).is_empty(), "no package response for start");
    step(&mut rt, 0, 1);
    rt.set_start_equals_accept(id, true);
    step(&mut rt, pad::START, 1);
    assert_eq!(game_messages(&mut rt), vec![PAD_ACCEPT]);
}

#[test]
fn input_can_be_switched_off_by_a_package_response() {
    let (mut rt, id) = row(&[]);
    // Disabling input stops the frame's processing for the package.
    rt.running_mut(id).unwrap().input_enabled = false;
    step(&mut rt, pad::RIGHT, 1);
    assert_eq!(rt.focus(id), Some(1));
}

//! The mod message's timing and its one slot on screen.

use super::message::{ModMessage, Tone, advance, current, hide, line, set};

#[test]
fn a_message_counts_down_and_runs_out() {
    let mut message = ModMessage::new("hello", Tone::Info, 2.0);
    assert!(!message.tick(1.5), "half a second is left");
    assert!(message.tick(0.5), "the time has run out");
    assert!(message.tick(-1.0), "a negative step does not add time");
}

#[test]
fn a_zero_or_negative_time_is_already_over() {
    assert!(ModMessage::new("x", Tone::Warning, 0.0).clone().tick(0.0));
    assert!(ModMessage::new("x", Tone::Warning, -3.0).clone().tick(0.0));
}

#[test]
fn a_new_message_replaces_the_one_on_screen_and_hide_takes_it_down() {
    set("first", Tone::Info, 1.0);
    set("second", Tone::Big, 3.0);
    let shown = current().expect("a message is up");
    assert_eq!((shown.text.as_str(), shown.tone), ("second", Tone::Big));
    hide();
    assert!(current().is_none());
}

#[test]
fn a_hud_that_draws_the_message_reads_its_line_and_counts_its_time() {
    set("drift", Tone::Warning, 1.0);
    assert_eq!(line(), Some(("drift".to_owned(), Tone::Warning)));
    advance(0.5);
    assert!(line().is_some(), "half its time is left");
    advance(0.5);
    assert_eq!(line(), None, "advanced past its time, it is taken down");
}

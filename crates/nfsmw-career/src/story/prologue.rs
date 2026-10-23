//! The prologue: the six-race chain that opens a career, and the resume point of a save in it
//! (`docs/specs/story-flow.md` sections 2.1, 2.3 and 2.7; `docs/specs/career-scripts-events.md` section 2).
//!
//! The races name the next one through their `PostRaceActivity`, so the chain is a fixed order. Every prologue race
//! forces the preset car `M3GTRCAREERSTART`; the game crate sets it when it starts the race, so it is not decided here.
//!
//! The resume rule uses the first unwon event of the resume order in section 2.7, which is the order
//! `safehouse::resume` uses. That order has five events and leaves out `16.8.1`, the NIS shell, so this module keeps
//! that table. A save does not store the current prologue node, only the node bits, so the first unwon event is the
//! only event a load can find.

use crate::safehouse::resume::{PROLOGUE_END_EVENT, PROLOGUE_EVENTS};
use crate::settings::{NodeStates, PROLOGUE_STAGE};

/// The six events of the chain, in the order the player plays them (section 2.1).
pub const PROLOGUE_CHAIN: [&str; 6] = ["16.1.0", "16.2.2", "16.2.3", "16.1.1", "16.8.1", "16.2.1"];

/// The first event of the chain: the opening race, whose car gives out a few seconds in.
pub const OPENING_EVENT: &str = "16.1.0";

/// The title card shown before a prologue flashback race that the install gives no message of its own: "Six Days Ago..."
/// before `16.2.2`, "Four Days Ago..." before `16.2.3`. `16.1.1` carries its own message (2, "Two Days Ago..."). The
/// chain counts down to the present, and the install holds the three cards (`story-flow.md` section 2.1). `None` for
/// the other events.
pub fn flashback_message(event: &str) -> Option<i32> {
    match event {
        "16.2.2" => Some(6),
        "16.2.3" => Some(4),
        _ => None,
    }
}

/// The language label of the title card of message `id`: the days-ago cards of the flashbacks, or `None` for an id that
/// has no card text. The text holds `^` for a line break.
pub fn message_card_label(id: i32) -> Option<u32> {
    match id {
        2 => Some(0xA461_D64A),
        4 => Some(0x2117_EECC),
        6 => Some(0x4CBB_A984),
        _ => None,
    }
}

/// What runs after a prologue event is done.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PrologueNext {
    /// The next prologue event, started through the done event's `PostRaceActivity`.
    Event(&'static str),
    /// After `16.2.1`: `JumpToCarLot` opens the car lot (section 2.3, rule 3).
    CarLot,
}

/// What a save in the prologue resumes at (section 2.7).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PrologueResume {
    /// The prologue restarts at this event: the first event of the resume order that is not won in the career.
    StartAt(&'static str),
    /// Nothing to restart: the save is not in the prologue, `16.2.1` is won, or the debug byte skips the check. The
    /// save loads through the normal rules, which end at the safe house menu.
    Done,
}

/// The step after a prologue event, or `None` when the event is not in the chain (section 2.3, rules 2 and 3).
///
/// `16.2.1` is the last event of the chain, so it leads to the car lot.
pub fn next_after(event: &str) -> Option<PrologueNext> {
    let index = PROLOGUE_CHAIN.iter().position(|id| *id == event)?;
    match PROLOGUE_CHAIN.get(index + 1) {
        Some(next) => Some(PrologueNext::Event(next)),
        None => Some(PrologueNext::CarLot),
    }
}

/// Where a save loads (section 2.7, the save-load table).
///
/// Only stage 16 with `16.2.1` not won restarts the prologue. The restart event is the first event of
/// [`PROLOGUE_EVENTS`] that is not won in the career (the win is the `0x2` bit of the node, section 2.7 and
/// `docs/formats/career-save.md`). The debug byte skips the whole check.
pub fn resume_point(stage: u8, nodes: &NodeStates, debug_skip_prologue: bool) -> PrologueResume {
    let won = |event: &str| nodes.get(event).is_some_and(|state| state.won_in_career());
    if stage != PROLOGUE_STAGE || debug_skip_prologue || won(PROLOGUE_END_EVENT) {
        return PrologueResume::Done;
    }
    let event = PROLOGUE_EVENTS.iter().copied().find(|event| !won(event)).unwrap_or(PROLOGUE_END_EVENT);
    PrologueResume::StartAt(event)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A career node list with the given events won in the career.
    fn won(events: &[&str]) -> NodeStates {
        let mut nodes = NodeStates::default();
        for event in events {
            nodes.set_won_in_career(event).expect("the list has room");
        }
        nodes
    }

    #[test]
    fn each_prologue_event_names_the_next_one() {
        let table = [
            ("16.1.0", PrologueNext::Event("16.2.2")),
            ("16.2.2", PrologueNext::Event("16.2.3")),
            ("16.2.3", PrologueNext::Event("16.1.1")),
            ("16.1.1", PrologueNext::Event("16.8.1")),
            ("16.8.1", PrologueNext::Event("16.2.1")),
            ("16.2.1", PrologueNext::CarLot),
        ];
        for (event, expected) in table {
            assert_eq!(next_after(event), Some(expected), "after {event}");
        }
    }

    #[test]
    fn the_flashbacks_count_down_six_four_two_days() {
        assert_eq!(flashback_message("16.2.2"), Some(6));
        assert_eq!(flashback_message("16.2.3"), Some(4));
        assert_eq!(flashback_message("16.1.0"), None);
        assert_eq!(flashback_message("16.2.1"), None);
        for id in [6, 4, 2] {
            assert!(message_card_label(id).is_some(), "card {id}");
        }
        assert_eq!(message_card_label(0), None);
    }

    #[test]
    fn an_event_outside_the_chain_has_no_next() {
        assert_eq!(next_after("1.2.3"), None);
        assert_eq!(next_after(""), None);
        assert_eq!(next_after("16.2.4"), None);
    }

    #[test]
    fn walking_the_chain_from_the_start_reaches_the_car_lot() {
        let mut event = PROLOGUE_CHAIN[0];
        let mut walked = vec![event];
        while let Some(PrologueNext::Event(next)) = next_after(event) {
            walked.push(next);
            event = next;
        }
        assert_eq!(walked, PROLOGUE_CHAIN.to_vec());
        assert_eq!(next_after(event), Some(PrologueNext::CarLot));
    }

    #[test]
    fn a_fresh_career_starts_at_the_first_race() {
        assert_eq!(resume_point(16, &won(&[]), false), PrologueResume::StartAt("16.1.0"));
    }

    #[test]
    fn the_first_unwon_event_of_the_resume_order_is_the_restart_point() {
        let table: [(&[&str], &str); 4] = [
            (&["16.1.0"], "16.2.2"),
            (&["16.1.0", "16.2.2"], "16.2.3"),
            (&["16.1.0", "16.2.2", "16.2.3"], "16.1.1"),
            (&["16.1.0", "16.2.2", "16.2.3", "16.1.1"], "16.2.1"),
        ];
        for (done, expected) in table {
            assert_eq!(resume_point(16, &won(done), false), PrologueResume::StartAt(expected), "won {done:?}");
        }
    }

    #[test]
    fn the_resume_order_wins_over_the_order_of_the_wins() {
        assert_eq!(resume_point(16, &won(&["16.2.3", "16.1.1"]), false), PrologueResume::StartAt("16.1.0"));
        assert_eq!(resume_point(16, &won(&["16.1.0", "16.2.3"]), false), PrologueResume::StartAt("16.2.2"));
    }

    #[test]
    fn the_shell_16_8_1_is_not_in_the_resume_order() {
        let nodes = won(&["16.1.0", "16.2.2", "16.2.3", "16.1.1"]);
        assert_eq!(resume_point(16, &nodes, false), PrologueResume::StartAt("16.2.1"));
    }

    #[test]
    fn a_won_16_2_1_ends_the_prologue_whatever_else_is_unwon() {
        assert_eq!(resume_point(16, &won(&["16.2.1"]), false), PrologueResume::Done);
    }

    #[test]
    fn an_unlocked_but_unwon_16_2_1_still_restarts_the_prologue() {
        let mut nodes = won(&["16.1.0", "16.2.2", "16.2.3", "16.1.1"]);
        nodes.unlock("16.2.1", false).expect("the list has room");
        assert_eq!(resume_point(16, &nodes, false), PrologueResume::StartAt("16.2.1"));
    }

    #[test]
    fn another_stage_or_the_debug_byte_is_not_a_prologue_resume() {
        assert_eq!(resume_point(15, &won(&[]), false), PrologueResume::Done);
        assert_eq!(resume_point(1, &won(&[]), false), PrologueResume::Done);
        assert_eq!(resume_point(16, &won(&[]), true), PrologueResume::Done);
    }
}

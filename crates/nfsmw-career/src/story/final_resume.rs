//! The resume rule of the final race: a save whose race `1.2.3` is won in the career, and whose career is not completed,
//! loads straight into the final race `1.8.1` (`docs/specs/story-flow.md` section 2.7, `docs/specs/career-scripts-events.md`
//! section 4, `docs/specs/story-flow-2.md` section 5). The game crate starts the race; this module decides it.

use crate::settings::{Flags, NodeStates};

/// The race whose win opens the final race on a load: the last race of bin 1 before it (section 2.7).
pub const RESUME_BEFORE_FINAL_EVENT: &str = "1.2.3";

/// Whether a save loads into the final race. Race `1.2.3` must be won in the career (the `0x2` bit of its node) and the
/// career-completed flag (`0x4000`) must be clear: a completed career goes to the safe house as before.
pub fn resume_final_race(nodes: &NodeStates, flags: Flags) -> bool {
    let won = nodes.get(RESUME_BEFORE_FINAL_EVENT).is_some_and(|state| state.won_in_career());
    won && !flags.contains(Flags::CAREER_COMPLETE)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A node list with the given events won in the career.
    fn won(events: &[&str]) -> NodeStates {
        let mut nodes = NodeStates::default();
        for event in events {
            nodes.set_won_in_career(event).expect("the list has room");
        }
        nodes
    }

    #[test]
    fn a_won_1_2_3_loads_the_final_race() {
        assert!(resume_final_race(&won(&["1.2.3"]), Flags::default()));
    }

    #[test]
    fn an_unwon_1_2_3_does_not() {
        assert!(!resume_final_race(&won(&[]), Flags::default()));
        let mut nodes = NodeStates::default();
        nodes.unlock("1.2.3", false).expect("the list has room");
        assert!(!resume_final_race(&nodes, Flags::default()), "unlocked is not won");
    }

    #[test]
    fn a_completed_career_does_not_load_the_final_race_again() {
        assert!(!resume_final_race(&won(&["1.2.3"]), Flags::CAREER_COMPLETE));
    }
}

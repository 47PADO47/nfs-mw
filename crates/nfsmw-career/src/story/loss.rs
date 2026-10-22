//! What a lost career race leads to (`docs/specs/race-loss-retry.md`). In the original a loss never reaches the race's
//! `done` state: the race waits in `raceover` for a restart or an abandon, so the chain stops and the story does not go on
//! (`story-flow.md` section 1.1, `story-flow-2.md` F-08, F-09). The rewrite shows the results first and then asks for the
//! retry, so the player cannot walk out of a race the story needs won.

/// The scripted chase (16.2.1).
const CHASE_EVENT: &str = "16.2.1";
/// The NIS shell race (16.8.1): it has no racing, and the game counts it as won (`story-flow.md` section 2.1).
const SHELL_EVENT: &str = "16.8.1";

/// What the results screen does when the player leaves it after a race that was not won.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum LossRule {
    /// Nothing is asked: the race was won, it is not a career race, or the story scripts its ending.
    #[default]
    Leave,
    /// A career race the story waits on, with no race chained after it: the player is asked to retry and may decline.
    Offer,
    /// A career race that chains on to the next one when it is won (the prologue, the boss approaches): the only way on is a
    /// retry.
    Require,
}

/// Whether the story ends `event` itself, so that not winning it is part of the script: the scripted chase 16.2.1 ends on its
/// `killrace` timer (section 2.4) and the NIS shell 16.8.1 has no racing at all.
pub fn ends_by_script(event: &str) -> bool {
    event == CHASE_EVENT || event == SHELL_EVENT
}

/// The rule for a race the player has finished. `career` is whether it is a career race, `won` whether the player placed first
/// with a time, `event` its career event id and `chained` whether it names a `PostRaceActivity`.
pub fn loss_rule(career: bool, won: bool, event: &str, chained: bool) -> LossRule {
    if !career || won || ends_by_script(event) {
        return LossRule::Leave;
    }
    if chained {
        return LossRule::Require;
    }
    LossRule::Offer
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_win_or_a_free_race_asks_nothing() {
        assert_eq!(loss_rule(true, true, "16.1.0", true), LossRule::Leave);
        assert_eq!(loss_rule(false, false, "16.1.0", true), LossRule::Leave);
    }

    #[test]
    fn a_lost_prologue_race_must_be_run_again() {
        for event in ["16.1.0", "16.2.2", "16.2.3", "16.1.1"] {
            assert_eq!(loss_rule(true, false, event, true), LossRule::Require, "{event}");
        }
    }

    #[test]
    fn a_lost_career_race_with_no_chain_offers_the_retry() {
        assert_eq!(loss_rule(true, false, "10.2.1", false), LossRule::Offer);
    }

    #[test]
    fn the_scripted_endings_are_not_retried() {
        assert_eq!(loss_rule(true, false, "16.2.1", true), LossRule::Leave);
        assert_eq!(loss_rule(true, false, "16.8.1", true), LossRule::Leave);
    }
}

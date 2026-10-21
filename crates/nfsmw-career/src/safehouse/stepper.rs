//! The Blacklist sequence stepper (`docs/specs/career-safehouse.md` section 5.3, with the corrected order; the older
//! text of `career-flow.md` 7.3 put the autosave condition on the wrong step).
//!
//! The stepper walks a counter that starts at -1 and adds 1 on every call, then opens one screen per call. Steps
//! whose condition fails are skipped inside the same call.

use super::context::FrontEndContext;
use crate::settings::{CareerSettings, Flags};

/// What a call of the stepper asks the front end to do.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StepAction {
    /// Step 0: `SafeHouseRivalChallenge.fng`.
    RivalChallenge,
    /// Step 1: `SafeHouseMarkers.fng`, the marker choice.
    Markers,
    /// Step 2: `SafeHouseRegionUnlock.fng` (only at stage 8 and 12).
    RegionUnlock,
    /// Step 3: the memory-card autosave flow (`MC_Main.fng`), with the rival bio stored as the screen to open when
    /// it ends. Only when autosave is possible.
    AutosaveFlow,
    /// Step 4: `SafeHouseRivalBio.fng`.
    RivalBio,
    /// Step 5: the rival movie `blacklist_<NN>` in `FEAnyMovie.fng`.
    RivalMovie,
    /// Step 6 at stage 15: context bit 17 is cleared, bit 0 set, settings flag bit 5 set (applied by
    /// [`BlacklistStepper::advance`]), `SafehouseReputationOverview.fng` opens and the sequence ends.
    ReputationOverview,
    /// Step 6 at stage 13 with the rap sheet flag clear: the flag is set (applied), the counter becomes 5 and the
    /// rap sheet movie plays; when it ends the stepper is called again and step 6 runs once more.
    RapSheetMovie,
    /// Step 6 at any other stage, or at stage 13 with the flag already set: leave the front end (the same leave as
    /// Resume Free Roam); context bit 17 is still set when it runs.
    LeaveFrontEnd,
    /// Step 7 (and any later call): broadcast on the gameplay subject, create a type-0 event, clear context bit 17
    /// and end; the player is in the safe house menu.
    End,
}

/// The stepper counter.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BlacklistStepper {
    /// The counter; -1 when the front end is created.
    pub counter: i32,
}

impl Default for BlacklistStepper {
    fn default() -> Self {
        BlacklistStepper::new()
    }
}

impl BlacklistStepper {
    /// A stepper at its start value, -1.
    pub const fn new() -> BlacklistStepper {
        BlacklistStepper { counter: -1 }
    }

    /// The next action. `autosave_possible` is the result of the gate with `force = false` ([`crate::savefile::AutosaveManager::can_autosave`]; it consumes the skip-once byte, so evaluate it once). The two
    /// effects of step 6 are applied to `settings` and `context` here.
    ///
    /// Where the stepper resumes after the first-visit bio is **unconfirmed** in the spec (the navigation code that
    /// advances the counter was not followed); this function only implements the counter rule.
    pub fn advance(
        &mut self,
        settings: &mut CareerSettings,
        context: &mut FrontEndContext,
        autosave_possible: bool,
    ) -> StepAction {
        loop {
            self.counter = self.counter.saturating_add(1);
            match self.counter {
                0 => return StepAction::RivalChallenge,
                1 => return StepAction::Markers,
                2 if matches!(settings.stage, 8 | 12) => return StepAction::RegionUnlock,
                3 if autosave_possible => return StepAction::AutosaveFlow,
                4 => return StepAction::RivalBio,
                5 => return StepAction::RivalMovie,
                6 => return self.step_six(settings, context),
                2 | 3 => continue,
                _ => {
                    context.remove(FrontEndContext::BLACKLIST_SEQUENCE);
                    return StepAction::End;
                }
            }
        }
    }

    fn step_six(&mut self, settings: &mut CareerSettings, context: &mut FrontEndContext) -> StepAction {
        if settings.stage == 15 {
            context.remove(FrontEndContext::BLACKLIST_SEQUENCE);
            context.insert(FrontEndContext::CAREER);
            settings.flags.insert(Flags::OPENING_SEEN);
            return StepAction::ReputationOverview;
        }
        if settings.stage == 13 && !settings.flags.contains(Flags::HAS_RAP_SHEET) {
            settings.flags.insert(Flags::HAS_RAP_SHEET);
            self.counter = 5;
            return StepAction::RapSheetMovie;
        }
        StepAction::LeaveFrontEnd
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run(stage: u8, autosave: bool, calls: usize) -> Vec<StepAction> {
        let mut settings = CareerSettings::new_career();
        settings.stage = stage;
        let mut ctx = FrontEndContext(FrontEndContext::CAREER.0 | FrontEndContext::BLACKLIST_SEQUENCE.0);
        let mut stepper = BlacklistStepper::new();
        (0..calls).map(|_| stepper.advance(&mut settings, &mut ctx, autosave)).collect()
    }

    use StepAction::*;

    #[test]
    fn the_counter_starts_at_minus_one() {
        assert_eq!(BlacklistStepper::new().counter, -1);
    }

    #[test]
    fn an_ordinary_stage_without_autosave_skips_two_steps() {
        assert_eq!(run(14, false, 6), [RivalChallenge, Markers, RivalBio, RivalMovie, LeaveFrontEnd, End]);
    }

    #[test]
    fn autosave_inserts_the_memory_card_flow_before_the_bio() {
        assert_eq!(run(14, true, 5), [RivalChallenge, Markers, AutosaveFlow, RivalBio, RivalMovie]);
    }

    #[test]
    fn the_region_unlock_is_only_for_stages_8_and_12() {
        for stage in [8, 12] {
            assert_eq!(run(stage, false, 3), [RivalChallenge, Markers, RegionUnlock]);
        }
        for stage in [7, 9, 11, 13, 15, 1] {
            assert_eq!(run(stage, false, 3), [RivalChallenge, Markers, RivalBio]);
        }
        assert_eq!(run(8, true, 5)[2..], [RegionUnlock, AutosaveFlow, RivalBio]);
    }

    #[test]
    fn stage_15_ends_with_the_reputation_overview_and_sets_the_opening_seen_flag() {
        let mut settings = CareerSettings::new_career();
        settings.stage = 15;
        let mut ctx = FrontEndContext(FrontEndContext::BLACKLIST_SEQUENCE.0);
        let mut stepper = BlacklistStepper { counter: 5 };
        assert_eq!(stepper.advance(&mut settings, &mut ctx, false), ReputationOverview);
        assert!(settings.flags.contains(Flags::OPENING_SEEN));
        assert!(!ctx.contains(FrontEndContext::BLACKLIST_SEQUENCE));
        assert!(ctx.contains(FrontEndContext::CAREER));
    }

    #[test]
    fn stage_13_unlocks_the_rap_sheet_and_replays_step_six_after_the_movie() {
        let mut settings = CareerSettings::new_career();
        settings.stage = 13;
        let mut ctx = FrontEndContext(FrontEndContext::CAREER.0 | FrontEndContext::BLACKLIST_SEQUENCE.0);
        let mut stepper = BlacklistStepper { counter: 5 };
        assert_eq!(stepper.advance(&mut settings, &mut ctx, false), RapSheetMovie);
        assert!(settings.flags.contains(Flags::HAS_RAP_SHEET));
        assert_eq!(stepper.counter, 5);
        assert_eq!(stepper.advance(&mut settings, &mut ctx, false), LeaveFrontEnd);
        assert!(ctx.contains(FrontEndContext::BLACKLIST_SEQUENCE), "bit 17 is still set when the leave runs");
        assert!(!settings.flags.contains(Flags::OPENING_SEEN));
    }

    #[test]
    fn stage_13_with_the_flag_already_set_just_leaves() {
        let mut settings = CareerSettings::new_career();
        settings.stage = 13;
        settings.flags.insert(Flags::HAS_RAP_SHEET);
        let mut ctx = FrontEndContext::CAREER;
        let mut stepper = BlacklistStepper { counter: 5 };
        assert_eq!(stepper.advance(&mut settings, &mut ctx, false), LeaveFrontEnd);
    }

    #[test]
    fn the_end_clears_the_blacklist_sequence_bit() {
        let mut settings = CareerSettings::new_career();
        settings.stage = 14;
        let mut ctx = FrontEndContext(FrontEndContext::CAREER.0 | FrontEndContext::BLACKLIST_SEQUENCE.0);
        let mut stepper = BlacklistStepper { counter: 6 };
        assert_eq!(stepper.advance(&mut settings, &mut ctx, false), End);
        assert!(!ctx.contains(FrontEndContext::BLACKLIST_SEQUENCE));
        assert!(ctx.contains(FrontEndContext::CAREER), "the other bits are kept");
    }

    #[test]
    fn step_seven_and_later_calls_end() {
        let mut settings = CareerSettings::new_career();
        settings.stage = 14;
        let mut ctx = FrontEndContext::CAREER;
        let mut stepper = BlacklistStepper { counter: 6 };
        assert_eq!(stepper.advance(&mut settings, &mut ctx, false), End);
        assert_eq!(stepper.advance(&mut settings, &mut ctx, false), End);
        let mut huge = BlacklistStepper { counter: i32::MAX };
        assert_eq!(huge.advance(&mut settings, &mut ctx, false), End);
    }
}

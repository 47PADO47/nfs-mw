//! The glue that applies one finished event to the career: the Blacklist, the race cash, the milestone and speed-trap
//! unlocks, the stage byte and the region-unlock screen (`docs/specs/career-blacklist.md` sections 4.2, 5.2, 5.4, 6
//! and 7.1; `docs/specs/career-economy-flows.md` section 7; `docs/specs/career-scripts-events.md` sections 6.2 and 7.2).
//!
//! Pure logic. The caller reads the race or challenge result from the game and passes it in with the mutable
//! parts of the career. Nothing here opens a file or sends a message; the caller shows the screens and sends the
//! SMS that [`EventOutcome`] reports.
//!
//! Rules the specs do not settle are marked **unconfirmed** next to the code:
//!
//! - Race cash is paid on a first place only **[unconfirmed]**: economy-flows section 7 says "placed first", while
//!   blacklist section 4.2 step 4 says "on every finish". This follows economy-flows.
//! - The challenge ladder (bins 18 and 19) has no rule here that a Blacklist can apply. Which pursuit challenge
//!   unlocks the next one is the unlock list of each race record, which the caller owns. The caller reports whether
//!   every required race of bin 19 is done through [`EventResult::challenge_series_complete`].
//! - The region screen is reported, not shown. The spec puts it on the safe-house arrival stepper, not on the race
//!   finish (regions section 7.1); the caller shows it there.

use super::Blacklist;
use crate::economy::cash::{RaceWin, award_race_win};
use crate::milestones::MilestoneSet;
use crate::milestones::speedtrap::{SpeedTrap, unlock_bin};
use crate::regions::{RegionPicture, region_unlock_screen};
use crate::settings::{CareerSettings, FIRST_RIVAL_STAGE, Flags, PROLOGUE_STAGE};
use crate::stable::Stable;

/// One finished event, as the caller read it from the game.
#[derive(Debug, Clone, PartialEq)]
pub struct EventResult {
    /// The event id (`1.5.2`) of a race, or the challenge name of a challenge.
    pub event: String,
    /// The local player's placing: 1 for first. Used for races only.
    pub placing: u8,
    /// The event's `CashValue` (attribute `0xD8BAA07B`). Paid on a first place only. Pass 0 for a challenge.
    pub cash_value: f32,
    /// True for a milestone or speed-trap challenge of [`EventResult::bin`]. False for a race.
    pub is_challenge: bool,
    /// The bin of a challenge. Not read for a race.
    pub bin: u8,
    /// True in career race mode (mode 2). Nothing is applied outside career mode.
    pub career: bool,
    /// The bounty total after this result, when the caller changed it (a speed trap or milestone bounty). `None`
    /// leaves the bounty as it is.
    pub bounty_total: Option<u32>,
    /// Caller-computed **[unconfirmed]**: every race of bin 19 is completed except `19.8.31` and the optional ones.
    /// Sets career flag `0x10000` ([`Flags::CHALLENGE_SERIES_DONE`]) when true.
    pub challenge_series_complete: bool,
}

/// What one event did. Empty or zero when nothing happened.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct EventOutcome {
    /// The bins that reached `done` during this result, in the order the Blacklist reported them.
    pub bins_done: Vec<u8>,
    /// The bins entered during this result, from the one above the old `CurrentBin` down to the new one.
    pub bins_entered: Vec<u8>,
    /// How many speed traps were unlocked by the bins entered.
    pub traps_unlocked: usize,
    /// The cash added by the race win (0 when no cash was paid).
    pub cash_paid: u32,
    /// How many impounded cars became releasable. The caller sends SMS 120 when this is not 0.
    pub released_cars: usize,
    /// The region-unlock screen the arrival stepper must show, or `None`.
    pub region_screen: Option<RegionPicture>,
}

/// Applies one finished event to the career.
///
/// Order: the Blacklist first (race or challenge, then the bounty), then the race cash, then the bin entries (the
/// milestones and speed traps follow them), then the stage byte. Returns what happened.
pub fn apply_event_result(
    career: &mut CareerSettings,
    blacklist: &mut Blacklist,
    milestones: &mut MilestoneSet,
    traps: &mut [SpeedTrap],
    stable: &mut Stable,
    result: &EventResult,
) -> EventOutcome {
    let mut outcome = EventOutcome::default();
    if !result.career {
        return outcome;
    }
    let previous = blacklist.current_bin();
    apply_event(career, blacklist, stable, result, &mut outcome);
    if let Some(total) = result.bounty_total {
        outcome.bins_done.extend(blacklist.bounty_changed(total));
    }
    if result.challenge_series_complete {
        career.flags.insert(Flags::CHALLENGE_SERIES_DONE);
    }
    apply_bin_entries(career, blacklist, milestones, traps, previous, &mut outcome);
    outcome.region_screen = region_screen(&outcome.bins_done, blacklist.current_bin());
    outcome
}

/// The first safe-house visit of a career still at the prologue stage (16) enters Blacklist bin 15, the way the rival
/// bio screen does (`docs/specs/career-safehouse.md` section 5.3; `safehouse::first_visit`). Does nothing at any other
/// stage, so a second call is a no-op. The bin entry, the milestones, the speed traps and the stage byte follow
/// [`apply_event_result`]. The first-visit rule grants no flag, SMS or marker, so none is written here.
pub fn enter_first_bin(
    career: &mut CareerSettings,
    blacklist: &mut Blacklist,
    milestones: &mut MilestoneSet,
    traps: &mut [SpeedTrap],
) -> EventOutcome {
    let mut outcome = EventOutcome::default();
    if career.stage != PROLOGUE_STAGE || blacklist.current_bin() != PROLOGUE_STAGE {
        return outcome;
    }
    let previous = blacklist.current_bin();
    blacklist.enter_bin(FIRST_RIVAL_STAGE);
    apply_bin_entries(career, blacklist, milestones, traps, previous, &mut outcome);
    outcome
}

/// Enters the bins between `previous` and the current bin: milestones and speed traps of each unlock, and the stage
/// byte is written. Shared by a race or challenge result and by the first safe-house visit.
fn apply_bin_entries(
    career: &mut CareerSettings,
    blacklist: &Blacklist,
    milestones: &mut MilestoneSet,
    traps: &mut [SpeedTrap],
    previous: u8,
    outcome: &mut EventOutcome,
) {
    let now = blacklist.current_bin();
    outcome.bins_entered = (now..previous).rev().collect();
    milestones.enter_bins(previous, now);
    for &bin in &outcome.bins_entered {
        unlock_bin(traps, bin);
    }
    outcome.traps_unlocked = traps.iter().filter(|trap| outcome.bins_entered.contains(&trap.bin())).count();
    blacklist.apply_to_settings(career);
}

/// Routes a challenge to the Blacklist's challenge count, and a race to [`apply_race`].
fn apply_event(
    career: &mut CareerSettings,
    blacklist: &mut Blacklist,
    stable: &mut Stable,
    result: &EventResult,
    outcome: &mut EventOutcome,
) {
    if result.is_challenge {
        outcome.bins_done = blacklist.challenge_passed(result.bin, &result.event);
        return;
    }
    apply_race(career, blacklist, stable, result, outcome);
}

/// A race finish: the Blacklist counts it, and a first place pays the race cash and ticks the impound countdown.
fn apply_race(
    career: &mut CareerSettings,
    blacklist: &mut Blacklist,
    stable: &mut Stable,
    result: &EventResult,
    outcome: &mut EventOutcome,
) {
    let first = result.placing == 1;
    outcome.bins_done = blacklist.race_finished(&result.event, first, &mut career.flags);
    if !first {
        return;
    }
    let RaceWin { cash, released_cars } = award_race_win(career, stable, result.cash_value);
    outcome.cash_paid = cash;
    outcome.released_cars = released_cars;
}

/// The region-unlock screen due after the arrival: stage 12 shows when rival 13 was beaten, stage 8 when rival 9
/// was (regions section 7.1). `stage` is the `CurrentBin` after the event.
fn region_screen(done: &[u8], stage: u8) -> Option<RegionPicture> {
    let picture = region_unlock_screen(stage)?;
    done.contains(&(stage + 1)).then_some(picture)
}

#[cfg(test)]
#[path = "results_tests.rs"]
mod tests;

//! The rival gate and the challenge button (`docs/specs/career-blacklist.md` sections 2.5, 3.1 and 3.3).
//!
//! The gate is three `>=` comparisons, all of them needed. The counters come from the caller or from
//! [`super::BinRecord::counters`]. Races won and challenges passed are recomputed from the race records and the
//! milestone table (section 4.3). The bounty is the stable total (section 3.2).

use super::bin::BinDef;
use crate::settings::Flags;

/// The two counters of a bin that the gate compares.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Counters {
    /// Free races of the bin completed in career mode.
    pub races_won: u32,
    /// Milestones and speed traps of the bin passed.
    pub challenges: u32,
}

/// True when the rival opens: races won, challenges passed and the total bounty each reach the bin's requirement
/// (`check_rival`, section 2.5).
pub fn rival_gate_open(def: &BinDef, counters: Counters, bounty: u32) -> bool {
    counters.races_won >= def.races_needed
        && counters.challenges >= def.challenges_needed
        && bounty >= def.bounty_needed
}

/// The challenge button of `SafeHouseRivalChallenge.fng` (section 3.3). Active when a boss race of the selected bin
/// is open (unlocked and not completed), or the career is completed (every rival is replayable), or the selected
/// bin is above the current one (the rival is already beaten and can be replayed).
pub fn challenge_button_active(current_bin: u8, selected_bin: u8, flags: Flags, boss_race_open: bool) -> bool {
    boss_race_open || flags.contains(Flags::CAREER_COMPLETE) || selected_bin > current_bin
}

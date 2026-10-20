//! The profile card of a [`Career`]: the completion percentages the main menu prints (`docs/specs/career-rapsheet-2.md`
//! section 4). The rules are `nfsmw_career::rapsheet::card`; this builds their inputs from the career being held.
//!
//! The milestones count when banked. The events are the free races and boss races of the Blacklist bins 1 to 15, counted
//! from what the bins record as won (the original reads a race record's "completed in career" flag, which this build keeps
//! as the bins' won lists: **[ours]**). The challenge series are the races of bin 19 other than `19.8.31`, none of which
//! the build records as completed outside career yet, so the series reads `0/total` **[ours]**.

use nfsmw_career::milestones::MilestoneState;
use nfsmw_career::rapsheet::{CardCounts, ProfileCard, lifetime_ranks};

use super::Career;

/// The event of the challenge series the card skips (`career-rapsheet-2.md` section 4).
const SKIPPED_SERIES_EVENT: &str = "19.8.31";

impl Career {
    /// The card of this career.
    pub fn card(&self) -> ProfileCard {
        ProfileCard::compute(&self.card_counts(), &lifetime_ranks(&self.stable, &self.history))
    }

    fn card_counts(&self) -> CardCounts {
        let milestones = self.milestones.milestones();
        let done = milestones.iter().filter(|m| m.state == MilestoneState::Complete).count();
        let (mut events_done, mut events_total) = (0, 0);
        for bin in self.data.bins.iter().filter_map(|def| self.blacklist.bin(def.number)) {
            events_done += bin.races_won.len() + bin.rival_races_won.len();
            events_total += bin.def.races.len() + bin.def.rival_events.len();
        }
        let series_total = self.data.gameplay.races.iter().filter(|race| is_series(race.event.as_deref())).count();
        CardCounts {
            milestones_done: count(done),
            milestones_total: count(milestones.len()),
            events_done: count(events_done),
            events_total: count(events_total),
            series_done: 0,
            series_total: count(series_total),
        }
    }
}

/// Whether an event id is a challenge series event: bin 19, but not the one the card skips.
fn is_series(event: Option<&str>) -> bool {
    event.is_some_and(|id| id.starts_with("19.") && id != SKIPPED_SERIES_EVENT)
}

fn count(n: usize) -> u32 {
    u32::try_from(n).unwrap_or(u32::MAX)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_series_are_the_events_of_bin_19_but_the_skipped_one() {
        assert!(is_series(Some("19.8.18")));
        assert!(!is_series(Some("19.8.31")));
        assert!(!is_series(Some("10.2.1")));
        assert!(!is_series(None));
    }
}

//! The rap sheet's counts that only the world can see: the cops that engaged the player, the cops damaged, the
//! helicopters and the roadblock statistics of a chase, written into the chase's [`PursuitStats`]
//! (`docs/specs/busted-aftermath.md` section 7, `docs/specs/career-rapsheet-2.md` section 2).

use std::collections::HashSet;

use nfsmw_career::rapsheet::PursuitStats;

use crate::scenes::world::ai::{RoadblockTally, TrafficWorld};

/// Slot of "roadblocks deployed" in [`PursuitStats::extras`].
const EXTRA_ROADBLOCKS_DEPLOYED: usize = 0;

/// What the career has seen of the running chase.
#[derive(Default)]
pub(super) struct ChaseTally {
    /// The cops that have come within the engagement radius.
    involved: HashSet<u32>,
    /// The most cops damaged at once (the world's damaged set only grows during a chase).
    damaged: u32,
    /// Helicopters that joined the chase.
    helicopters: u32,
    heli_present: bool,
}

impl ChaseTally {
    /// A new chase starts: the counts begin again.
    pub(super) fn begin(&mut self) {
        *self = ChaseTally::default();
    }

    /// One frame of the running chase.
    pub(super) fn observe(&mut self, traffic: &TrafficWorld) {
        self.involved.extend(traffic.engaged_cop_ids());
        self.damaged = self.damaged.max(traffic.cops_damaged());
        let heli = traffic.helicopter_status().is_some();
        if heli && !self.heli_present {
            self.helicopters = self.helicopters.saturating_add(1);
        }
        self.heli_present = heli;
    }

    /// Writes the counts into the chase's stats. The slots the world does not count yet (traffic cars hit, spike
    /// strips deployed, helicopter spike strips, patrol cars, support vehicles, property damage) stay 0.
    pub(super) fn write(&self, roadblocks: RoadblockTally, stats: &mut PursuitStats) {
        stats.cops_involved = u32::try_from(self.involved.len()).unwrap_or(u32::MAX);
        stats.cops_damaged = self.damaged;
        stats.helicopters = self.helicopters;
        stats.roadblocks_dodged = roadblocks.dodged;
        stats.spikes_dodged = roadblocks.spike_strips_dodged;
        stats.extras[EXTRA_ROADBLOCKS_DEPLOYED] = roadblocks.deployed;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_counts_land_in_the_chase_stats() {
        let tally =
            ChaseTally { involved: [1, 2, 3].into_iter().collect(), damaged: 4, helicopters: 1, heli_present: true };
        let roadblocks = RoadblockTally { deployed: 5, dodged: 3, props_deployed: 12, spike_strips_dodged: 2 };
        let mut stats = PursuitStats::default();
        tally.write(roadblocks, &mut stats);
        assert_eq!((stats.cops_involved, stats.cops_damaged, stats.helicopters), (3, 4, 1));
        assert_eq!((stats.roadblocks_dodged, stats.spikes_dodged, stats.extras[0]), (3, 2, 5));
        assert_eq!(stats.extras[1..], [0; 7], "the other sums are not counted");
    }

    #[test]
    fn a_new_chase_forgets_the_old_one() {
        let mut tally = ChaseTally { damaged: 9, helicopters: 2, heli_present: true, ..ChaseTally::default() };
        tally.involved.insert(7);
        tally.begin();
        let mut stats = PursuitStats::default();
        tally.write(RoadblockTally::default(), &mut stats);
        assert_eq!(stats, PursuitStats::default());
    }
}

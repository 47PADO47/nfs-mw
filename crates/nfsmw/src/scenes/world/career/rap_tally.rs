//! The rap sheet's counts that only the world can see: the cops that engaged the player, the cops damaged, the
//! helicopters and the roadblock statistics of a chase, written into the chase's [`PursuitStats`]
//! (`docs/specs/busted-aftermath.md` section 7, `docs/specs/career-rapsheet-2.md` section 2).

use nfsmw_career::rapsheet::PursuitStats;

use crate::scenes::world::ai::{PursuitCostTally, RoadblockTally, TrafficWorld};

/// What the career has seen of the running chase.
#[derive(Default)]
pub(super) struct ChaseTally {
    /// The most cops damaged at once (the world's damaged set only grows during a chase).
    damaged: u32,
    costs: PursuitCostTally,
}

impl ChaseTally {
    /// A new chase starts: the counts begin again.
    pub(super) fn begin(&mut self) {
        *self = ChaseTally::default();
    }

    /// One frame of the running chase.
    pub(super) fn observe(&mut self, traffic: &TrafficWorld) {
        self.damaged = self.damaged.max(traffic.cops_damaged());
        self.costs = traffic.pursuit_cost_tally();
    }

    /// Writes attachment and impact totals, including units that left before the scene sampled them.
    pub(super) fn write(&self, roadblocks: RoadblockTally, stats: &mut PursuitStats) {
        stats.cops_involved =
            self.costs.patrol_cars.saturating_add(self.costs.support_cars).saturating_add(self.costs.helicopters);
        stats.cops_damaged = self.damaged;
        stats.helicopters = self.costs.helicopters;
        stats.roadblocks_dodged = roadblocks.dodged;
        stats.spikes_dodged = roadblocks.spike_strips_dodged;
        stats.extras = [
            roadblocks.deployed,
            self.costs.traffic_hits,
            roadblocks.props_deployed,
            0,
            self.costs.patrol_cars,
            self.costs.support_cars,
            self.costs.property_value,
            self.costs.property_count,
        ];
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_counts_land_in_the_chase_stats() {
        let tally = ChaseTally {
            damaged: 4,
            costs: PursuitCostTally {
                patrol_cars: 6,
                support_cars: 7,
                helicopters: 1,
                traffic_hits: 8,
                property_value: 900,
                property_count: 3,
            },
        };
        let roadblocks = RoadblockTally { deployed: 5, dodged: 3, props_deployed: 12, spike_strips_dodged: 2 };
        let mut stats = PursuitStats::default();
        tally.write(roadblocks, &mut stats);
        assert_eq!((stats.cops_involved, stats.cops_damaged, stats.helicopters), (14, 4, 1));
        assert_eq!((stats.roadblocks_dodged, stats.spikes_dodged, stats.extras[0]), (3, 2, 5));
        assert_eq!(stats.extras, [5, 8, 12, 0, 6, 7, 900, 3]);
    }

    #[test]
    fn a_new_chase_forgets_the_old_one() {
        let mut tally =
            ChaseTally { damaged: 9, costs: PursuitCostTally { helicopters: 2, patrol_cars: 7, ..Default::default() } };
        tally.begin();
        let mut stats = PursuitStats::default();
        tally.write(RoadblockTally::default(), &mut stats);
        assert_eq!(stats, PursuitStats::default());
    }
}

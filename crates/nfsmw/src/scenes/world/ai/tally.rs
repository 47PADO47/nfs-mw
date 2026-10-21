//! What the career's rap sheet reads off a chase: the cops that engaged the player and the roadblock statistics
//! (`docs/specs/busted-aftermath.md` section 7). Read-only views of state the chase already keeps.

use super::TrafficWorld;

/// The chase's roadblock statistics (`NotifyRoadblockDeployed`, `NotifyRoadblockDodged`, `NotifySpikeStripDodged`).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct RoadblockTally {
    pub deployed: u32,
    pub dodged: u32,
    /// Sawhorses and spike strips alike.
    pub props_deployed: u32,
    pub spike_strips_dodged: u32,
}

impl TrafficWorld {
    /// The ids of the cops that have been within the engagement radius of the player in this chase so far (the ones
    /// that count toward "cops involved"). Empty with no chase.
    pub fn engaged_cop_ids(&self) -> Vec<u32> {
        self.pursuit.as_ref().map_or_else(Vec::new, |pursuit| pursuit.engaged.iter().copied().collect())
    }

    /// The roadblock statistics of the last chase; they stay after it ends until the next chase begins.
    pub fn roadblock_tally(&self) -> RoadblockTally {
        let stats = self.roadblocks.stats();
        RoadblockTally {
            deployed: stats.deployed,
            dodged: stats.dodged,
            props_deployed: stats.props_deployed,
            spike_strips_dodged: stats.spike_strips_dodged,
        }
    }
}

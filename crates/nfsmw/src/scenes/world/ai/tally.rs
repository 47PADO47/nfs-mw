//! What the career's rap sheet reads off a chase: the cops that engaged the player and the roadblock statistics
//! (`docs/specs/busted-aftermath.md` section 7). Read-only views of state the chase already keeps.

use super::TrafficWorld;

/// Deployment and impact totals retained until the next pursuit begins.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct PursuitCostTally {
    pub traffic_hits: u32,
    pub patrol_cars: u32,
    pub support_cars: u32,
    pub helicopters: u32,
    pub property_value: u32,
    pub property_count: u32,
}

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
    pub fn pursuit_cost_tally(&self) -> PursuitCostTally {
        self.police.cost_tally()
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

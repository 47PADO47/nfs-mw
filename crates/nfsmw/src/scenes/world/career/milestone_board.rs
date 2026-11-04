//! The HUD's milestone board during a free-roam chase in the career (`docs/specs/hud-milestone-board.md`): the current
//! bin's milestones in their order, each with its icon, its statistic's current value, its goal, and whether it is at
//! stake or banked. [decomp] The original builds the board this way every frame of a chase outside a race.

use nfsmw_career::milestones::{MilestoneState, StatType};

use crate::hud::{BoardSlot, MilestoneBoard, MilestoneDisplay};
use crate::scenes::world::WorldScene;

/// The texture key of a statistic's icon on the board; 0 for a statistic with none. [decomp] the icon table of the HUD
/// element (the length icon in its board form).
pub(super) fn icon(stat: StatType) -> u32 {
    match stat {
        StatType::PursuitLength => 0x950F_CEBC,
        StatType::PursuitEvasionTime => 0x831B_7EBE,
        StatType::BountyInPursuit => 0xB4E6_456B,
        StatType::CopsDestroyedInPursuit | StatType::CopsDestroyed => 0x0FE6_08E6,
        StatType::CostToStateInPursuit => 0x8780_7869,
        StatType::RoadblocksDodged => 0xC439_59D2,
        StatType::CopsDamaged => 0x88E8_DE9E,
        StatType::TotalInfractions => 0x6784_A80E,
        StatType::TireSpikesDodged => 0xE621_B2EF,
        StatType::SpeedtrapSpeed => 0x3FFE_9EC9,
        _ => 0,
    }
}

/// How the board writes a statistic: the two times as minutes and seconds, the rest as numbers.
fn display(stat: StatType) -> MilestoneDisplay {
    match stat {
        StatType::PursuitLength | StatType::PursuitEvasionTime => MilestoneDisplay::Time,
        _ => MilestoneDisplay::Number,
    }
}

impl WorldScene {
    /// The board of this frame: during a chase outside a race, in the career; `None` otherwise.
    pub(in crate::scenes::world) fn milestone_board(&self, pursuing: bool) -> Option<MilestoneBoard> {
        if !pursuing || self.race.is_some() {
            return None;
        }
        self.career.handle.with(|career| {
            let bin = career.stage();
            let slots = career
                .milestones
                .milestones()
                .iter()
                .filter(|m| m.def.bin == bin)
                .take(crate::hud::MILESTONE_SLOTS)
                .map(|m| BoardSlot {
                    icon: icon(m.def.stat),
                    display: display(m.def.stat),
                    value: career.milestones.stat(m.def.stat).current.unwrap_or(0.0),
                    goal: m.def.threshold,
                    done: matches!(m.state, MilestoneState::AtStake | MilestoneState::Complete),
                })
                .collect();
            MilestoneBoard { slots }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_counted_statistic_has_its_board_icon() {
        for stat in [StatType::PursuitLength, StatType::CostToStateInPursuit, StatType::RoadblocksDodged] {
            assert_ne!(icon(stat), 0, "{stat:?}");
        }
        assert_eq!(icon(StatType::HeatMeter), 0);
        assert_eq!(display(StatType::PursuitEvasionTime), MilestoneDisplay::Time);
        assert_eq!(display(StatType::BountyInPursuit), MilestoneDisplay::Number);
    }
}

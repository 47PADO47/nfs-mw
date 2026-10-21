//! The rewards of a chase for the rewards screen (`PostRace_MilestoneRewards.fng`): the milestones the escape banked,
//! each with its bounty. The original words each milestone with a language label (`BLACKLIST_PURSUIT_MILESTONES_NN`) whose
//! use is not fully read (`docs/specs/career-milestones.md` section 3.9), so the texts here are the rewrite's own.

use nfsmw_career::milestones::{Banked, MilestoneSet, StatType};

use crate::viewer::{RewardLine, RewardsReport};

/// The line a milestone gets: what it asked for, in plain words.
pub(super) fn milestone_text(stat: StatType, threshold: f32) -> String {
    let n = threshold.round() as u64;
    let time = |secs: u64| format!("{:02}:{:02}", secs / 60, secs % 60);
    match stat {
        StatType::PursuitLength => format!("Chase for {}", time(n)),
        StatType::PursuitEvasionTime => format!("Escape within {}", time(n)),
        StatType::BountyInPursuit => format!("{n} bounty in one chase"),
        StatType::CopsDestroyedInPursuit | StatType::CopsDestroyed => format!("{n} police cars destroyed"),
        StatType::CostToStateInPursuit | StatType::CostToState => format!("{n} cost to state"),
        StatType::RoadblocksDodged => format!("{n} roadblocks dodged"),
        StatType::CopsDamaged => format!("{n} police cars hit"),
        StatType::TotalInfractions => format!("{n} infractions in one chase"),
        StatType::TireSpikesDodged => format!("{n} spike strips dodged"),
        _ => format!("{} {n}", stat.name()),
    }
}

/// The icon texture of a statistic's milestone: the row of icons of `PostRace_MilestoneRewards.fng` by what each shows
/// (the time icons: max time is the chase length, min time the evasion time) **[unconfirmed]**.
pub(super) fn milestone_icon(stat: StatType) -> &'static str {
    match stat {
        StatType::PursuitLength => "Milestone_Pursuit_MaxTime",
        StatType::PursuitEvasionTime => "Milestone_Pursuit_MinTime",
        StatType::BountyInPursuit => "Milestone_PursuitBounty",
        StatType::CopsDestroyedInPursuit | StatType::CopsDestroyed => "Cops_TakenOut_Icon",
        StatType::CostToStateInPursuit | StatType::CostToState => "Milestone_Cost_To_State",
        StatType::RoadblocksDodged => "Milestone_Roadblocks",
        StatType::CopsDamaged => "Cops_Damaged_Icon",
        StatType::TotalInfractions => "Milestone_Infractions",
        StatType::TireSpikesDodged => "Milestone_SpikeStrips",
        _ => "Milestone_Main",
    }
}

/// The report of the milestones `banked`, with the car's bounty after them. `None` when nothing was banked.
pub(super) fn report_of(set: &MilestoneSet, banked: &[Banked], total_bounty: u32) -> Option<RewardsReport> {
    let lines: Vec<RewardLine> = banked
        .iter()
        .filter_map(|b| set.milestones().iter().find(|m| m.def.key == b.key).map(|m| (m, b.bounty)))
        .map(|(m, bounty)| RewardLine {
            text: milestone_text(m.def.stat, m.def.threshold),
            bounty,
            icon: milestone_icon(m.def.stat),
        })
        .collect();
    (!lines.is_empty()).then_some(RewardsReport { lines, total_bounty })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_milestone_reads_as_what_it_asked_for() {
        assert_eq!(milestone_text(StatType::CopsDestroyedInPursuit, 10.0), "10 police cars destroyed");
        assert_eq!(milestone_text(StatType::PursuitLength, 300.0), "Chase for 05:00");
        assert_eq!(milestone_text(StatType::PursuitEvasionTime, 95.0), "Escape within 01:35");
        assert_eq!(milestone_text(StatType::CostToStateInPursuit, 25000.0), "25000 cost to state");
    }

    #[test]
    fn nothing_banked_has_no_report() {
        let set = MilestoneSet::new(&[]);
        assert_eq!(report_of(&set, &[], 10), None);
        assert_eq!(report_of(&set, &[Banked { key: 9, bounty: 5 }], 10), None, "an unknown key is skipped");
    }
}

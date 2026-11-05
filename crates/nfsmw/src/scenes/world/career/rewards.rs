//! The rewards of a chase for the rewards screen (`PostRace_MilestoneRewards.fng`). After an escape in free roam: the
//! chase's duration with its time bounty, the cop cars immobilized with their bounty, then the milestones the escape
//! banked, and the statistics page that Continue opens (`docs/specs/pursuit-escape-results.md`). After a race's chase:
//! the milestones only. The original words each milestone with a language label (`BLACKLIST_PURSUIT_MILESTONES_NN`)
//! whose use is not fully read (`docs/specs/career-milestones.md` section 3.9), so the milestones' texts here are the
//! rewrite's own.

use blackbox_pursuit::InfractionLog;
use blackbox_text::label_hash;
use nfsmw_career::milestones::{Banked, MilestoneSet, MilestoneState, StatType};
use nfsmw_career::rapsheet::PursuitStats;
use nfsmw_data::gameplay::GameplayData;

use crate::i18n::{self, fmt::grouped, t};
use crate::viewer::{
    MILESTONE_LINES, MilestoneLine, MilestoneStatus, PursuitPage, PursuitReport, RewardLine, RewardsReport,
};

/// The language labels of the escape's first two rows **[exe]**: `Pursuit Duration` and `Immobilized Police Vehicles`.
const DURATION_LABEL: u32 = 0x4D64_888D;
const IMMOBILIZED_LABEL: u32 = 0x23F6_E732;

/// What an escape in free roam banked: the time in sight's and the wrecks' bounty, and the milestones.
pub(super) struct Escape<'a> {
    pub secs: f32,
    pub time_bounty: u32,
    pub destroy_bounty: u32,
    pub stats: &'a PursuitStats,
    /// The car's unserved infractions after the chase.
    pub unserved: u32,
    /// The chase's infractions.
    pub committed: InfractionLog,
    /// The milestones page: [`milestone_page`] after the escape banked.
    pub milestones: [Option<MilestoneLine>; MILESTONE_LINES],
}

/// The milestones page of the statistics screen: one line per milestone of `bin`, in the set's order, with the
/// statistic's current value, the threshold and the state (`docs/specs/pursuit-escape-results.md` section 3.1). The
/// package has room for nine; more are left out.
pub(super) fn milestone_page(
    set: &MilestoneSet,
    data: &GameplayData,
    bin: u8,
) -> [Option<MilestoneLine>; MILESTONE_LINES] {
    let mut lines = [None; MILESTONE_LINES];
    let in_bin = set.milestones().iter().filter(|m| m.def.bin == bin);
    for (slot, milestone) in lines.iter_mut().zip(in_bin) {
        let stat = milestone.def.stat;
        let label = data
            .milestone_types
            .iter()
            .find(|kind| kind.name == stat.name())
            .and_then(|kind| kind.label)
            .map(|index| label_hash(&format!("BLACKLIST_PURSUIT_MILESTONES_{index:02}_SHORT")));
        let status = match milestone.state {
            MilestoneState::AtStake => MilestoneStatus::AtStake,
            MilestoneState::Complete => MilestoneStatus::Complete,
            MilestoneState::Locked | MilestoneState::Active => MilestoneStatus::Open,
        };
        let value = set.stat(stat).current.unwrap_or(0.0).max(0.0);
        *slot = Some(MilestoneLine { stat, label, value, goal: milestone.def.threshold, status });
    }
    lines
}

/// The line a milestone gets: what it asked for, in plain words.
pub(super) fn milestone_text(stat: StatType, threshold: f32) -> String {
    let n = threshold.round() as u64;
    let time = |secs: u64| format!("{:02}:{:02}", secs / 60, secs % 60);
    match stat {
        StatType::PursuitLength => t!("race-milestone-chase-for", time = time(n)),
        StatType::PursuitEvasionTime => t!("race-milestone-escape-within", time = time(n)),
        StatType::BountyInPursuit => t!("race-milestone-bounty", n = n),
        StatType::CopsDestroyedInPursuit | StatType::CopsDestroyed => t!("race-milestone-cops-destroyed", n = n),
        StatType::CostToStateInPursuit | StatType::CostToState => t!("race-milestone-cost-to-state", n = n),
        StatType::RoadblocksDodged => t!("race-milestone-roadblocks", n = n),
        StatType::CopsDamaged => t!("race-milestone-cops-hit", n = n),
        StatType::TotalInfractions => t!("race-milestone-infractions", n = n),
        StatType::TireSpikesDodged => t!("race-milestone-spike-strips", n = n),
        // A statistic the rewards screen has no words for: its own name, which is not language.
        _ => format!("{} {n}", stat.name()),
    }
}

/// The icon texture of a statistic's milestone: the row of icons of `PostRace_MilestoneRewards.fng` by what each shows
/// (the time icons: min time is the chase length, max time the evasion time) **[exe]**.
pub(super) fn milestone_icon(stat: StatType) -> &'static str {
    match stat {
        StatType::PursuitLength => "Milestone_Pursuit_MinTime",
        StatType::PursuitEvasionTime => "Milestone_Pursuit_MaxTime",
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

/// The rows of the milestones `banked`, in the order they were banked. A key the set does not know is skipped.
fn milestone_lines(set: &MilestoneSet, banked: &[Banked]) -> Vec<RewardLine> {
    banked
        .iter()
        .filter_map(|b| set.milestones().iter().find(|m| m.def.key == b.key).map(|m| (m, b.bounty)))
        .map(|(m, bounty)| RewardLine {
            text: milestone_text(m.def.stat, m.def.threshold),
            bounty,
            icon: milestone_icon(m.def.stat),
        })
        .collect()
}

/// The report of a race's chase: the milestones `banked`. `None` when nothing was banked.
pub(super) fn report_of(set: &MilestoneSet, banked: &[Banked]) -> Option<RewardsReport> {
    let lines = milestone_lines(set, banked);
    (!lines.is_empty()).then_some(RewardsReport { lines, pursuit: None })
}

/// The report of an escape in free roam: always shown **[exe]**. The duration row comes first; the immobilized row only
/// when the wrecks paid a bounty; then the milestones (section 2 of the spec).
pub(super) fn escape_report(set: &MilestoneSet, banked: &[Banked], escape: &Escape) -> RewardsReport {
    let mut lines = vec![RewardLine {
        text: i18n::game_or_error(DURATION_LABEL),
        bounty: escape.time_bounty,
        icon: "Milestone_Pursuit_Duration",
    }];
    if escape.destroy_bounty > 0 {
        let wrecked = grouped(i64::from(escape.stats.cops_destroyed));
        lines.push(RewardLine {
            text: format!("{}: {wrecked}", i18n::game_or_error(IMMOBILIZED_LABEL)),
            bounty: escape.destroy_bounty,
            icon: "Cops_TakenOut_Icon",
        });
    }
    lines.extend(milestone_lines(set, banked));
    RewardsReport { lines, pursuit: Some(chase_page(escape)) }
}

/// The statistics page of an escape in free roam (section 3 of the spec).
fn chase_page(escape: &Escape) -> PursuitReport {
    let stats = escape.stats;
    PursuitReport {
        secs: escape.secs,
        evasion: Some(escape.secs),
        cops_damaged: stats.cops_damaged,
        cops_destroyed: stats.cops_destroyed,
        roadblocks_dodged: stats.roadblocks_dodged,
        spike_strips_dodged: stats.spikes_dodged,
        cost_to_state: stats.cost_to_state,
        infractions: escape.unserved,
        goal: None,
        page: PursuitPage::Chase,
        committed: escape.committed,
        milestones: escape.milestones,
    }
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
    fn a_race_chase_with_nothing_banked_has_no_report() {
        let set = MilestoneSet::new(&[]);
        assert_eq!(report_of(&set, &[]), None);
        assert_eq!(report_of(&set, &[Banked { key: 9, bounty: 5 }]), None, "an unknown key is skipped");
    }

    fn escape(stats: &PursuitStats, destroy_bounty: u32) -> Escape<'_> {
        Escape {
            secs: 95.5,
            time_bounty: 1200,
            destroy_bounty,
            stats,
            unserved: 3,
            committed: InfractionLog::default(),
            milestones: [None; MILESTONE_LINES],
        }
    }

    #[test]
    fn an_escape_always_has_its_duration_row_and_its_statistics_page() {
        let set = MilestoneSet::new(&[]);
        let stats = PursuitStats { cops_damaged: 4, roadblocks_dodged: 2, spikes_dodged: 1, ..PursuitStats::default() };
        let report = escape_report(&set, &[], &escape(&stats, 0));
        assert_eq!(report.lines.len(), 1, "no wreck bounty, no milestone: the duration row alone");
        assert_eq!((report.lines[0].bounty, report.lines[0].icon), (1200, "Milestone_Pursuit_Duration"));
        let page = report.pursuit.expect("Continue opens the statistics");
        assert_eq!(page.page, PursuitPage::Chase);
        assert_eq!((page.cops_damaged, page.spike_strips_dodged, page.infractions), (4, 1, 3));
    }

    #[test]
    fn wrecks_that_paid_a_bounty_take_the_second_row() {
        let set = MilestoneSet::new(&[]);
        let stats = PursuitStats { cops_destroyed: 3, ..PursuitStats::default() };
        let report = escape_report(&set, &[], &escape(&stats, 1500));
        assert_eq!(report.lines.len(), 2);
        assert!(report.lines[1].text.ends_with(": 3"), "{}", report.lines[1].text);
        assert_eq!((report.lines[1].bounty, report.lines[1].icon), (1500, "Cops_TakenOut_Icon"));
    }
}

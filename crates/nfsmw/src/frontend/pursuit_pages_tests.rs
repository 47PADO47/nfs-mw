use blackbox_pursuit::{Fines, Infraction, InfractionLog};
use nfsmw_career::milestones::StatType;

use super::*;
use crate::viewer::{MILESTONE_LINES, MilestoneLine, MilestoneStatus};

#[test]
fn the_infractions_page_ticks_what_the_chase_committed_in_the_originals_order() {
    let mut log = InfractionLog::default();
    let fines = Fines::new([1; 8]);
    log.record(Infraction::Racing, &fines);
    log.record(Infraction::OffRoad, &fines);
    let marks: Vec<Mark> = infractions(&log).iter().map(|line| line.mark).collect();
    let (tick, empty) = (Mark::Ticked, Mark::Empty);
    assert_eq!(marks, [tick, empty, empty, empty, empty, empty, empty, tick], "Speeding (mask 2) first");
    assert!(infractions(&log).iter().all(|line| line.value.is_none()), "the value column is hidden");
}

fn line(stat: StatType, value: f32, goal: f32, status: MilestoneStatus) -> MilestoneLine {
    MilestoneLine { stat, label: None, value, goal, status }
}

#[test]
fn an_open_milestone_shows_its_value_against_the_goal_and_a_ticked_one_the_goal_alone() {
    let mut page = [None; MILESTONE_LINES];
    page[0] = Some(line(StatType::CopsDestroyedInPursuit, 3.0, 10.0, MilestoneStatus::Open));
    page[1] = Some(line(StatType::CostToStateInPursuit, 40_000.0, 25_000.0, MilestoneStatus::AtStake));
    page[2] = Some(line(StatType::PursuitLength, 400.0, 300.0, MilestoneStatus::Complete));
    page[3] = Some(line(StatType::CopsDestroyedInPursuit, 12.0, 10.0, MilestoneStatus::Open));
    let lines = milestones(&page);
    let shown: Vec<(Option<&str>, Mark)> = lines.iter().map(|l| (l.value.as_deref(), l.mark)).collect();
    assert_eq!(
        shown,
        [
            (Some("3/10"), Mark::Empty),
            (Some("25,000"), Mark::Ticked),
            (Some("05:00.00"), Mark::Banked),
            (Some("10/10"), Mark::Empty),
        ],
        "an open value is clamped to its goal"
    );
}

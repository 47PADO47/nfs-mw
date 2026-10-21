//! The strategy flow: formations, backup requests, calls to position, outcomes, heat jumps, dead air.

use super::super::*;
use super::support::*;

fn formation_cop(id: u32, distance: f32) -> CopInfo {
    CopInfo { in_formation: true, ..cop(id, distance) }
}

/// A chase with `cops` already past its opening.
fn running(seed: u64, cops: Vec<CopInfo>) -> Chase {
    let mut chase = Chase::new(seed);
    chase.snap.cops = cops;
    chase.run(16.0);
    chase
}

#[test]
fn a_group_in_formation_hears_the_formation_announced() {
    let mut chase = running(1, vec![formation_cop(10, 40.0), formation_cop(11, 60.0), formation_cop(12, 80.0)]);
    chase.snap.pursuit.strategy = Strategy::Pit;
    chase.run(10.0);
    // The formation was Follow when the flow started; the change to a pit resets it with "strategy reset".
    let lines = chase.radio.lines();
    assert!(lines.contains(&Line::StrategyReset) || lines.contains(&Line::InitiateStrategy), "{lines:?}");
}

#[test]
fn the_leader_announces_the_formation_when_the_flow_starts() {
    let mut chase = Chase::new(2);
    chase.snap.pursuit.strategy = Strategy::BoxIn;
    chase.snap.cops = vec![formation_cop(10, 40.0), formation_cop(11, 60.0)];
    chase.run(16.0);
    let announce = chase.radio.first(Line::InitiateStrategy).expect("initiate strategy");
    assert_eq!(announce.params.rolling, Some(RollingStrategy::Box));
    assert!(layout().primary.contains(&announce.speaker));
}

#[test]
fn a_lone_cop_in_sight_says_what_it_will_do() {
    let mut chase = Chase::new(3);
    chase.snap.pursuit.strategy = Strategy::Pit;
    chase.snap.cops = vec![formation_cop(10, 40.0), CopInfo { los: false, ..cop(11, 90.0) }];
    chase.run(16.0);
    let own = chase.radio.first(Line::SelfStrategy).expect("self strategy");
    assert!(matches!(
        own.params.strategy,
        Some(SelfStrategy::Pit | SelfStrategy::SelfPit | SelfStrategy::SideRam | SelfStrategy::Ram)
    ));
}

#[test]
fn a_single_cop_calls_for_backup_and_dispatch_answers() {
    let mut chase = Chase::new(4);
    chase.run(16.0);
    let lines = chase.radio.lines();
    let call = lines.iter().position(|l| *l == Line::InitialCallForBackup).expect("the first call for backup");
    let reply =
        lines.iter().position(|l| matches!(l, Line::DispBackupReply | Line::DispBackupEta)).expect("dispatch answers");
    assert!(reply > call, "{lines:?}");
    let answer = chase.radio.first(Line::DispBackupReply);
    if let Some(answer) = answer {
        assert_eq!(answer.params.yes, Some(true));
        assert_eq!(answer.params.backup, Some(Backup::GroundUnits));
        assert!(answer.params.subject.is_some(), "dispatch names the unit that asked");
    }
}

#[test]
fn a_denied_request_gets_a_negative_reply() {
    let mut chase = running(5, vec![cop(10, 40.0), cop(11, 70.0)]);
    chase.notices.push(Notice::BackupDenied(Backup::Rhino));
    chase.run(10.0);
    let lines = chase.radio.lines();
    assert!(lines.contains(&Line::NegativeBackupReply) || lines.contains(&Line::DispBackupUpdate), "{lines:?}");
    let reply = chase.radio.first(Line::DispBackupReply).or(chase.radio.first(Line::DispBackupUpdate)).unwrap();
    assert_eq!(reply.params.yes, Some(false));
}

#[test]
fn a_heavy_support_call_names_the_backup() {
    let mut chase = running(6, vec![cop(10, 40.0), cop(11, 70.0)]);
    chase.notices.push(Notice::BackupRequested(Backup::Rhino));
    chase.run(6.0);
    let call =
        chase.radio.first(Line::InitialCallForBackup).or(chase.radio.first(Line::CallForBackup)).expect("a call");
    assert!(call.speaker != 0);
    if call.line == Line::CallForBackup {
        assert_eq!(call.params.backup, Some(Backup::Rhino));
    }
}

#[test]
fn the_backup_call_goes_through_its_stages() {
    let mut chase = Chase::new(7);
    chase.snap.pursuit.backup_eta = 45.0;
    chase.run(16.0);
    chase.notices.push(Notice::BackupRequested(Backup::GroundUnits));
    chase.run(30.0);
    chase.notices.push(Notice::BackupRequested(Backup::GroundUnits));
    chase.run(30.0);
    chase.notices.push(Notice::BackupRequested(Backup::GroundUnits));
    chase.run(30.0);
    let lines = chase.radio.lines();
    assert!(lines.contains(&Line::InitialCallForBackup), "{lines:?}");
    assert!(lines.contains(&Line::CallForBackup), "{lines:?}");
    let etas = chase.radio.all(Line::DispBackupEta);
    assert!(
        etas.iter().all(|e| e.params.eta == Some(Eta::Min1)),
        "an arrival of 45 seconds is about a minute: {etas:?}"
    );
}

#[test]
fn a_collapsing_box_calls_the_cops_into_position_or_executes() {
    let mut chase = Chase::new(8);
    chase.snap.pursuit.strategy = Strategy::BoxIn;
    chase.snap.cops = vec![formation_cop(10, 40.0), formation_cop(11, 60.0)];
    chase.run(16.0);
    chase.snap.pursuit.collapse_active = true;
    chase.run(8.0);
    let lines = chase.radio.lines();
    assert!(lines.contains(&Line::StrategyExecute) || lines.contains(&Line::CallToPosition), "{lines:?}");
}

#[test]
fn an_outcome_is_commented_after_a_strategy_line() {
    let mut chase = Chase::new(9);
    chase.snap.pursuit.strategy = Strategy::Pit;
    chase.snap.cops = vec![formation_cop(10, 40.0), formation_cop(11, 60.0)];
    chase.run(16.0);
    chase.snap.pursuit.collapse_active = true;
    chase.run(20.0);
    let lines = chase.radio.lines();
    let outcome = lines.contains(&Line::AnticipateSuccess)
        || lines.contains(&Line::AnticipateFail)
        || lines.contains(&Line::StrategyReset);
    assert!(outcome, "{lines:?}");
}

#[test]
fn a_cop_falling_back_far_behind_says_the_suspect_is_outrunning_them() {
    let mut chase = Chase::new(10);
    chase.snap.cops = vec![cop(10, 200.0), CopInfo { los: false, ..cop(11, 220.0) }];
    chase.snap.cops[0].ahead = false;
    chase.run(30.0);
    assert!(chase.radio.lines().contains(&Line::SuspectOutrun), "{:?}", chase.radio.lines());
}

#[test]
fn the_heat_rising_after_the_opening_may_be_announced() {
    let mut seen = Vec::new();
    for seed in 0..16 {
        let mut chase = running(seed, vec![cop(10, 40.0), cop(11, 70.0)]);
        chase.snap.pursuit.heat = 2.4;
        chase.run(3.0);
        seen.extend(chase.radio.all(Line::HeatJump));
    }
    assert!(!seen.is_empty(), "a coin flip over sixteen chases says it at least once");
    assert!(seen.iter().all(|r| r.params.heat_level == Some(2)));
    let dispatch = seen.iter().any(|r| r.speaker == DISPATCH);
    let cop = seen.iter().any(|r| r.speaker != DISPATCH);
    assert!(dispatch && cop, "dispatch and the cops both announce it");
}

#[test]
fn the_jurisdiction_shifts_with_the_heat() {
    let mut shifts = Vec::new();
    for seed in 0..40 {
        let mut chase = running(seed, vec![cop(10, 40.0), cop(11, 70.0)]);
        chase.snap.pursuit.heat = 3.5;
        chase.run(3.0);
        shifts.extend(chase.radio.all(Line::DispJurisShift));
    }
    assert!(!shifts.is_empty());
    assert!(shifts.iter().all(|r| r.params.jurisdiction == Some(Jurisdiction::State)));
}

#[test]
fn dispatch_asks_for_an_update_when_a_long_chase_goes_quiet() {
    let mut chase = running(11, vec![cop(10, 40.0), cop(11, 70.0)]);
    chase.run(80.0);
    let update = chase.radio.first(Line::DispPursuitUpdate).expect("pursuit update");
    assert_eq!(update.speaker, DISPATCH);
    assert!(update.params.subject.is_some());
    let replies = chase.radio.count(Line::PursuitUpdateReply) + chase.radio.count(Line::LocationReport);
    assert!(replies > 0);
}

#[test]
fn a_cop_that_leaves_the_chase_says_why() {
    let mut chase = running(12, vec![formation_cop(10, 40.0), formation_cop(11, 70.0), formation_cop(12, 90.0)]);
    chase.run(50.0);
    let before = chase.radio.count(Line::Bailout);
    chase.snap.cops[1].health = 0.2;
    chase.run(0.5);
    chase.snap.cops[1].active = false;
    chase.run(3.0);
    let bailout = chase.radio.all(Line::Bailout);
    assert!(bailout.len() > before, "{:?}", chase.radio.lines());
    assert_eq!(bailout.last().unwrap().params.bailout, Some(Bailout::HighIntensity));
}

#[test]
fn a_wrecked_cop_is_called_in_as_disabled() {
    let mut chase = running(13, vec![formation_cop(10, 40.0), formation_cop(11, 70.0)]);
    chase.run(5.0);
    chase.snap.cops[1].destroyed = true;
    chase.run(0.5);
    chase.snap.cops[1].active = false;
    chase.run(3.0);
    let lines = chase.radio.lines();
    assert!(lines.contains(&Line::UnitDisabled), "{lines:?}");
}

#[test]
fn a_cop_wrecked_and_gone_in_one_update_is_still_called_in_as_disabled() {
    let mut chase = running(15, vec![formation_cop(10, 40.0), formation_cop(11, 70.0)]);
    chase.run(5.0);
    chase.snap.cops[1].destroyed = true;
    chase.snap.cops[1].active = false;
    chase.run(3.0);
    let lines = chase.radio.lines();
    assert!(lines.contains(&Line::UnitDisabled), "{lines:?}");
    assert_eq!(chase.radio.count(Line::Bailout), 0, "a wreck does not bail out");
}

#[test]
fn a_new_cop_in_the_chase_announces_itself() {
    let mut chase = running(14, vec![cop(10, 40.0), cop(11, 70.0)]);
    chase.run(5.0);
    chase.snap.cops.push(cop(12, 120.0));
    chase.run(3.0);
    let lines = chase.radio.lines();
    assert!(lines.contains(&Line::BackupArrives) || lines.contains(&Line::UnitBackupReply), "{lines:?}");
}

#[test]
fn the_helicopter_has_a_voice_of_its_own() {
    let mut chase = running(15, vec![cop(10, 40.0), cop(11, 70.0)]);
    chase.snap.heli =
        Some(HeliInfo { id: 99, active: true, los: true, distance: 80.0, health: 1.0, fuel_secs: Some(100.0) });
    chase.run(5.0);
    let arrives = chase.radio.first(Line::BackupArrives).expect("the helicopter checks in");
    assert_eq!(arrives.speaker, HELI);
    assert_eq!(arrives.params.battalion, Some(Battalion(64)));
}

#[test]
fn a_helicopter_low_on_fuel_says_it_must_leave() {
    let mut chase = running(16, vec![cop(10, 40.0), cop(11, 70.0)]);
    chase.snap.heli =
        Some(HeliInfo { id: 99, active: true, los: true, distance: 80.0, health: 1.0, fuel_secs: Some(5.0) });
    chase.run(4.0);
    let bail = chase.radio.first(Line::HeliIntentToBail).expect("intent to bail");
    assert_eq!(bail.params.heli_bailout, Some(HeliBailout::FuelLow));
}

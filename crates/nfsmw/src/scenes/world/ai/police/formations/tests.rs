use super::*;

#[test]
fn an_expired_picker_cannot_interrupt_a_running_finisher() {
    let mut police = Police::default();
    police.begin(3);
    police.last_formation = Some(Formation::Pit);
    police.tick_finisher(1.2, Some((1, 0.0)), 60.0);
    assert!(police.finisher_running());
    police.formations.heat_changed();
    police.tick_formations(0.5, 3, [0.5; 2]);
    assert_eq!(police.last_formation(), Some(Formation::Pit));
    assert!(police.finisher_running());
}

#[test]
fn selecting_the_same_formation_preserves_its_arming_timer() {
    let mut police = Police::default();
    police.begin(3);
    police.last_formation = Some(Formation::StaggerFollow);
    // An identical selection must leave its state in place, including a retained timer.
    let params = Params { tolerance: 1.0, time_to_arm: 4.0, run_time: 5.0, min_cops: 2 };
    police.finisher.tick(1.0, 2, Some(0.0), &params);
    police.tick_formations(0.0, 3, [0.0; 2]);
    assert_eq!(police.finisher.in_position(), 1.0);
}

#[test]
fn setup_focus_holds_selection_for_fifteen_seconds_after_it_ends() {
    let mut police = Police::default();
    police.begin(1);
    police.set_speech_enabled(true);
    police.set_speech_setup(true);
    police.tick_formations(20.0, 1, [0.0; 2]);
    assert_eq!(police.last_formation(), None);
    police.set_speech_setup(false);
    police.tick_formations(14.9, 1, [0.0; 2]);
    assert_eq!(police.last_formation(), None);
    police.tick_formations(0.11, 1, [0.0; 2]);
    assert_eq!(police.last_formation(), Some(Formation::StaggerFollow));
}

#[test]
fn heat_changes_the_complete_finisher_run_and_progressive_closure() {
    for (level, duration) in [(1, 4.0), (5, 8.0), (7, 10.0), (10, 3.0)] {
        let mut police = Police::default();
        police.set_forced_pursuit(true);
        police.begin(level);
        assert_eq!(police.level(), Some(level));
        police.last_formation = Some(Formation::BoxIn);
        assert_eq!(police.formation_progress(), 1.0);
        police.tick_finisher(2.0, Some((2, 0.0)), 60.0);
        assert!((police.formation_progress() - 0.5).abs() < 1e-5);
        police.tick_finisher(2.0, Some((2, 0.0)), 60.0);
        assert!(police.finisher_running());
        police.tick_finisher(duration - 0.01, None, 60.0);
        assert!(police.finisher_running(), "heat {level}");
        police.tick_finisher(0.02, None, 60.0);
        assert!(!police.finisher_running(), "heat {level}");
    }
}

#[test]
fn collapse_prevents_arming_an_attack_until_the_target_drives_away() {
    let mut police = Police::default();
    police.begin(3);
    police.last_formation = Some(Formation::BoxIn);
    police.tick_finisher(8.0, Some((2, 0.0)), 0.0);
    assert!(!police.finisher_running());
    assert_eq!(police.formation_progress(), 1.0);
    police.tick_finisher(4.0, Some((2, 0.0)), 60.0);
    assert!(police.finisher_running());
}

#[test]
fn a_runtime_row_controls_picker_goal_and_collapse_tuning_together() {
    let mut police = Police::default();
    police.begin(1);
    police.set_formation_rows(vec![nfsmw_data::pursuit::FormationRow {
        records: vec![nfsmw_data::pursuit::FormationRecord { kind: 2, duration: 50.0, frequency: 1.0 }],
        stagger_time: 10.0,
        boxin_tightness: 0.9,
        boxin_duration: 7.0,
        rolling_block_tightness: 0.6,
        rolling_block_duration: 5.0,
        collapse_aggression: 0.7,
        collapse_speed: 15.0,
        collapse_inner: 3.0,
        collapse_outer: 15.0,
        max_cops_collapsing: 6,
    }]);
    police.tick_formations(0.0, 1, [0.0; 2]);
    police.tick_formations(9.0, 1, [0.0; 2]);
    assert_eq!(police.last_formation(), Some(Formation::StaggerFollow));
    police.tick_formations(1.0, 1, [0.0; 2]);
    assert_eq!(police.last_formation(), Some(Formation::BoxIn));
    assert_eq!(police.goal_tuning().boxin_duration, 7.0);
    assert_eq!(police.goal_tuning().rolling_block_tightness, 0.6);
    assert_eq!(police.collapse_tuning().speed_kmh, 15.0);
}

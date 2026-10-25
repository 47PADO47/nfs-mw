use super::*;

#[test]
fn the_gap_is_taken_at_the_latest_shared_checkpoint() {
    // The player passed three checkpoints, the rival two: the second one is the latest both passed.
    let (seconds, ahead) = checkpoint_gap(&[10.0, 21.0, 33.0], &[10.5, 22.42]).unwrap();
    assert!((seconds - 1.42).abs() < 1e-4);
    assert!(ahead, "the player was there first");
    let (seconds, ahead) = checkpoint_gap(&[10.0, 23.0], &[9.0, 22.15, 30.0]).unwrap();
    assert!((seconds - 0.85).abs() < 1e-4);
    assert!(!ahead, "the rival was there first");
}

#[test]
fn the_gap_is_never_negative_and_a_dead_heat_counts_as_ahead() {
    let (seconds, ahead) = checkpoint_gap(&[12.0], &[12.0]).unwrap();
    assert_eq!(seconds, 0.0);
    assert!(ahead);
    for (mine, theirs) in [(5.0, 9.0), (9.0, 5.0)] {
        assert!(checkpoint_gap(&[mine], &[theirs]).unwrap().0 >= 0.0);
    }
}

#[test]
fn there_is_no_gap_before_the_first_shared_checkpoint() {
    assert_eq!(checkpoint_gap(&[], &[]), None);
    assert_eq!(checkpoint_gap(&[10.0, 20.0], &[]), None, "the rival has passed none yet");
    assert_eq!(checkpoint_gap(&[], &[10.0]), None, "the player has passed none yet");
}

#[test]
fn the_rival_is_a_place_ahead_or_for_the_leader_a_place_behind() {
    // Racer 0 is the player; racers' places by index.
    let places = [3, 1, 2, 4];
    let out = [false; 4];
    assert_eq!(rival_of(&places, 0, &out), Some(2), "third chases second");
    let leading = [1, 3, 2, 4];
    assert_eq!(rival_of(&leading, 0, &out), Some(2), "the leader is measured against second");
}

#[test]
fn a_rival_that_is_out_or_missing_gives_none() {
    assert_eq!(rival_of(&[2, 1], 0, &[false, true]), None, "the racer ahead was knocked out");
    assert_eq!(rival_of(&[1], 0, &[false]), None, "a race of one has no rival");
    assert_eq!(rival_of(&[], 0, &[]), None);
}

#[test]
fn drag_speed_trap_tollbooth_and_challenges_hide_the_gap() {
    for kind in [RaceType::Drag, RaceType::SpeedTrap, RaceType::Tollbooth, RaceType::Challenge] {
        assert!(!shows_rival_gap(kind), "{kind:?}");
    }
    for kind in [RaceType::Sprint, RaceType::Circuit, RaceType::Knockout] {
        assert!(shows_rival_gap(kind), "{kind:?}");
    }
}

#[test]
fn outside_a_race_there_is_no_race_info() {
    assert_eq!(crate::hud::HudState::default().race_info, None);
}

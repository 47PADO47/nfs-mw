//! Tests for the mode resolver, the mode clock's crossfade and the detail level.

use super::{Detail, HudMode, ModeClock, resolve};
use crate::hud::{Count, HudContext, HudState, PursuitPhase, RaceHud, RaceStage};
use crate::settings::hud::{HudPreset, HudSettings};

fn race(count: Option<Count>) -> RaceHud {
    RaceHud { place: 2, racers: 4, lap: Some((1, 3)), clock: 12.0, progress: 0.2, count }
}

fn state(stage: Option<RaceStage>, race: Option<RaceHud>, phase: PursuitPhase) -> HudState {
    HudState {
        race,
        pursuit_phase: phase,
        context: HudContext { race_stage: stage, ..HudContext::default() },
        ..HudState::default()
    }
}

#[test]
fn free_roam_without_a_race_or_a_chase() {
    assert_eq!(resolve(&HudState::default()), HudMode::FreeRoam);
}

#[test]
fn each_race_stage_has_its_mode() {
    let intro = state(Some(RaceStage::Intro), None, PursuitPhase::None);
    assert_eq!(resolve(&intro), HudMode::RaceIntro, "the intro, before the readout is up");
    let counting = state(Some(RaceStage::Running), Some(race(Some(Count::Two))), PursuitPhase::None);
    assert_eq!(resolve(&counting), HudMode::RaceIntro, "the countdown is part of the intro");
    let go = state(Some(RaceStage::Running), Some(race(Some(Count::Go))), PursuitPhase::None);
    assert_eq!(resolve(&go), HudMode::RaceIntro, "GO is the countdown's last step");
    let running = state(Some(RaceStage::Running), Some(race(None)), PursuitPhase::None);
    assert_eq!(resolve(&running), HudMode::Racing);
    let results = state(Some(RaceStage::Results), Some(race(None)), PursuitPhase::None);
    assert_eq!(resolve(&results), HudMode::RaceResults);
}

#[test]
fn a_race_readout_without_a_stage_runs() {
    assert_eq!(resolve(&state(None, Some(race(None)), PursuitPhase::None)), HudMode::Racing);
    assert_eq!(resolve(&state(None, Some(race(Some(Count::Three))), PursuitPhase::None)), HudMode::RaceIntro);
}

#[test]
fn a_chase_in_a_race_keeps_the_race_layout() {
    for phase in [PursuitPhase::Active, PursuitPhase::Search, PursuitPhase::Cooldown, PursuitPhase::Busted] {
        let chased = state(Some(RaceStage::Running), Some(race(None)), phase);
        assert_eq!(resolve(&chased), HudMode::Racing, "{phase:?}");
    }
}

#[test]
fn every_chase_phase_is_its_own_pursuit_mode() {
    for phase in [
        PursuitPhase::Active,
        PursuitPhase::Search,
        PursuitPhase::Cooldown,
        PursuitPhase::Escaped,
        PursuitPhase::Busted,
    ] {
        let mode = resolve(&state(None, None, phase));
        assert_eq!(mode, HudMode::Pursuit(phase));
        assert_eq!(mode.pursuit_phase(), Some(phase));
    }
    assert_eq!(resolve(&state(None, None, PursuitPhase::None)), HudMode::FreeRoam, "no chase, no pursuit layout");
}

#[test]
fn hidden_wins_over_everything() {
    let mut busy = state(Some(RaceStage::Running), Some(race(None)), PursuitPhase::Active);
    busy.visible = false;
    assert_eq!(resolve(&busy), HudMode::Hidden);
    let hidden_chase = HudState { visible: false, ..state(None, None, PursuitPhase::Cooldown) };
    assert_eq!(resolve(&hidden_chase), HudMode::Hidden);
}

#[test]
fn the_clock_fades_an_element_in_with_its_mode_and_out_with_the_last() {
    let racing = |mode: HudMode| mode == HudMode::Racing;
    let mut clock = ModeClock::default();
    assert!(clock.update(HudMode::FreeRoam, 1.0));
    assert!(!clock.update(HudMode::FreeRoam, 2.0), "the same mode is no change");
    assert!(clock.update(HudMode::Racing, 10.0));
    assert_eq!(clock.mode(), HudMode::Racing);
    assert_eq!(clock.opacity(racing, 10.0, 0.3), 0.0, "fades in from nothing");
    assert!((clock.opacity(racing, 10.15, 0.3) - 0.5).abs() < 1e-5, "half way");
    assert_eq!(clock.opacity(racing, 11.0, 0.3), 1.0);
    clock.update(HudMode::RaceResults, 20.0);
    assert_eq!(clock.opacity(racing, 20.0, 0.3), 1.0, "fades out from full");
    assert!(clock.opacity(racing, 20.3, 0.3) < 1e-4, "gone after the crossfade");
}

#[test]
fn an_element_shared_by_both_modes_does_not_fade() {
    let chase = |mode: HudMode| mode.pursuit_phase().is_some();
    let mut clock = ModeClock::default();
    clock.update(HudMode::Pursuit(PursuitPhase::Active), 1.0);
    clock.update(HudMode::Pursuit(PursuitPhase::Search), 5.0);
    assert_eq!(clock.opacity(chase, 5.0, 0.3), 1.0, "Active to Search keeps the panel");
    assert_eq!(clock.age(5.25), 0.25);
    assert_eq!(clock.opacity(|mode| mode == HudMode::FreeRoam, 5.0, 0.3), 0.0, "in neither mode");
}

#[test]
fn the_detail_follows_the_preset() {
    let of = |preset| Detail::of(&HudSettings { preset, ..HudSettings::default() });
    assert_eq!(of(HudPreset::Minimal), Detail::Minimal);
    assert_eq!(of(HudPreset::Balanced), Detail::Balanced);
    assert_eq!(of(HudPreset::Advanced), Detail::Advanced);
    assert!(Detail::Minimal < Detail::Balanced && Detail::Balanced < Detail::Advanced);
}

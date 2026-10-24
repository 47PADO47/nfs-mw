use super::common::CAR;
use super::*;
use crate::hud::{ConditionWarning, HEAVY_DAMAGE, HudEvent, PursuitPhase, TireCondition, TurnDirection};
use crate::settings::hud::HudSettings;
use crate::settings::{Partial, Settings};

fn first(scenario: HudScenario) -> HudState {
    scenario.state(0, VIEWPORT)
}

#[test]
fn every_scenario_builds_on_any_frame() {
    for scenario in HudScenario::ALL {
        for frame in [0, 1, PREEMPT_FRAME, 10_000] {
            let state = scenario.state(frame, VIEWPORT);
            assert!(state.visible, "{scenario}: the HUD shows");
            assert!(state.minimap.is_some() && state.camera.is_some(), "{scenario}: a map place and a camera");
        }
        // A window of no height still builds a camera.
        scenario.state(0, [800.0, 0.0]);
    }
}

#[test]
fn the_names_parse_back_and_are_unique() {
    for scenario in HudScenario::ALL {
        assert_eq!(scenario.name().parse::<HudScenario>(), Ok(scenario));
        assert_eq!(scenario.to_string(), scenario.name());
    }
    assert_eq!(" Cooldown ".parse::<HudScenario>(), Ok(HudScenario::Cooldown), "case and spaces do not matter");
    let mut names: Vec<&str> = HudScenario::ALL.iter().map(|s| s.name()).collect();
    names.sort_unstable();
    names.dedup();
    assert_eq!(names.len(), 12);
}

#[test]
fn an_unknown_name_is_an_error_that_lists_the_names() {
    let error = "warp-speed".parse::<HudScenario>().unwrap_err();
    assert!(
        error.contains("warp-speed") && error.contains("notify-preempt") && error.contains("map-expanded"),
        "{error}"
    );
    assert!("".parse::<HudScenario>().is_err());
}

#[test]
fn the_same_frame_gives_the_same_state() {
    for scenario in HudScenario::ALL {
        assert_eq!(scenario.state(3, VIEWPORT), scenario.state(3, VIEWPORT), "{scenario}");
    }
}

#[test]
fn events_are_new_on_their_frame_only() {
    for scenario in HudScenario::ALL.into_iter().filter(|s| *s != HudScenario::NotifyPreempt) {
        let later = scenario.state(1, VIEWPORT);
        assert!(later.events.is_empty(), "{scenario}: the opening events come once");
    }
    let frames: Vec<u64> = (0..4).map(|frame| HudScenario::Racing.state(frame, VIEWPORT).events_frame).collect();
    assert_eq!(frames, [1, 2, 3, 4], "every frame is a new events frame, and none is the default 0");
}

#[test]
fn free_roam_cruises_with_no_race_and_no_chase() {
    let state = first(HudScenario::FreeRoam);
    assert_eq!(state.display_speed(), 128);
    assert_eq!((state.gear, state.nos), (4, 0.5));
    assert!(state.race.is_none() && state.pursuit.is_none() && state.route.is_none());
    assert_eq!(state.pursuit_phase, PursuitPhase::None);
    assert!(state.markers.iter().any(|marker| !marker.discovered), "a place to discover");
    assert_eq!(state.minimap.unwrap().position, CAR);
}

#[test]
fn the_race_intro_holds_the_countdown_at_three() {
    let state = first(HudScenario::RaceIntro);
    let race = state.race.unwrap();
    assert_eq!(race.count, Some(crate::hud::Count::Three));
    assert_eq!((race.racers, race.lap), (6, Some((1, 3))));
    assert_eq!(state.leaders.unwrap().racers, 6);
    assert_eq!(state.race_info.unwrap().gate, Some(1));
}

#[test]
fn racing_is_second_on_lap_two_with_the_leader_ahead() {
    let state = first(HudScenario::Racing);
    let race = state.race.unwrap();
    assert_eq!((race.place, race.lap, race.count), (2, Some((2, 3)), None));
    assert!((race.progress - 0.55).abs() < 1e-6);
    let info = state.race_info.unwrap();
    assert_eq!((info.remaining_m, info.gate, info.gates), (Some(4200.0), Some(15), 24));
    let gap = info.rival_gap.unwrap();
    assert!((gap.seconds - 1.42).abs() < 1e-6 && !gap.ahead);
    let leaders = state.leaders.unwrap();
    assert_eq!(leaders.rows[0].name, gap.name, "the rival is the leader");
    assert_eq!(leaders.player, Some(1));
    assert!(state.events.contains(&HudEvent::Checkpoint { number: 7, total: 8 }));
}

#[test]
fn the_results_have_every_gate_passed() {
    let state = first(HudScenario::RaceResults);
    assert_eq!(state.race.unwrap().progress, 1.0);
    let info = state.race_info.unwrap();
    assert_eq!((info.next_gate, info.gate, info.remaining_m), (None, None, None));
}

#[test]
fn the_active_chase_places_only_what_is_in_sight() {
    let state = first(HudScenario::PursuitActive);
    assert_eq!(state.pursuit_phase, PursuitPhase::Active);
    let intel = &state.police_intel;
    assert_eq!(intel.sightings.len(), 2);
    let helicopter = intel.helicopter.unwrap();
    assert!(helicopter.in_sight && helicopter.position.is_some());
    assert!(intel.roadblock_reported && intel.roadblocks.len() == 1);
    assert!((state.pursuit.unwrap().secs - 134.0).abs() < 1e-6);
    assert!(state.events.contains(&HudEvent::HeatUp { level: 3 }));
}

#[test]
fn the_search_knows_only_the_last_known_place() {
    let state = first(HudScenario::Search);
    assert_eq!(state.pursuit_phase, PursuitPhase::Search);
    assert!(state.police_intel.sightings.is_empty());
    assert_eq!(state.police_intel.last_known.unwrap().age_secs, 3.0);
    assert_eq!(state.police_intel.helicopter.unwrap().position, None, "a blind helicopter is not placed");
    assert!(!state.pursuit.unwrap().cooldown);
    assert!(state.events.contains(&HudEvent::ContactLost));
}

#[test]
fn the_cooldown_shows_the_meter_and_the_hiding_spot() {
    let state = first(HudScenario::Cooldown);
    assert_eq!(state.pursuit_phase, PursuitPhase::Cooldown);
    let pursuit = state.pursuit.unwrap();
    assert!(pursuit.cooldown, "the pursuit cool-down is shown");
    assert!((pursuit.evade - 0.4).abs() < 1e-6);
    assert_eq!(pursuit.hiding, Some(0.6));
    assert!(state.events.contains(&HudEvent::CooldownStarted));
}

#[test]
fn the_escape_holds_the_escaped_phase() {
    let state = first(HudScenario::Escape);
    assert_eq!(state.pursuit_phase, PursuitPhase::Escaped);
    assert!((state.pursuit.unwrap().secs - 245.0).abs() < 1e-6);
    assert!(state.events.contains(&HudEvent::Escaped));
}

#[test]
fn critical_damage_has_a_blown_and_a_punctured_tyre_and_heavy_damage() {
    let condition = first(HudScenario::CriticalDamage).condition.unwrap();
    assert_eq!(condition.warning(), Some(ConditionWarning::TireBlown));
    assert_eq!(condition.damaged_tires(), 2);
    assert_eq!(condition.tires[2], TireCondition::Blown);
    assert!(condition.cosmetic.unwrap() >= HEAVY_DAMAGE && !condition.disabled);
}

#[test]
fn notify_preempt_steps_a_critical_event_in_after_the_first_ones() {
    let opening = first(HudScenario::NotifyPreempt).events;
    assert_eq!(opening.len(), 3);
    assert!(matches!(opening[0], HudEvent::BountyMilestone { .. }), "the Low notice comes first");
    let preempt = HudScenario::NotifyPreempt.state(PREEMPT_FRAME, VIEWPORT).events;
    assert_eq!(preempt, [HudEvent::RoadblockReported]);
    for frame in (1..PREEMPT_FRAME).chain([PREEMPT_FRAME + 1]) {
        assert!(HudScenario::NotifyPreempt.state(frame, VIEWPORT).events.is_empty(), "frame {frame}");
    }
}

#[test]
fn the_map_scenarios_share_a_route_with_a_left_turn_next() {
    let centred = first(HudScenario::MapCentered);
    assert_eq!(centred, first(HudScenario::MapExpanded), "only the map mode differs");
    let route = centred.route.unwrap();
    let turn = route.next_turn.unwrap();
    assert_eq!(turn.direction, TurnDirection::Left);
    assert!((turn.distance_m - 300.0).abs() < 1e-3);
    assert_eq!(route.path.last(), Some(&route.target));
}

#[test]
fn the_settings_pin_the_hud_and_take_the_map_mode() {
    let defaults = Settings::from(Partial::default());
    let mine = Settings {
        hud: false,
        speed_units: crate::settings::SpeedUnits::Imperial,
        hud_options: HudSettings { high_contrast: true, ..HudSettings::default() },
        ..defaults
    };
    let expanded = HudScenario::MapExpanded.settings(mine);
    assert_eq!(expanded.hud_options.map_mode, MapMode::Expanded);
    assert!(expanded.hud && !expanded.hud_options.high_contrast, "the player's options are put back to the defaults");
    assert_eq!(expanded.speed_units, crate::settings::SpeedUnits::Metric);
    assert_eq!(HudScenario::MapCentered.settings(mine).hud_options.map_mode, MapMode::Centered);
    let cooldown = HudScenario::Cooldown.settings(mine);
    assert_eq!(cooldown.hud_options, HudSettings::default());
}

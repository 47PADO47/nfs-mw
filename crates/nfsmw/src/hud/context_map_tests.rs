use super::*;
use crate::settings::{MinimapMode, Partial};
use blackbox_minimap::Orientation;

#[test]
fn batch18_hud_map_mode_follows_race_context_and_restores_free_roam() {
    let settings = Settings::from(Partial {
        free_roam_map_mode: Some(MinimapMode::Off),
        race_map_mode: Some(MinimapMode::Rotating),
        ..Partial::default()
    });
    let at = MapPosition { position: [10.0, 20.0], heading: [1.0, 0.0], orientation: Orientation::North };
    let mut state = HudState { minimap: Some(at), ..HudState::default() };
    apply_map_mode(&mut state, &settings);
    assert!(state.minimap.is_none(), "free-roam override hides the minimap");
    assert_eq!(state.map_car, Some(at), "Custom HUD retains its independent map input");
    for source in [MapSource::Event("test".into()), MapSource::Variant(nfsmw_career::regions::MapVariant::Full)] {
        state.map_source = source;
        state.minimap = Some(at);
        apply_map_mode(&mut state, &settings);
        assert_eq!(state.minimap.unwrap().orientation, Orientation::Heading);
    }
    state.map_source = MapSource::Stage;
    state.minimap = Some(at);
    apply_map_mode(&mut state, &settings);
    assert!(state.minimap.is_none(), "leaving the race returns to the free-roam mode");
}

#[test]
fn batch18_race_intro_uses_race_mode_before_event_tiles_and_readouts_exist() {
    let settings = Settings::from(Partial {
        free_roam_map_mode: Some(MinimapMode::Off),
        race_map_mode: Some(MinimapMode::Rotating),
        ..Partial::default()
    });
    let at = MapPosition { position: [0.0, 0.0], heading: [1.0, 0.0], orientation: Orientation::North };
    let mut state = HudState { minimap: Some(at), ..HudState::default() };
    state.context.race_stage = Some(super::super::state::RaceStage::Intro);
    apply_map_mode(&mut state, &settings);
    assert_eq!(state.minimap.unwrap().orientation, Orientation::Heading);
}

#[test]
fn batch18_drag_telemetry_selects_race_mode_without_an_event_map() {
    let settings = Settings::from(Partial {
        free_roam_map_mode: Some(MinimapMode::Fixed),
        race_map_mode: Some(MinimapMode::Off),
        ..Partial::default()
    });
    let mut state = HudState {
        drag: true,
        minimap: Some(MapPosition { position: [0.0, 0.0], heading: [1.0, 0.0], orientation: Orientation::North }),
        ..HudState::default()
    };
    apply_map_mode(&mut state, &settings);
    assert!(state.minimap.is_none());
    assert!(state.map_car.is_some());
}

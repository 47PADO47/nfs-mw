//! The HUD map's default bindings (`hud_map`) never share an input with a driving action, so opening or zooming the
//! map never accelerates, brakes, steers or shifts.

use super::{Action, Source, defaults, driving, hud_map};

const MAP_ACTIONS: [Action; 6] = [
    Action::MapMode,
    Action::MapExpand,
    Action::MapZoomIn,
    Action::MapZoomOut,
    Action::MapOrientation,
    Action::ToggleHud,
];

#[test]
fn no_map_default_shares_an_input_with_a_driving_action() {
    let driving = driving();
    for map in hud_map() {
        let clash = driving.iter().find(|drive| drive.source == map.source);
        assert!(clash.is_none(), "{:?} on {:?} collides with {:?}", map.action, map.source, clash.map(|b| b.action));
    }
}

#[test]
fn the_map_keys_are_bound_to_nothing_else() {
    let all = defaults();
    for map in hud_map().into_iter().filter(|b| matches!(b.source, Source::Key(_))) {
        let others: Vec<Action> =
            all.iter().filter(|b| b.source == map.source && b.action != map.action).map(|b| b.action).collect();
        assert!(others.is_empty(), "{:?} shares {:?} with {others:?}", map.action, map.source);
    }
}

#[test]
fn the_map_pad_buttons_share_only_with_the_free_camera_and_the_menus() {
    let all = defaults();
    for map in hud_map().into_iter().filter(|b| matches!(b.source, Source::PadButton(_))) {
        for other in all.iter().filter(|b| b.source == map.source && b.action != map.action) {
            let elsewhere = matches!(other.action, Action::Zoom | Action::MenuDown | Action::MenuRight);
            assert!(elsewhere, "{:?} shares {:?} with {:?}", map.action, map.source, other.action);
        }
    }
}

#[test]
fn every_map_action_has_a_default_and_a_config_name() {
    let bound = hud_map();
    for action in MAP_ACTIONS {
        assert!(bound.iter().any(|b| b.action == action), "{action:?} has no default");
        assert_eq!(Action::parse(action.name()), Ok(action));
        assert!(defaults().iter().any(|b| b.action == action), "{action:?} is in the default layout");
    }
    assert_eq!(Action::MapMode.name(), "map_mode");
    assert_eq!(Action::ToggleHud.name(), "toggle_hud");
}

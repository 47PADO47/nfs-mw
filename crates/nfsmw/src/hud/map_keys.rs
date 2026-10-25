//! The Custom map's keys (`input/action.rs`: `MapMode`, `MapExpand`, `MapZoomIn`, `MapZoomOut`, `MapOrientation`,
//! `ToggleHud`), applied live to the settings by [`MapKeys`](crate::ui::theme::custom::map::modes::MapKeys).
//!
//! They work while a scene shows its HUD, and never while a menu or the console has the input (the action state reads
//! as released then). The settings change for the session only: nothing is written to the config file.

use bevy_ecs::prelude::*;

use super::state::HudState;
use crate::input::{Action, ActionState};
use crate::settings::Settings;
use crate::ui::theme::custom::map::modes::{MapKeys, MapPresses};

/// Applies this frame's map key presses to the settings.
pub(super) fn apply(
    actions: Option<Res<ActionState>>,
    state: Res<HudState>,
    mut settings: ResMut<Settings>,
    mut keys: Local<MapKeys>,
) {
    let Some(actions) = actions else { return };
    if !state.visible {
        return;
    }
    let presses = MapPresses {
        mode: actions.just_pressed(Action::MapMode),
        expand: actions.just_pressed(Action::MapExpand),
        zoom_in: actions.just_pressed(Action::MapZoomIn),
        zoom_out: actions.just_pressed(Action::MapZoomOut),
        orientation: actions.just_pressed(Action::MapOrientation),
        toggle_hud: actions.just_pressed(Action::ToggleHud),
    };
    // Only a press touches the settings, so their change detection fires only then.
    if !presses.any() {
        return;
    }
    keys.apply(presses, &mut settings);
}

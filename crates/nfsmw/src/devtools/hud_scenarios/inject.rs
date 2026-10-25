//! How a scenario reaches the HUD: the resource a `--hud-scenario` run inserts, the settings the run starts with, the
//! system that writes the fixture over the scene's state each frame, and the label drawn over the frame. Nothing here
//! runs without the resource (see the module above).

use bevy_ecs::prelude::*;
use bevy_window::{PrimaryWindow, Window};
use egui::{Align2, Color32, RichText};

use super::HudScenario;
use super::common::VIEWPORT;
use crate::hud::{HudState, MapPosition};
use crate::settings::hud::HudSettings;
use crate::settings::{MinimapMode, Settings, SpeedUnits};

/// The scenario of a `--hud-scenario` run. Inserted at startup only when the flag is given; its presence is what turns
/// the fixture on.
#[derive(Resource, Debug, Clone, Copy, PartialEq, Eq)]
pub struct ActiveScenario(pub HudScenario);

impl HudScenario {
    /// The settings a run of this scenario starts with: `settings` with the HUD on, km/h, the map north up, and the
    /// Custom HUD's options at their defaults with the scenario's map mode on top, so the same scenario gives the same
    /// frame whatever the config file says. Console commands (`--exec`) still change them once the run is up.
    pub fn settings(self, settings: Settings) -> Settings {
        let mut hud_options = HudSettings::default();
        if let Some(mode) = self.map_mode() {
            hud_options.map_mode = mode;
        }
        // The free-roam picture shows the optional cash and bounty line too.
        hud_options.free_roam_extras = self == HudScenario::FreeRoam;
        Settings { hud: true, speed_units: SpeedUnits::Metric, minimap: MinimapMode::Fixed, hud_options, ..settings }
    }
}

/// Writes the scenario's state for this frame over the one the scene supplied. The HUD plugin runs it after its sync and
/// before the notifications take the frame's events, only while the HUD is up and [`ActiveScenario`] exists. The frame
/// count starts at the first frame it runs. The player's unit and map orientation still apply, as the sync applies them.
pub fn apply(
    scenario: Res<ActiveScenario>,
    settings: Res<Settings>,
    windows: Query<&Window, With<PrimaryWindow>>,
    mut frame: Local<u64>,
    mut state: ResMut<HudState>,
) {
    let viewport = windows.single().map_or(VIEWPORT, |window| [window.width(), window.height()]);
    let mut fixture = scenario.0.state(*frame, viewport);
    *frame += 1;
    fixture.use_mph = settings.speed_units == SpeedUnits::Imperial;
    let orientation = settings.minimap.orientation();
    fixture.minimap = fixture.minimap.zip(orientation).map(|(at, orientation)| MapPosition { orientation, ..at });
    // The roads come from the loaded world, not the fixture, so the map draws the real network around the fixture's car.
    fixture.map_roads = state.map_roads.clone();
    *state = fixture;
}

/// Draws `SCENARIO: <name>` in the top-left corner, so a frame of fixture data is never taken for gameplay. The UI pass
/// calls it only while [`ActiveScenario`] exists.
pub fn label(ctx: &egui::Context, scenario: HudScenario) {
    let text = RichText::new(format!("SCENARIO: {}", scenario.name()))
        .monospace()
        .strong()
        .color(Color32::from_rgb(255, 196, 64))
        .background_color(Color32::from_black_alpha(170));
    egui::Area::new(egui::Id::new("hud_scenario_label"))
        .anchor(Align2::LEFT_TOP, [16.0, 16.0])
        .interactable(false)
        .show(ctx, |ui| {
            ui.label(text);
        });
}

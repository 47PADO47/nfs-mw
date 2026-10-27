//! F5 controls: preferences persist; testing profiles are explicitly loaded.
use super::{Action, State};
use crate::settings::{CameraMode, Settings};
mod camera;
mod places;
mod testing;

pub fn show(ctx: &egui::Context, open: &mut bool, state: &mut State, settings: &mut Settings) -> Vec<Action> {
    let mut actions = Vec::new();
    if !*open {
        return actions;
    }
    egui::Window::new("Extra options · F5")
        .open(open)
        .default_width(570.0)
        .max_height((ctx.content_rect().height() - 80.0).max(250.0))
        .show(ctx, |ui| {
            ui.horizontal_wrapped(|ui| {
                for (i, label) in ["Camera", "World & testing", "Locations & views", "Profiles"].into_iter().enumerate()
                {
                    ui.selectable_value(&mut state.tab, i, label);
                }
            });
            ui.separator();
            egui::ScrollArea::vertical().show(ui, |ui| {
                if !state.local_allowed {
                    ui.weak("Extra options require a local single-player driving world.");
                }
                ui.add_enabled_ui(state.local_allowed, |ui| match state.tab {
                    0 => camera::show(ui, state, settings, &mut actions),
                    1 => testing::show(ui, state, settings, &mut actions),
                    2 => places::show(ui, state, &mut actions),
                    _ => profiles(ui, state, &mut actions),
                });
                if let Some(message) = &state.message {
                    ui.separator();
                    ui.label(message);
                }
            });
        });
    actions
}
fn profiles(ui: &mut egui::Ui, state: &mut State, actions: &mut Vec<Action>) {
    ui.label("Testing overrides start disabled each launch. Load a profile to apply them.");
    ui.horizontal_wrapped(|ui| {
        if ui.button("Clean drive").clicked() {
            state.testing = super::Overrides { traffic_off: true, police_off: true, ..Default::default() };
        }
        if ui.button("Pursuit test").clicked() {
            state.testing = super::Overrides { lock_heat: true, heat: 3, unlimited_nos: true, ..Default::default() };
            actions.push(Action::StartPursuit);
        }
        if ui.button("Visual inspection").clicked() {
            state.testing =
                super::Overrides { traffic_off: true, police_off: true, paused: true, ..Default::default() };
        }
        if ui.button("Default gameplay").clicked() {
            actions.push(Action::Clear);
        }
    });
    match state.profiles.show_quick(ui, "testing-presets.toml") {
        Some(crate::preset_store::Action::Save) => {
            state.profiles.save(toml::to_string(&state.testing).map_err(Into::into))
        }
        Some(crate::preset_store::Action::Load) => {
            let result = state.profiles.load().and_then(|text| {
                let value: super::Overrides = toml::from_str(&text)?;
                value.validate()?;
                state.testing = value;
                Ok("Testing profile applied".into())
            });
            state.profiles.report(result);
        }
        None => {}
    }
    ui.separator();
    if ui.button("Reload preferences from disk").clicked() {
        actions.push(Action::Reload);
    }
    if ui.button("Clear all testing overrides").clicked() {
        actions.push(Action::Clear);
    }
    if let Some(path) = Settings::config_path() {
        ui.label("Active config:");
        ui.monospace(path.display().to_string());
        if ui.button("Open config folder").clicked()
            && let Some(parent) = path.parent()
        {
            let opener = match std::env::consts::OS {
                "windows" => "explorer.exe",
                "macos" => "open",
                _ => "xdg-open",
            };
            if let Err(e) = std::process::Command::new(opener).arg(parent).spawn() {
                state.message = Some(e.to_string());
            }
        }
    }
}
pub fn camera_name(mode: CameraMode) -> &'static str {
    match mode {
        CameraMode::Chase => "Remake chase",
        CameraMode::DynamicFov => "Remake chase · dynamic FOV",
        CameraMode::Bumper => "Bumper (original)",
        CameraMode::Hood => "Hood (original)",
        CameraMode::Close => "Close chase (original)",
        CameraMode::Far => "Far chase (original)",
        CameraMode::SuperFar => "Extra far chase (original extra)",
        CameraMode::Drift => "Drift (original extra)",
        CameraMode::Pursuit => "Pursuit (original extra)",
    }
}

//! Disk snapshots restore requests through the same live path on every renderer.
use super::{GraphicsLab, Settings, presets};
use anyhow::{Context, Result, ensure};

pub(super) fn restore(settings: &mut Settings, text: &str) -> Result<()> {
    let table: toml::Table = text.parse().context("Invalid graphics profile")?;
    if let Some(value) = table.get("traffic_tuning") {
        ensure!(crate::settings::TrafficPolicy::read(value, "F8 profile").is_some(), "Invalid traffic tuning");
    }
    if let Some(value) = table.get("detail") {
        ensure!(crate::settings::DetailSettings::read(value, "graphics profile").is_some(), "Invalid detail settings");
    }
    if let Some(value) = table.get("appearance") {
        ensure!(
            crate::settings::Appearance::read(value, "graphics profile").is_some(),
            "Invalid or unsupported appearance schema"
        );
    }
    if let Some(version) = table.get("profile_version") {
        ensure!(version.as_integer() == Some(1), "Unsupported graphics preset schema");
    }
    let partial = crate::settings::file::parse(text, "graphics profile");
    let mut snapshot = Settings::from(partial.or(settings.graphics_layer()));
    snapshot.appearance.lighting.freeze_sun = settings.appearance.lighting.freeze_sun;
    snapshot.appearance.lighting.lighting_debug = settings.appearance.lighting.lighting_debug;
    presets::restore_graphics(settings, &snapshot);
    Ok(())
}

fn payload(settings: &Settings) -> Result<String> {
    crate::settings::write::merge("profile_version=1", &settings.graphics_layer())
}

pub(super) fn show(ui: &mut egui::Ui, lab: &mut GraphicsLab, settings: &mut Settings) {
    // Lighting widgets keep a draft for the current frame. Snapshot that draft too.
    settings.appearance.lighting = lab.options.sanitized();
    let action = lab.preset_picker.show(ui, "graphics-presets.toml");
    match action {
        Some(crate::preset_store::Action::Save) => lab.preset_picker.save(payload(settings)),
        Some(crate::preset_store::Action::Load) => {
            let result = lab
                .preset_picker
                .load()
                .and_then(|text| restore(settings, &text))
                .map(|()| "Graphics preset loaded. Save profile to use it on startup.".into());
            lab.options = settings.appearance.lighting;
            lab.preset_picker.report(result);
        }
        None => {}
    }
}

#[cfg(test)]
mod tests;

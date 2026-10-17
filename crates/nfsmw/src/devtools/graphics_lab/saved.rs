//! Disk snapshots restore requests through the same live path on every renderer.
use super::{Settings, presets};
use anyhow::{Context, Result, ensure};

pub(super) fn restore(settings: &mut Settings, text: &str) -> Result<()> {
    let table: toml::Table = text.parse().context("Invalid graphics profile")?;
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

//! Disk snapshots restore requests through the same live path on every renderer.
use super::{GraphicsLab, Settings, presets};
use anyhow::{Context, Result, ensure};

#[derive(Default)]
pub(super) struct Presets {
    names: Vec<String>,
    selected: String,
    name: String,
    naming: bool,
    initialized: bool,
    startup: Option<crate::settings::Partial>,
}

fn snapshot(settings: &Settings) -> crate::settings::Partial {
    let mut settings = *settings;
    settings.appearance.lighting.freeze_sun = false;
    settings.appearance.lighting.lighting_debug = Default::default();
    settings.graphics_layer()
}

fn preset_path(lab: &GraphicsLab) -> Result<std::path::PathBuf> {
    Ok(lab.profile_path.as_ref().context("No settings folder")?.with_file_name("graphics-presets.toml"))
}

fn reload_startup(lab: &mut GraphicsLab, settings: &mut Settings) -> Result<()> {
    let text = std::fs::read_to_string(lab.profile_path.as_ref().context("No settings folder")?)?;
    // Missing fields in an older startup file mean their launch-time values,
    // rather than whatever unsaved experiments the user is currently previewing.
    let mut loaded = lab
        .disk
        .startup
        .as_ref()
        .map(|layer| Settings::from(*layer))
        .or(lab.original.map(|(_, settings)| settings))
        .unwrap_or_else(|| Settings::from(crate::settings::Partial::default()));
    restore(&mut loaded, &text)?;
    loaded.appearance.lighting.freeze_sun = settings.appearance.lighting.freeze_sun;
    loaded.appearance.lighting.lighting_debug = settings.appearance.lighting.lighting_debug;
    presets::restore_graphics(settings, &loaded);
    lab.options = settings.appearance.lighting;
    lab.disk.startup = Some(snapshot(settings));
    Ok(())
}

pub(super) fn save_startup(lab: &mut GraphicsLab, settings: &Settings) {
    let result = lab
        .profile_path
        .as_deref()
        .context("No settings folder")
        .and_then(|path| crate::settings::write_file(path, &settings.graphics_layer()));
    lab.message = Some(match result {
        Ok(()) => {
            lab.disk.startup = Some(snapshot(settings));
            "Saved for next launch.".into()
        }
        Err(error) => format!("Could not save startup graphics: {error:#}"),
    });
}

pub(super) fn restore(settings: &mut Settings, text: &str) -> Result<()> {
    let mut table: toml::Table = text.parse().context("Invalid graphics profile")?;
    if let Some(value) = table.get("traffic_tuning") {
        ensure!(crate::settings::TrafficPolicy::read(value, "F8 profile").is_some(), "Invalid traffic tuning");
    }
    if let Some(value) = table.get("pursuit_options") {
        ensure!(crate::settings::PursuitOptions::read(value, "F8 profile").is_some(), "Invalid pursuit options");
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
    if let Some(appearance) = table.get_mut("appearance").and_then(toml::Value::as_table_mut) {
        let baseline = settings.appearance.value()?;
        fill_missing(appearance, baseline.as_table().context("Invalid current appearance")?);
    }
    let merged = toml::to_string(&table)?;
    let partial = crate::settings::file::parse(&merged, "graphics profile");
    let mut snapshot = Settings::from(partial.or(settings.graphics_layer()));
    snapshot.appearance.lighting.freeze_sun = settings.appearance.lighting.freeze_sun;
    snapshot.appearance.lighting.lighting_debug = settings.appearance.lighting.lighting_debug;
    presets::restore_graphics(settings, &snapshot);
    Ok(())
}

fn fill_missing(table: &mut toml::Table, baseline: &toml::Table) {
    for (key, value) in baseline {
        if let Some(old) = table.get_mut(key) {
            if let (Some(old), Some(value)) = (old.as_table_mut(), value.as_table()) {
                fill_missing(old, value);
            }
            continue;
        }
        table.insert(key.clone(), value.clone());
    }
}

fn payload(settings: &Settings) -> Result<String> {
    crate::settings::write::merge("profile_version=1", &settings.graphics_layer())
}

fn save_named(lab: &GraphicsLab, settings: &Settings) -> Result<()> {
    let path = preset_path(lab)?;
    let name = lab.disk.name.trim();
    let text = match crate::preset_store::names(&path)?.iter().any(|entry| entry == name) {
        true => {
            let previous = crate::preset_store::load(&path, name)?;
            // Validate before merging; retain fields from newer builds with the same schema.
            let mut validation = *settings;
            restore(&mut validation, &previous)?;
            crate::settings::write::merge(&previous, &settings.graphics_layer())?
        }
        false => payload(settings)?,
    };
    crate::preset_store::save(&path, name, &text)
}

pub(super) fn show(ui: &mut egui::Ui, lab: &mut GraphicsLab, settings: &mut Settings) {
    let mut focus_name = false;
    if !lab.disk.initialized {
        lab.disk.initialized = true;
        let mut saved = *settings;
        let _ = reload_startup(lab, &mut saved);
        lab.options = settings.appearance.lighting;
        refresh(lab);
    }
    ui.horizontal_wrapped(|ui| {
        if ui
            .button("Save for startup")
            .on_hover_text(
                "Ctrl+S: keep current graphics settings for the next normal launch. Named snapshots stay unchanged.",
            )
            .clicked()
        {
            lab.save_requested = true;
        }
        if ui
            .button("Revert saved")
            .on_hover_text("Reloads the last saved startup graphics. Driving, weather and audio stay unchanged.")
            .clicked()
        {
            let result = reload_startup(lab, settings);
            lab.message = Some(match result {
                Ok(()) => "Saved startup graphics restored.".into(),
                Err(error) => format!("Cannot restore startup graphics: {error:#}"),
            });
        }
        ui.weak(match lab.disk.startup.as_ref() == Some(&snapshot(settings)) {
            true => "Saved for next launch",
            false => "Unsaved changes",
        });
    });
    ui.horizontal_wrapped(|ui| {
        ui.label("Saved graphics:").on_hover_text("Named snapshots restore saved graphics choices, including reflection range, resolution and lighting. They are separate from rendering-quality presets.");
        let mut load = false;
        egui::ComboBox::from_id_salt("graphics_snapshot")
            .selected_text(match lab.disk.selected.is_empty() {
                true => "Load preset…",
                false => &lab.disk.selected,
            })
            .show_ui(ui, |ui| {
                for name in &lab.disk.names {
                    // Clicking the selected entry also reloads it, useful after experiments.
                    if ui.selectable_label(lab.disk.selected == *name, name).clicked() {
                        lab.disk.selected = name.clone();
                        load = true;
                    }
                }
            });
        if load {
            load_selected(lab, settings);
        }
        if ui.button("Save as…").on_hover_text("Stores all current graphics choices as a named snapshot and saves them for startup.").clicked() {
            lab.disk.naming = !lab.disk.naming;
            lab.disk.name = lab.disk.selected.clone();
            focus_name = lab.disk.naming;
        }
        if ui.small_button("↻").on_hover_text("Refresh presets from disk").clicked() {
            refresh(lab);
        }
    });
    if lab.disk.naming {
        ui.horizontal(|ui| {
            let name =
                ui.add(egui::TextEdit::singleline(&mut lab.disk.name).hint_text("Preset name").desired_width(220.0));
            if focus_name {
                name.request_focus();
            }
            let enter = name.lost_focus() && ui.input(|input| input.key_pressed(egui::Key::Enter));
            if ui.button("Save preset + startup").clicked() || enter {
                let result = save_named(lab, settings);
                match result {
                    Ok(()) => {
                        lab.disk.selected = lab.disk.name.trim().into();
                        lab.disk.naming = false;
                        lab.save_requested = true;
                        refresh(lab);
                    }
                    Err(error) => lab.message = Some(format!("Could not save preset: {error:#}")),
                }
            }
            if ui.button("Cancel").clicked() {
                lab.disk.naming = false;
            }
        });
    }
}

fn refresh(lab: &mut GraphicsLab) {
    match preset_path(lab).and_then(|path| crate::preset_store::names(&path)) {
        Ok(names) => lab.disk.names = names,
        Err(error) => lab.message = Some(format!("Cannot read presets: {error:#}")),
    }
}

fn load_selected(lab: &mut GraphicsLab, settings: &mut Settings) {
    let result = preset_path(lab)
        .and_then(|path| crate::preset_store::load(&path, &lab.disk.selected))
        .and_then(|text| restore(settings, &text));
    lab.message = Some(match result {
        Ok(()) => format!("Loaded {}. Save for startup to keep it next launch.", lab.disk.selected),
        Err(error) => format!("Cannot load preset: {error:#}"),
    });
    lab.options = settings.appearance.lighting;
}

#[cfg(test)]
mod tests;

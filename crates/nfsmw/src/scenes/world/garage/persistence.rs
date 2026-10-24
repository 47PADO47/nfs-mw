//! Native customization saves plus versioned extensions for RGB and mod fields.
use anyhow::{Context, Result, ensure};
use blackbox_gfx::RenderBackend;
use nfsmw_data::car::CarTables;

use super::{WorldScene, records};
use crate::career::ProfileDir;
use crate::garage::{Model, Selection, presets};

fn path(profiles: &ProfileDir, alias: &str) -> Result<std::path::PathBuf> {
    Ok(profiles.mod_data_dir(alias)?.join("garage.toml"))
}

fn extension_payload(
    profiles: &ProfileDir,
    alias: &str,
    handle: u32,
    record: &nfsmw_career::stable::CustomizationRecord,
    selection: &Selection,
) -> Result<String> {
    let mut table = toml::Table::new();
    table.insert("record".into(), records::fingerprint(record).into());
    table.insert("selection".into(), presets::encode(selection)?.into());
    crate::preset_store::updated(&path(profiles, alias)?, &format!("{handle:08x}"), &toml::to_string(&table)?)
}

#[cfg(test)]
fn save_extension(
    profiles: &ProfileDir,
    alias: &str,
    handle: u32,
    record: &nfsmw_career::stable::CustomizationRecord,
    selection: &Selection,
) -> Result<()> {
    let file = path(profiles, alias)?;
    std::fs::create_dir_all(file.parent().context("Missing extension folder")?)?;
    std::fs::write(file, extension_payload(profiles, alias, handle, record, selection)?)?;
    Ok(())
}

/// Replacing the native file is the commit point. A failed native write restores the prior extension.
pub(super) fn save_purchase(
    profiles: &ProfileDir,
    alias: &str,
    handle: u32,
    record: &nfsmw_career::stable::CustomizationRecord,
    selection: &Selection,
    native: &[u8],
) -> Result<()> {
    profiles.recover_extensions(alias)?;
    let extension = extension_payload(profiles, alias, handle, record, selection)?;
    profiles.write_with_extension(alias, native, "garage.toml", extension.as_bytes())
}

fn extension(
    profiles: &ProfileDir,
    alias: &str,
    handle: u32,
    record: &nfsmw_career::stable::CustomizationRecord,
    model: &Model,
) -> Result<Option<Selection>> {
    profiles.recover_extensions(alias)?;
    let file = path(profiles, alias)?;
    let key = format!("{handle:08x}");
    if !crate::preset_store::names(&file)?.contains(&key) {
        return Ok(None);
    }
    let table: toml::Table = crate::preset_store::load(&file, &key)?.parse()?;
    if table.get("record").and_then(toml::Value::as_str) != Some(records::fingerprint(record).as_str()) {
        return Ok(None);
    }
    let payload = table.get("selection").and_then(toml::Value::as_str).context("Invalid garage extension")?;
    Ok(Some(presets::decode(payload, model)?))
}

#[cfg(test)]
#[path = "persistence_tests.rs"]
mod tests;

impl WorldScene {
    pub(in crate::scenes::world) fn driving_current_career_car(&self) -> bool {
        self.career
            .handle
            .with(|career| {
                let Some(car) = career.stable.car_by_handle(career.settings.car_handle) else { return false };
                self.garage.cars.iter().find(|entry| entry.folder == self.last_car).is_some_and(|entry| {
                    car.vehicle_key == blackbox_attrib::vlt_hash(&entry.physics_name.to_ascii_lowercase())
                })
            })
            .unwrap_or(false)
    }

    /// Restores only the current owned car. An unavailable mod part is reported and never applied blindly.
    pub(in crate::scenes::world) fn restore_career_garage(&mut self, renderer: &mut dyn RenderBackend) -> Result<()> {
        let Some(saved) = self.career.handle.with(|career| {
            let handle = career.settings.car_handle;
            let car = career.stable.car_by_handle(handle)?;
            let entry =
                self.garage.cars.iter().find(|entry| {
                    car.vehicle_key == blackbox_attrib::vlt_hash(&entry.physics_name.to_ascii_lowercase())
                })?;
            Some((career.alias.clone(), handle, entry.folder.clone(), career.stable.customization_for_handle(handle)))
        }) else {
            return Ok(());
        };
        let Some((alias, handle, folder, Some(record))) = saved else { return Ok(()) };
        self.discover_garage_parts(&folder)?;
        let tables = CarTables::load(&self.dir)?;
        let mut selection = records::read(&record, &tables, &folder, &self.garage)?;
        if let Some(profiles) = &self.career.profiles {
            match extension(profiles, &alias, handle, &record, &self.garage) {
                Ok(Some(mut saved)) => {
                    ensure!(saved.car == folder, "The saved garage extension belongs to a different car");
                    saved.inherit_native_color_parts(&selection);
                    selection = saved;
                }
                Ok(None) => {}
                Err(error) => log::warn!("garage extension unavailable: {error:#}"),
            }
        }
        // Loading a profile is not fitting a new purchase and must not cool its heat.
        self.career.garage =
            Some(super::session::ShopSession { baseline: self.garage.applied.clone(), handle: Some(handle), yaw: 2.4 });
        let result = self.apply_garage_selection(renderer, selection);
        self.career.garage = None;
        result
    }
}

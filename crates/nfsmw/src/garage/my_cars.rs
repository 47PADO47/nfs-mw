//! Profile-scoped custom car slots. Native imports keep their handle; remake slots have monotonic identities.

mod store;
#[cfg(test)]
mod tests;

use anyhow::{Context, Result};
use game_install::GameDir;

use super::{Model, Selection};
use crate::career::{CareerHandle, ProfileDir};

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Slot {
    pub id: String,
    pub label: String,
    pub car: String,
    pub vehicle_key: Option<u32>,
}

/// The current alias is part of the path, not a user-provided path fragment.
pub(crate) fn path() -> Result<std::path::PathBuf> {
    let alias = CareerHandle::shared().with_loaded(|career| career.alias.clone());
    if let Some(alias) = alias {
        return Ok(ProfileDir::default_dir().context("No profile folder")?.mod_data_dir(&alias)?.join("my-cars.toml"));
    }
    Ok(crate::settings::Settings::config_path()
        .context("No user configuration folder")?
        .with_file_name("my-cars-guest.toml"))
}

fn legacy_path() -> Option<std::path::PathBuf> {
    crate::settings::Settings::config_path().map(|path| path.with_file_name("my-cars.toml"))
}

fn current() -> Result<store::Store> {
    store::Store::read(&path()?, legacy_path().as_deref())
}

/// Native slots are listed separately, even when several slots use the same vehicle type.
pub(crate) fn slots() -> Result<Vec<Slot>> {
    let store = current()?;
    let mut slots = CareerHandle::shared()
        .with_loaded(|career| {
            career
                .stable
                .cars
                .iter()
                .filter(|car| !car.is_free() && car.is_quick_race())
                .map(|car| {
                    let name = career
                        .data
                        .cars
                        .iter()
                        .find(|entry| entry.frontend_key == car.frontend_key)
                        .map(|entry| entry.name.clone())
                        .unwrap_or_else(|| "Custom car".into());
                    Slot {
                        id: format!("native-{:08x}", car.handle),
                        label: name.clone(),
                        car: name,
                        vehicle_key: Some(car.vehicle_key),
                    }
                })
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    slots.retain(|slot| !store.deleted.contains(&slot.id) && !store.slots.contains_key(&slot.id));
    slots.extend(store.metadata()?);
    Ok(slots)
}

/// What the My Cars list calls the car `car`: its name, behind the maker's when the name does not start with it (a mod car
/// is `Porsche 911 Carrera S 2012`, not `Porsche Porsche 911 Carrera S 2012`).
pub(crate) fn car_label(car: &super::Car) -> String {
    let (maker, name) = (car.manufacturer.trim(), car.name.trim());
    let starts_with_maker = name.get(..maker.len()).is_some_and(|head| head.eq_ignore_ascii_case(maker));
    if maker.is_empty() || starts_with_maker {
        return name.to_owned();
    }
    format!("{maker} {name}")
}

/// What the My Cars list shows for `slot`: its label, or the name of the car where the label is only the car's folder (slots
/// made in the world's garage and the ones migrated from the old presets are labelled with the folder, which for a mod car
/// is an id like `POR911CARR12`).
pub(crate) fn display_label(slot: &Slot, model: &Model) -> String {
    let named = model.cars.iter().find(|car| car.folder.eq_ignore_ascii_case(slot.label.trim()));
    named.map_or_else(|| slot.label.clone(), car_label)
}

pub(crate) fn folder<'a>(slot: &Slot, model: &'a Model) -> Option<&'a str> {
    model
        .cars
        .iter()
        .find(|car| {
            car.folder.eq_ignore_ascii_case(&slot.car)
                || slot.vehicle_key == Some(blackbox_attrib::vlt_hash(&car.physics_name.to_ascii_lowercase()))
        })
        .map(|car| car.folder.as_str())
}

pub(crate) fn load(dir: &GameDir, model: &Model, id: &str, folder: &str) -> Result<Selection> {
    let store = current()?;
    anyhow::ensure!(!store.deleted.contains(id), "Custom car slot was deleted");
    if let Some(record) = store.slots.get(id) {
        let mut selection = super::presets::decode(&record.selection, model)?;
        if let Some(handle) = id.strip_prefix("native-").and_then(|value| u32::from_str_radix(value, 16).ok())
            && let Ok(Some(native)) = crate::scenes::world::native_my_car_handle(dir, model, folder, handle)
        {
            selection.inherit_native_color_parts(&native);
        }
        return Ok(selection);
    }
    let handle = id
        .strip_prefix("native-")
        .and_then(|value| u32::from_str_radix(value, 16).ok())
        .context("Custom car slot is missing")?;
    crate::scenes::world::native_my_car_handle(dir, model, folder, handle)?.context("Native custom car slot is missing")
}

pub(crate) fn create(selection: &Selection, label: &str) -> Result<String> {
    let mut store = current()?;
    anyhow::ensure!(slots()?.len() < store.slot_limit, "My Cars is full ({} slots)", store.slot_limit);
    let id = store.create(selection, label)?;
    store.write(&path()?)?;
    Ok(id)
}

pub(crate) fn slot_limit() -> Result<usize> {
    Ok(current()?.slot_limit)
}

pub(crate) fn save(id: &str, selection: &Selection, label: &str) -> Result<()> {
    let mut store = current()?;
    store.save(id, selection, label)?;
    store.write(&path()?)
}

pub(crate) fn delete(id: &str) -> Result<()> {
    let mut store = current()?;
    store.slots.remove(id);
    store.deleted.insert(id.to_owned());
    store.write(&path()?)
}

pub(crate) fn save_world_slot(selection: &Selection, id: Option<&str>, model: &Model) -> Result<String> {
    let Some(id) = id else {
        let label =
            model.cars.iter().find(|car| car.folder == selection.car).map_or_else(|| selection.car.clone(), car_label);
        return create(selection, &label);
    };
    let slot = slots()?.into_iter().find(|slot| slot.id == id).context("The driven custom car slot was deleted")?;
    save(id, selection, &slot.label)?;
    Ok(id.to_owned())
}

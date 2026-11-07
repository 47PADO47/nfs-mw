//! Restore the selected career slot on the backdrop's existing CPU worker.

use std::collections::HashMap;
use std::sync::{Arc, Mutex, OnceLock};

use anyhow::{Context, Result};
use game_install::GameDir;
use nfsmw_data::car::{
    CarModel, CarTables, LoadOptions, Paint, customization,
    physics::{PerformanceCatalog, PhysicsData},
    slot, stock_parts,
};

use crate::frontend::look::Look;
use crate::garage::{Car, Model};

/// Only the catalogue is memoised. Each request decodes its own captured record and current extension.
fn catalog(dir: &GameDir, folder: &str) -> Result<Arc<Model>> {
    type Key = (std::path::PathBuf, String);
    static CATALOGS: OnceLock<Mutex<HashMap<Key, Arc<Model>>>> = OnceLock::new();
    let mut catalogs = CATALOGS.get_or_init(Default::default).lock().unwrap_or_else(|e| e.into_inner());
    let key = (dir.root().to_owned(), folder.to_owned());
    if let Some(model) = catalogs.get(&key) {
        return Ok(model.clone());
    }
    let tables = CarTables::load(dir)?;
    let car = tables.car_type_for_folder(folder).context("Selected car type is not installed")?;
    let physics = PhysicsData::load(dir)?;
    let performance = match nfsmw_data::car::mods::stats(folder) {
        Some(stats) => nfsmw_data::car::mods::components::catalog::performance(folder, stats),
        None => PerformanceCatalog::read(physics.database(), &car.type_name)?,
    };
    let entry = Car {
        folder: folder.to_owned(),
        physics_name: car.type_name.clone(),
        paintable: car.skinnable,
        original_color: stock_parts(&tables, car)[slot::BASE_PAINT].map(|part| Paint::of(&tables, &part).rgb),
        performance,
        parts: Some(customization::Inventory::load(dir)?.catalog(dir, folder)?.into()),
        ..Car::default()
    };
    let model = Arc::new(Model { cars: vec![entry], ..Model::default() });
    catalogs.insert(key, model.clone());
    Ok(model)
}

#[cfg(test)]
#[path = "backdrop_career_tests.rs"]
mod tests;

pub(super) fn load(dir: &GameDir, look: &Look) -> Option<Result<(CarModel, LoadOptions)>> {
    let car = match look {
        Look::SafeHouse(car) | Look::Car(car) | Look::CarLot(car) | Look::LotShowcase(car) => car,
        _ => return None,
    };
    let record = car.record.as_ref()?;
    let customization = record.customization.as_ref()?;
    let alias = record.alias.as_deref()?;
    Some((|| {
        let (folder, preset) = car.resolve(dir);
        let model = catalog(dir, &folder)?;
        let selection = crate::scenes::world::career_preview_selection(
            dir,
            &model,
            &folder,
            alias,
            record.handle,
            customization,
            crate::career::ProfileDir::default_dir().as_ref(),
        )?;
        let options = LoadOptions { lod: 'A', all_parts: false, preset };
        let mut car = nfsmw_data::car::load_customized(dir, &folder, &options, &selection.parts)?;
        if let Some(color) = selection.color {
            car.set_body_color(color)?;
        }
        Ok((car, options))
    })())
}

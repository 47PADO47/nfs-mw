//! Loading a car for the world's driving mode.

use anyhow::{Context, Result};
use game_install::GameDir;
use nfsmw_data::car::CarModel;

/// Load a car for driving (highest detail, stock parts). `name` may be a unique prefix.
pub(super) fn load_car(dir: &GameDir, name: &str) -> Result<(String, CarModel)> {
    load_car_as(dir, name, None)
}

/// Load a car for driving as `CAR[:PRESET]`: the car folder and, after a colon, the preset ride it is built with (the custom
/// race's bonus and preset cars, as `view-screen --car` names them).
pub(super) fn load_named_car(dir: &GameDir, wanted: &str) -> Result<(String, CarModel)> {
    let (name, preset) =
        wanted.split_once(':').map_or((wanted, None), |(name, preset)| (name, Some(preset.to_owned())));
    load_car_as(dir, name, preset)
}

fn load_car_as(dir: &GameDir, name: &str, preset: Option<String>) -> Result<(String, CarModel)> {
    let cars = nfsmw_data::car::list(dir);
    let folder = nfsmw_data::car::pick_folder(&cars, name)
        .with_context(|| format!("no car {name:?}, or several start with it (`nfsmw list-cars`)"))?
        .to_owned();
    let options = nfsmw_data::car::LoadOptions { lod: 'A', all_parts: false, preset };
    let model = nfsmw_data::car::load_driving(dir, &folder, &options)?;
    Ok((folder, model))
}

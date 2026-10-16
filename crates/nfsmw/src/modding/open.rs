//! Opening the install with its mods: the one place where the mods start and their layers reach the install.

use std::path::Path;

use anyhow::Result;
use game_install::GameDir;

use super::backends;
use super::{Mods, layers, policy};
use crate::cli::ViewArgs;
use crate::settings::Settings;
use crate::settings::mods::ModsSettings;

/// Open the install, start the mods, and add the `files/` folder of each loaded mod to the install as a layer.
///
/// The mods start first, and the layers are in place before the install is returned, so every scene that clones the
/// install sees them. A mod that fails does not stop the game.
pub fn open_install(game_dir: Option<&Path>, view: &ViewArgs) -> Result<(GameDir, Mods)> {
    let mut dir = nfsmw_data::game::open_install(game_dir)?;
    let settings = ModsSettings::load(view.settings_layer());
    let policy = policy::policy(&settings);
    let config_file = Settings::config_path();
    let roots = policy::roots(config_file.as_deref(), view.mods_dir.as_deref());
    let mods = Mods::start(policy::offer(), backends::interfaces(), &policy, &roots, policy::run_mode(view.mod_dev));
    let loaded: Vec<String> = mods.loaded().into_iter().map(|summary| summary.id).collect();
    let discovery = blackbox_modding::host::discover(&roots, &policy, &policy::offer());
    layers::apply(&mut dir, &layers::plan(&loaded, &discovery.candidates));
    Ok((dir, mods))
}

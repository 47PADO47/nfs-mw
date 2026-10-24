//! Opening the install with its mods: the one place where the mods start and their layers reach the install.

use std::path::Path;

use anyhow::Result;
use game_install::GameDir;

use super::backends;
use super::manager::ModManager;
use super::{Mods, content, layers, policy};
use crate::cli::ViewArgs;
use crate::settings::Settings;
use crate::settings::mods::ModsSettings;

/// Open the install, start the mods, and add the `files/` folder of each loaded mod to the install as a layer. Then the
/// cars and music the mods declare are read and sealed ([`content`]), against the layers just added.
///
/// The mods start first, and the layers are in place before the install is returned, so every scene that clones the
/// install sees them. A mod that fails does not stop the game.
pub fn open_install(game_dir: Option<&Path>, view: &ViewArgs) -> Result<(GameDir, Mods)> {
    let mut dir = nfsmw_data::game::open_install(game_dir)?;
    let settings = ModsSettings::load(view.settings_layer());
    let policy = policy::policy(&settings);
    let config_file = Settings::config_path();
    let roots = policy::roots(config_file.as_deref(), view.mods_dir.as_deref());
    let mut mods =
        Mods::start(policy::offer(), backends::interfaces(), &policy, &roots, policy::run_mode(view.mod_dev));
    let loaded: Vec<String> = mods.loaded().into_iter().map(|summary| summary.id).collect();
    let refused: Vec<String> = mods.rejected().iter().filter_map(|rejected| rejected.id.clone()).collect();
    mods.set_manager(ModManager::new(settings, config_file.clone(), &loaded, &refused));
    // The runtime found the mods once, when it started; their folders come from it, not from a second scan.
    let candidates = mods.loaded_candidates();
    layers::apply(&mut dir, &layers::plan(&loaded, &candidates));
    content::register(&dir, &content::plan(&loaded, &candidates));
    // The language packs are mod content: the language is chosen once the mods' folders are known.
    crate::i18n::start(&dir, Settings::load(view.settings_layer()).language);
    mods.set_install(&dir);
    Ok((dir, mods))
}

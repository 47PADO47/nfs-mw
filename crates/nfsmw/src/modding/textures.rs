//! The texture replacement sources of a run (`docs/texture-packs.md`): the `textures/` folder of each loaded mod, at the
//! mod's `[content] priority`, and the HD texture pack folder of the settings, below every mod. Indexed once, when the
//! install opens with its mods, before any scene loads.

use std::path::Path;

use blackbox_modding::host::Candidate;
use nfsmw_data::texture_packs::{self, ModFolder, SETTINGS_PRIORITY, SourceSpec, TexturePacks};

use crate::settings::hd_content::HdContent;

/// The folder inside a mod that holds its replacement DDS files.
pub const TEXTURES_FOLDER: &str = "textures";

/// The id the settings' folder goes by in the logs and the F8 panel.
pub const SETTINGS_SOURCE: &str = "settings";

/// The sources, lowest first: the settings' folder, then each loaded mod with a `textures/` folder in load order (the
/// index sorts them by priority; equal priorities keep this order, so the mod loaded later wins). Also every loaded
/// mod's folder, for the floor rule.
pub fn plan(loaded: &[String], candidates: &[Candidate], hd: &HdContent) -> (Vec<SourceSpec>, Vec<ModFolder>) {
    let mut specs = Vec::new();
    let folder = hd.texture_packs.as_str();
    if !folder.is_empty() {
        let root = Path::new(folder).to_owned();
        specs.push(SourceSpec { id: SETTINGS_SOURCE.to_owned(), root, priority: SETTINGS_PRIORITY });
    }
    let mut mods = Vec::new();
    for id in loaded {
        let Some(candidate) = candidates.iter().find(|candidate| &candidate.manifest.id == id) else {
            continue;
        };
        let priority = candidate.manifest.priority;
        mods.push(ModFolder { dir: candidate.dir.clone(), priority });
        let root = candidate.dir.join(TEXTURES_FOLDER);
        if root.is_dir() {
            specs.push(SourceSpec { id: id.clone(), root, priority });
        }
    }
    (specs, mods)
}

/// Index the sources and make them the run's. Logged as warnings, so release builds (which drop info lines) show it:
/// the game will not look as the install has it.
pub fn apply(specs: Vec<SourceSpec>, mods: Vec<ModFolder>, max_size: u32) {
    for spec in specs.iter().filter(|spec| !spec.root.is_dir()) {
        log::warn!("texture replacement: {} is not a folder; it replaces nothing", spec.root.display());
    }
    let any = !specs.is_empty();
    let packs = texture_packs::seal(TexturePacks::new(specs, mods, max_size));
    if !any {
        return;
    }
    for line in packs.describe() {
        log::warn!("texture replacement: {line}");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_settings_folder_comes_first_and_a_missing_mod_adds_nothing() {
        let mut hd = HdContent::default();
        hd.texture_packs = crate::settings::hd_content::texture_packs::parse("/packs").unwrap();
        let (specs, mods) = plan(&["absent".to_owned()], &[], &hd);
        assert_eq!(
            specs,
            vec![SourceSpec { id: SETTINGS_SOURCE.to_owned(), root: "/packs".into(), priority: SETTINGS_PRIORITY }]
        );
        assert!(mods.is_empty());
        let (specs, _) = plan(&[], &[], &HdContent::default());
        assert!(specs.is_empty());
    }
}

//! The second worlds of the mods (docs/specs/second-world.md): each `worlds/<id>.toml` is a world the player can choose
//! instead of the city, and its map packs are the manifests in `worlds/<id>/`. The list is read when a world opens or a
//! menu lists the choices, against the loaded mods; nothing here is sealed at start-up, so a world a mod finishes writing
//! later (the converting mods do it on a thread) is found at the next look.

use std::collections::HashSet;

use blackbox_modding::content::{WORLDS_FOLDER, WorldManifest, parse_world};
use game_install::GameDir;

use super::scenery::{MapPack, world_packs};
use super::{ModDir, mod_dirs, toml_files};

/// A world a mod provides.
#[derive(Debug, Clone, PartialEq)]
pub struct World {
    /// The mod that provides it.
    pub owner: ModDir,
    pub manifest: WorldManifest,
}

impl World {
    pub fn id(&self) -> &str {
        &self.manifest.id
    }

    pub fn name(&self) -> &str {
        &self.manifest.name
    }
}

/// The worlds of `mods`, in load order and then file-name order. A manifest that is refused is logged and left out, and an
/// id an earlier mod has is not taken twice.
pub fn list_in(mods: &[ModDir]) -> Vec<World> {
    let mut out: Vec<World> = Vec::new();
    for m in mods {
        for path in toml_files(&m.dir.join(WORLDS_FOLDER)) {
            let Some(stem) = path.file_stem().and_then(|stem| stem.to_str()) else { continue };
            let manifest = match std::fs::read_to_string(&path)
                .map_err(|e| e.to_string())
                .and_then(|text| parse_world(stem, &text).map_err(|e| e.to_string()))
            {
                Ok(manifest) => manifest,
                Err(reason) => {
                    log::warn!("mod {}: world {} is not added: {reason}", m.id, path.display());
                    continue;
                }
            };
            if out.iter().any(|world| world.id() == manifest.id) {
                log::warn!("mod {}: world {} is already provided by another mod", m.id, manifest.id);
                continue;
            }
            out.push(World { owner: m.clone(), manifest });
        }
    }
    out
}

/// The worlds of the loaded mods.
pub fn list() -> Vec<World> {
    list_in(mod_dirs())
}

/// The world called `id` (ignoring case), if a loaded mod provides it.
pub fn find(id: &str) -> Option<World> {
    list().into_iter().find(|world| world.id().eq_ignore_ascii_case(id))
}

/// The world a `world` setting names: `None` for the city (no setting, or `rockport`), else the world a loaded mod provides.
/// A name no mod provides is an error that lists what is installed.
pub fn chosen(id: Option<&str>) -> anyhow::Result<Option<World>> {
    let Some(id) = id.filter(|id| !id.eq_ignore_ascii_case(crate::settings::world_id::CITY)) else { return Ok(None) };
    if let Some(world) = find(id) {
        return Ok(Some(world));
    }
    let known: Vec<String> = list().iter().map(|world| world.id().to_owned()).collect();
    anyhow::bail!("world {id:?} is not provided by a loaded mod (installed: {})", known.join(", "))
}

/// The map packs of `world`, checked against the install's section numbers. Empty when the mod has not written any yet.
pub fn packs(dir: &GameDir, world: &World, installed: &HashSet<i16>) -> Vec<MapPack> {
    let mut taken = installed.clone();
    world_packs(dir, &world.owner, world.id(), &mut taken)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mod_with(id: &str, worlds: &[(&str, &str)]) -> ModDir {
        let root = std::env::temp_dir().join(format!("nfsmw-worlds-{}-{id}", std::process::id()));
        let folder = root.join(WORLDS_FOLDER);
        std::fs::create_dir_all(&folder).unwrap();
        for (name, text) in worlds {
            std::fs::write(folder.join(format!("{name}.toml")), text).unwrap();
        }
        ModDir { id: id.to_owned(), dir: root, priority: 0 }
    }

    const OK: &str = "[world]\nname = \"Okayama\"\nstart = { pos = [1.0, 2.0, 3.0], heading = 90.0 }\n";

    #[test]
    fn the_worlds_of_a_mod_are_listed_in_name_order_and_a_bad_one_is_left_out() {
        let m = mod_with("tracks", &[("zeta", OK), ("alpha", OK), ("broken", "[world]\nname = 1\n")]);
        let worlds = list_in(std::slice::from_ref(&m));
        let ids: Vec<_> = worlds.iter().map(World::id).collect();
        assert_eq!(ids, ["alpha", "zeta"]);
        assert_eq!(worlds[0].name(), "Okayama");
        assert_eq!(worlds[0].manifest.start, [1.0, 2.0, 3.0]);
    }

    #[test]
    fn an_id_that_an_earlier_mod_has_is_not_taken_twice() {
        let first = mod_with("first", &[("okayama", OK)]);
        let second = mod_with("second", &[("okayama", OK)]);
        let worlds = list_in(&[first, second]);
        assert_eq!(worlds.len(), 1);
        assert_eq!(worlds[0].owner.id, "first");
    }
}

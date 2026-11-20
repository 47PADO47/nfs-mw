//! The second worlds of the mods (docs/specs/second-world.md): each `worlds/<id>.toml` is a world the player can choose
//! instead of the city, and its map packs are the manifests in `worlds/<id>/`. The list is read when a world opens or a
//! menu lists the choices, against the loaded mods; nothing here is sealed at start-up, so a world a mod finishes writing
//! later (the converting mods do it on a thread) is found at the next look.

use std::collections::HashSet;

use blackbox_modding::content::{
    PATCHES_FOLDER, PackInput, PackOrigin, PatchInput, WORLDS_FOLDER, WorldManifest, WorldPatch, apply_world_patches,
    order_world_patches, parse_world, parse_world_patch, plan_packs,
};
use game_install::GameDir;

use super::scenery::{MapPack, patch_packs, world_packs};
use super::{ModDir, mod_dirs, toml_files};

/// What the host does with worlds beyond the manifest, as `bb.content` capability bits: world patches and alternative maps.
pub const WORLD_CAPABILITIES: u32 =
    blackbox_modding::abi::BB_CONTENT_CAP_WORLD_PATCHES | blackbox_modding::abi::BB_CONTENT_CAP_ALT_MAPS;

/// A change a mod makes to a world: the patch, the mod that brings it, its place in the load order, and the keys of the pack manifests
/// in its folder.
#[derive(Debug, Clone, PartialEq)]
pub struct Patch {
    pub owner: ModDir,
    pub order: usize,
    pub patch: WorldPatch,
    pub keys: Vec<String>,
}

/// A world a mod provides.
#[derive(Debug, Clone, PartialEq)]
pub struct World {
    /// The mod that provides it.
    pub owner: ModDir,
    /// The manifest with the patches applied (`blackbox_modding::content::apply_world_patches`).
    pub manifest: WorldManifest,
    /// The patches that apply to it, in the order they apply.
    pub patches: Vec<Patch>,
}

impl World {
    /// Whether the world is an alternative map: it plays in the city.
    pub fn is_alternative_map(&self) -> bool {
        self.manifest.is_alternative_map()
    }

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
            out.push(World { owner: m.clone(), manifest, patches: Vec::new() });
        }
    }
    apply_patches(mods, &mut out);
    out
}

/// The patches of `mods`: each `world-patches/<id>.toml` with the keys of the packs in `world-patches/<id>/`. A manifest that is
/// refused is logged and left out.
fn read_patches(mods: &[ModDir]) -> Vec<Patch> {
    let mut patches = Vec::new();
    for (order, m) in mods.iter().enumerate() {
        for path in toml_files(&m.dir.join(PATCHES_FOLDER)) {
            let Some(stem) = path.file_stem().and_then(|stem| stem.to_str()) else { continue };
            let parsed = std::fs::read_to_string(&path)
                .map_err(|error| error.to_string())
                .and_then(|text| parse_world_patch(stem, &text).map_err(|error| error.to_string()));
            let patch = match parsed {
                Ok(patch) => patch,
                Err(reason) => {
                    log::warn!("mod {}: world patch {} is not added: {reason}", m.id, path.display());
                    continue;
                }
            };
            let keys = toml_files(&m.dir.join(PATCHES_FOLDER).join(stem))
                .iter()
                .filter_map(|file| file.file_stem().and_then(|stem| stem.to_str()).map(str::to_owned))
                .collect();
            patches.push(Patch { owner: m.clone(), order, patch, keys });
        }
    }
    patches
}

/// Apply the patches of `mods` to the worlds in `worlds`: each world takes the patches that name it, in the order the merge rule
/// gives, and the clashes the merge settled are logged. A patch that names no world here changes nothing.
fn apply_patches(mods: &[ModDir], worlds: &mut [World]) {
    let all = read_patches(mods);
    for world in worlds {
        let mine: Vec<&Patch> =
            all.iter().filter(|patch| patch.patch.target.eq_ignore_ascii_case(world.id())).collect();
        if mine.is_empty() {
            continue;
        }
        let inputs: Vec<PatchInput<'_>> = mine
            .iter()
            .map(|patch| PatchInput { owner: &patch.owner.id, order: patch.order, patch: &patch.patch })
            .collect();
        let applied = apply_world_patches(&world.manifest, &inputs);
        for line in &applied.report {
            log::warn!("world {}: {line}", world.id());
        }
        let ordered = order_world_patches(&inputs);
        world.patches = ordered
            .iter()
            .filter_map(|input| mine.iter().find(|patch| std::ptr::eq(&patch.patch, input.patch)))
            .map(|patch| (*patch).clone())
            .collect();
        world.manifest = applied.manifest;
    }
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

/// The map packs of `world`, checked against the install's section numbers: its own, minus what its patches remove or replace, plus
/// what they add (`plan_packs`). Empty when the mod has not written any yet. A section number two packs would take is given to the
/// first; the other is left out with a warning.
pub fn packs(dir: &GameDir, world: &World, installed: &HashSet<i16>) -> Vec<MapPack> {
    let own = world_packs(dir, &world.owner, world.id(), &mut installed.clone());
    if world.patches.is_empty() {
        return own;
    }
    let own_keys: Vec<String> = own.iter().map(|pack| pack.key.clone()).collect();
    let inputs: Vec<PackInput<'_>> = world
        .patches
        .iter()
        .map(|patch| PackInput { owner: &patch.owner.id, patch: &patch.patch, keys: &patch.keys })
        .collect();
    let plan = plan_packs(&own_keys, &inputs);
    for line in &plan.report {
        log::warn!("world {}: {line}", world.id());
    }
    let added: Vec<Vec<MapPack>> = world
        .patches
        .iter()
        .map(|patch| patch_packs(dir, &patch.owner, &patch.patch.id, &mut installed.clone()))
        .collect();
    let mut taken = HashSet::new();
    let mut out = Vec::new();
    for origin in plan.packs {
        let found = match &origin {
            PackOrigin::World(key) => own.iter().find(|pack| pack.key.eq_ignore_ascii_case(key)),
            PackOrigin::Patch { index, key } => added[*index].iter().find(|pack| pack.key.eq_ignore_ascii_case(key)),
        };
        let Some(pack) = found else { continue };
        if !taken.insert(pack.section.number) {
            log::warn!(
                "world {}: the pack {} takes section {}, which another pack has; left out",
                world.id(),
                pack.key,
                pack.section.number
            );
            continue;
        }
        out.push(pack.clone());
    }
    out
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

    #[test]
    fn a_patch_of_another_mod_changes_the_world_and_a_patch_of_nothing_is_ignored() {
        let tracks = mod_with("tracks-patched", &[("okayama", OK)]);
        let patcher = mod_with("patcher", &[]);
        let folder = patcher.dir.join(PATCHES_FOLDER);
        std::fs::create_dir_all(&folder).unwrap();
        std::fs::write(
            folder.join("dusk.toml"),
            "format = 1
[patch]
target = \"Okayama\"
name = \"Okayama at dusk\"
environment = { hour = 19.0, freeze = true }
start = { pos = [5.0, 6.0, 7.0], heading = 10.0 }
",
        )
        .unwrap();
        std::fs::write(
            folder.join("other.toml"),
            "format = 1
[patch]
target = \"nowhere\"
name = \"X\"
",
        )
        .unwrap();
        let worlds = list_in(&[tracks.clone(), patcher.clone()]);
        assert_eq!(worlds.len(), 1);
        let world = &worlds[0];
        assert_eq!(
            (world.name(), world.manifest.start, world.manifest.heading),
            ("Okayama at dusk", [5.0, 6.0, 7.0], 10.0)
        );
        assert_eq!(world.manifest.environment.and_then(|env| env.hour), Some(19.0));
        assert_eq!(world.patches.len(), 1);
        assert_eq!(world.patches[0].owner.id, "patcher");
        assert_eq!(world.owner.id, "tracks-patched", "the world's own mod stays its owner");
        let plain = list_in(&[tracks]);
        assert_eq!(plain[0].name(), "Okayama", "without the patching mod the world is as it was");
    }

    #[test]
    fn a_world_with_the_city_as_base_is_an_alternative_map() {
        let alt = OK.replace(
            "[world]
",
            "[world]
base = \"rockport\"
",
        );
        let m = mod_with("maps", &[("harbour", &alt), ("plain", OK)]);
        let worlds = list_in(std::slice::from_ref(&m));
        assert!(worlds.iter().find(|w| w.id() == "harbour").unwrap().is_alternative_map());
        assert!(!worlds.iter().find(|w| w.id() == "plain").unwrap().is_alternative_map());
    }
}

//! `world-patches/<id>.toml`: a change one mod makes to a world another mod (or the game) provides
//! (docs/specs/mod-content-replacement-worlds.md).
//!
//! ```toml
//! format = 1
//!
//! [patch]
//! target = "okayama"                 # a world's id, or "rockport" for the city (and its alternative maps)
//! priority = 0                       # -1000 to 1000: patches apply from the lowest; the highest wins a clash
//! name = "Okayama at dusk"           # optional: renames the world in the menu
//! start = { pos = [-230.8, -214.7, 26.1], heading = 95.1 }
//! environment = { hour = 19.0 }      # optional; replaces the world's own, whole
//! remove_packs = ["g017"]            # map packs of the target, by key, left out
//! replace_packs = ["g003"]           # keys of packs this patch's folder gives again: they take the place of the target's
//!
//! [[patch.grid]]                     # start slots, appended to the target's (grid_replace = true: instead of them)
//! pos = [0.0, 0.0, 0.0]
//! ```
//!
//! The packs a patch adds are the scenery manifests in `world-patches/<id>/` of the same mod, checked like `worlds/<id>/` ones.
//! This module checks the schema and does the merging as pure functions ([`apply`], [`plan_packs`]); the game reads the files,
//! checks the packs and applies the plan.

use std::collections::HashSet;

use serde::Deserialize;

use super::world::{Environment, MAX_GRID_SLOTS, RawEnvironment, RawStart, Slot, WorldManifest, check_name};
use super::{ContentError, is_id, variant};

/// The folder of a mod that holds one manifest per patch, and, in a folder of the patch's id, the packs it adds.
pub const PATCHES_FOLDER: &str = "world-patches";
/// The most packs a patch removes or replaces.
const MAX_KEYS: usize = 4096;
const MAX_PRIORITY: i32 = 1000;

/// A checked `world-patches/<id>.toml`.
#[derive(Debug, Clone, PartialEq)]
pub struct WorldPatch {
    /// The patch's id: the file name without `.toml`.
    pub id: String,
    /// The world it changes (lower case): a mod's world, or `rockport`.
    pub target: String,
    pub priority: i32,
    pub name: Option<String>,
    pub start: Option<Slot>,
    pub environment: Option<Environment>,
    pub grid: Vec<Slot>,
    /// `grid` replaces the target's slots instead of being added to them.
    pub grid_replace: bool,
    pub remove_packs: Vec<String>,
    pub replace_packs: Vec<String>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Raw {
    format: u32,
    patch: RawPatch,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawPatch {
    target: String,
    #[serde(default)]
    priority: i32,
    name: Option<String>,
    start: Option<RawStart>,
    environment: Option<RawEnvironment>,
    #[serde(default)]
    grid: Vec<RawStart>,
    #[serde(default)]
    grid_replace: bool,
    #[serde(default)]
    remove_packs: Vec<String>,
    #[serde(default)]
    replace_packs: Vec<String>,
}

fn keys(list: Vec<String>, what: &'static str) -> Result<Vec<String>, ContentError> {
    if list.len() > MAX_KEYS {
        return Err(ContentError::Field { field: what, reason: "at most 4096 keys" });
    }
    let mut seen = HashSet::new();
    for key in &list {
        if !is_id(key) {
            return Err(ContentError::InvalidId(key.clone()));
        }
        if !seen.insert(key.to_ascii_lowercase()) {
            return Err(ContentError::Field { field: what, reason: "a key is listed once" });
        }
    }
    Ok(list)
}

/// Parse `world-patches/<stem>.toml`.
pub fn parse(stem: &str, text: &str) -> Result<WorldPatch, ContentError> {
    let raw: Raw = toml::from_str(text).map_err(|error| ContentError::Parse(error.to_string()))?;
    variant::check_format(raw.format)?;
    if !is_id(stem) {
        return Err(ContentError::InvalidId(stem.to_owned()));
    }
    let RawPatch { target, priority, name, start, environment, grid, grid_replace, remove_packs, replace_packs } =
        raw.patch;
    if !is_id(&target) {
        return Err(ContentError::InvalidId(target));
    }
    if !(-MAX_PRIORITY..=MAX_PRIORITY).contains(&priority) {
        return Err(ContentError::Field { field: "patch.priority", reason: "-1000 to 1000" });
    }
    if let Some(name) = &name {
        check_name(name)?;
    }
    if grid.len() > MAX_GRID_SLOTS {
        return Err(ContentError::Field { field: "grid", reason: "at most 64 start slots" });
    }
    let (remove_packs, replace_packs) = (keys(remove_packs, "remove_packs")?, keys(replace_packs, "replace_packs")?);
    if remove_packs.iter().any(|key| replace_packs.iter().any(|other| other.eq_ignore_ascii_case(key))) {
        return Err(ContentError::Field { field: "replace_packs", reason: "a pack is removed or replaced, not both" });
    }
    Ok(WorldPatch {
        id: stem.to_owned(),
        target: target.to_ascii_lowercase(),
        priority,
        name,
        start: start.map(RawStart::check).transpose()?,
        environment: environment.map(RawEnvironment::check).transpose()?,
        grid: grid.into_iter().map(RawStart::check).collect::<Result<Vec<_>, _>>()?,
        grid_replace,
        remove_packs,
        replace_packs,
    })
}

/// A patch with the mod it came from and its place in the load order (earlier mods have lower numbers).
#[derive(Debug, Clone, Copy)]
pub struct PatchInput<'a> {
    pub owner: &'a str,
    pub order: usize,
    pub patch: &'a WorldPatch,
}

/// The patches in the order they apply: lowest priority first, then load order, then id. The last one to set a thing wins.
pub fn order<'a>(patches: &[PatchInput<'a>]) -> Vec<PatchInput<'a>> {
    let mut sorted = patches.to_vec();
    sorted.sort_by(|a, b| (a.patch.priority, a.order, &a.patch.id).cmp(&(b.patch.priority, b.order, &b.patch.id)));
    sorted
}

/// What [`apply`] made: the world with the patches in, and a line for every clash that was settled.
#[derive(Debug, Clone, PartialEq)]
pub struct Applied {
    pub manifest: WorldManifest,
    pub report: Vec<String>,
}

/// The world `base` with `patches` applied in [`order`]: a later patch's name, start and environment replace an earlier one's (and
/// the report says so), and start slots are appended (or replace the slots so far).
pub fn apply(base: &WorldManifest, patches: &[PatchInput<'_>]) -> Applied {
    let mut manifest = base.clone();
    let mut report = Vec::new();
    let mut set_by: [Option<&str>; 3] = [None; 3];
    for input in order(patches) {
        let (owner, patch) = (input.owner, input.patch);
        let mut take = |index: usize, what: &str, changed: bool| {
            if !changed {
                return;
            }
            if let Some(earlier) = set_by[index] {
                report.push(format!(
                    "mod {owner}: patch {} sets the {what} that mod {earlier} set; this one wins",
                    patch.id
                ));
            }
            set_by[index] = Some(owner);
        };
        take(0, "name", patch.name.is_some());
        take(1, "start", patch.start.is_some());
        take(2, "environment", patch.environment.is_some());
        if let Some(name) = &patch.name {
            manifest.name.clone_from(name);
        }
        if let Some(start) = patch.start {
            manifest.start = start.pos;
            manifest.heading = start.heading;
        }
        if let Some(environment) = patch.environment {
            manifest.environment = Some(environment);
        }
        if patch.grid_replace {
            manifest.grid.clear();
        }
        manifest.grid.extend(patch.grid.iter().copied());
        manifest.grid.truncate(MAX_GRID_SLOTS);
    }
    Applied { manifest, report }
}

/// A patch's packs: the keys of the scenery manifests in its folder.
#[derive(Debug, Clone, Copy)]
pub struct PackInput<'a> {
    pub owner: &'a str,
    pub patch: &'a WorldPatch,
    pub keys: &'a [String],
}

/// Where a pack of the plan comes from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PackOrigin {
    /// The world's own pack with this key.
    World(String),
    /// The pack with this key of the patch at this index of the inputs.
    Patch { index: usize, key: String },
}

/// The packs of a world after the patches: the world's own, minus what is removed or replaced, plus what the patches add.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PackPlan {
    pub packs: Vec<PackOrigin>,
    pub report: Vec<String>,
}

fn key_of(origin: &PackOrigin) -> &str {
    match origin {
        PackOrigin::World(key) | PackOrigin::Patch { key, .. } => key,
    }
}

/// The plan for `world_keys` and `patches` (already in application order, [`order`]). A pack a patch adds under a key that is
/// already there is refused, unless the patch lists the key in `replace_packs`, in which case it takes the place (and the position)
/// of the pack that had it. Removing or replacing a pack that is not there is reported and changes nothing.
pub fn plan_packs(world_keys: &[String], patches: &[PackInput<'_>]) -> PackPlan {
    let mut packs: Vec<PackOrigin> = world_keys.iter().cloned().map(PackOrigin::World).collect();
    let mut report = Vec::new();
    let position =
        |packs: &[PackOrigin], key: &str| packs.iter().position(|pack| key_of(pack).eq_ignore_ascii_case(key));
    for (index, input) in patches.iter().enumerate() {
        let (owner, patch) = (input.owner, input.patch);
        for key in &patch.remove_packs {
            match position(&packs, key) {
                Some(at) => {
                    packs.remove(at);
                }
                None => {
                    report.push(format!("mod {owner}: patch {} removes the pack {key}, which is not there", patch.id))
                }
            }
        }
        for key in input.keys {
            let replaces = patch.replace_packs.iter().any(|listed| listed.eq_ignore_ascii_case(key));
            let entry = PackOrigin::Patch { index, key: key.clone() };
            match (position(&packs, key), replaces) {
                (None, false) => packs.push(entry),
                (None, true) => {
                    report.push(format!("mod {owner}: patch {} replaces the pack {key}, which is not there; it is added", patch.id));
                    packs.push(entry);
                }
                (Some(at), true) => packs[at] = entry,
                (Some(_), false) => report.push(format!(
                    "mod {owner}: patch {} adds the pack {key}, but the key is taken (list it in replace_packs to replace it); left out",
                    patch.id
                )),
            }
        }
        for key in &patch.replace_packs {
            if !input.keys.iter().any(|own| own.eq_ignore_ascii_case(key)) {
                report
                    .push(format!("mod {owner}: patch {} lists {key} in replace_packs but has no such pack", patch.id));
            }
        }
    }
    PackPlan { packs, report }
}

#[cfg(test)]
#[path = "world_patch_tests.rs"]
mod tests;

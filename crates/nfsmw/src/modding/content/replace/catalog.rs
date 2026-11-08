//! The replacement sets the loaded mods declare: each `replace/<id>.toml` read and checked, and grouped by kind (the option groups).
//! Nothing is loaded from the sets' files here: the manifests are small text, read at start-up and again while a chosen value has no set
//! yet (a mod that converts content on a thread writes its manifest when it is done).

use std::path::PathBuf;
use std::sync::Arc;

use blackbox_modding::content::{REPLACE_FOLDER, ReplaceSet, parse_replace_set};

use crate::modding::content::{ModDir, toml_files};

/// The games whose models and textures this host reads as they are: the running one.
const READABLE_GAMES: [&str; 1] = ["nfsmw"];

/// A set a mod declares.
#[derive(Debug, Clone)]
pub struct SetRecord {
    /// The mod's id and its place in the load order.
    pub owner: String,
    pub order: usize,
    pub set: Arc<ReplaceSet>,
}

impl SetRecord {
    /// `mod:set`, the name logs and reports use.
    pub fn label(&self) -> String {
        format!("{}:{}", self.owner, self.set.id)
    }
}

/// A manifest that was not taken, with why.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Refused {
    pub owner: String,
    pub path: PathBuf,
    pub reason: String,
}

/// One value of a kind's option.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KindValue {
    pub id: String,
    pub label: String,
    pub owner: String,
}

/// A kind: the option group of the sets that name it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Kind {
    pub kind: String,
    /// The id of its option: `<kind>_variant`.
    pub option: String,
    /// What the option is called in the menu, if a set says; else the kind, spaced and capitalised.
    pub label: String,
    pub values: Vec<KindValue>,
}

/// Every declared set.
#[derive(Debug, Clone, Default)]
pub struct Catalog {
    pub sets: Vec<SetRecord>,
    pub refused: Vec<Refused>,
}

impl Catalog {
    /// The sets of `mods`, in load order and then file-name order. A manifest that is refused is logged and left out; so is a second
    /// set with the same kind and value (the first mod keeps it).
    pub fn scan(mods: &[ModDir]) -> Self {
        let mut catalog = Self::default();
        catalog.scan_into(mods, true);
        catalog
    }

    /// Read the manifests again, keeping the `Arc` of every set that is unchanged (so the sets in effect are not read again), and
    /// without logging what is refused a second time.
    pub fn rescan(&mut self, mods: &[ModDir]) {
        let mut fresh = Self::default();
        fresh.scan_into(mods, false);
        for record in &mut fresh.sets {
            let same =
                |old: &&SetRecord| old.owner == record.owner && old.set.id == record.set.id && *old.set == *record.set;
            if let Some(old) = self.sets.iter().find(same) {
                record.set = old.set.clone();
            }
        }
        *self = fresh;
    }

    fn scan_into(&mut self, mods: &[ModDir], log: bool) {
        for (order, m) in mods.iter().enumerate() {
            for path in toml_files(&m.dir.join(REPLACE_FOLDER)) {
                self.read(m, order, path, log);
            }
        }
    }

    fn read(&mut self, m: &ModDir, order: usize, path: PathBuf, log: bool) {
        let Some(stem) = path.file_stem().and_then(|stem| stem.to_str()).map(str::to_owned) else { return };
        let result = std::fs::read_to_string(&path)
            .map_err(|error| error.to_string())
            .and_then(|text| parse_replace_set(&stem, &text).map_err(|error| error.to_string()))
            .and_then(|set| self.accept(set));
        match result {
            Ok(set) => self.sets.push(SetRecord { owner: m.id.clone(), order, set: Arc::new(set) }),
            Err(reason) => {
                if log {
                    log::warn!("mod {}: replacement set {} is not added: {reason}", m.id, path.display());
                }
                self.refused.push(Refused { owner: m.id.clone(), path, reason });
            }
        }
    }

    /// The set when this host can take it.
    fn accept(&self, set: ReplaceSet) -> Result<ReplaceSet, String> {
        if let Some(game) = set.source_game.as_deref().filter(|game| !READABLE_GAMES.contains(game)) {
            return Err(format!("it is made for {game:?}, which this game has no reader for"));
        }
        if !set.always
            && let Some(taken) = self.find(&set.kind, &set.value)
        {
            return Err(format!(
                "the value {:?} of {:?} is already provided by {}",
                set.value,
                set.kind,
                taken.label()
            ));
        }
        Ok(set)
    }

    /// The set that provides `value` of `kind`.
    pub fn find(&self, kind: &str, value: &str) -> Option<&SetRecord> {
        self.sets.iter().find(|record| !record.set.always && record.set.kind == kind && record.set.value == value)
    }

    /// The sets that apply with no option.
    pub fn always(&self) -> impl Iterator<Item = &SetRecord> {
        self.sets.iter().filter(|record| record.set.always)
    }

    /// The kinds the sets declare, in the order they first appear, with the values each offers.
    pub fn kinds(&self) -> Vec<Kind> {
        let mut kinds: Vec<Kind> = Vec::new();
        for record in self.sets.iter().filter(|record| !record.set.always) {
            let set = &record.set;
            let at = match kinds.iter().position(|kind| kind.kind == set.kind) {
                Some(at) => at,
                None => {
                    kinds.push(Kind {
                        kind: set.kind.clone(),
                        option: format!("{}_variant", set.kind),
                        label: humanise(&set.kind),
                        values: Vec::new(),
                    });
                    kinds.len() - 1
                }
            };
            if let Some(label) = &set.kind_label
                && kinds[at].values.is_empty()
            {
                kinds[at].label.clone_from(label);
            }
            kinds[at].values.push(KindValue {
                id: set.value.clone(),
                label: set.value_label.clone(),
                owner: record.owner.clone(),
            });
        }
        kinds
    }
}

/// `street_lamps` as `Street lamps`.
fn humanise(kind: &str) -> String {
    let spaced = kind.replace('_', " ");
    let mut chars = spaced.chars();
    chars.next().map(|first| first.to_uppercase().chain(chars).collect()).unwrap_or_default()
}

#[cfg(test)]
#[path = "catalog_tests.rs"]
mod tests;

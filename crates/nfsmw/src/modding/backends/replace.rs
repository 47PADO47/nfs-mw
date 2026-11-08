//! Handing the replacement sets the player chose to the world's loaders (`docs/modding/api-replace.md`).
//!
//! Every kind of set (`trees`, `bushes`, `street_lamps`, any name a mod gives) is one list option, `<kind>_variant`, in the Gameplay
//! options: the game's own `original` and a value for each set a mod offers. [`Bridge::declare_replacements`] reads the manifests the
//! loaded mods keep in `replace/` and adds the values to the options (a mod needs no code for it); [`Bridge::publish_replacements`] runs
//! every frame, works out which sets the choices (and the `always` sets) make, and when they changed reads them and makes their merge the
//! replacements in effect (`content::replace::set_active`). A mod built before `bb.content` 1.2 that adds a value to `trees_variant` itself
//! and ships `TREES/<value>/profile.toml` is read as a set of kind `trees` (`from_model_swap`): one mechanism.

use std::sync::Arc;

use blackbox_modding::content::{LEGACY_KIND, ModelSwapProfile, ReplaceSet, from_model_swap};
use blackbox_modding::host::{OptionKind, OptionSpec, ValueSpec};

use super::bridge::Bridge;
use super::settings::{self, ORIGINAL};
use super::variants::PROFILE_RETRY_CALLS;
use crate::modding::content::{self, ModDir, replace};

/// What the text of the `original` value of a kind a mod declares is, when the mod's catalogue has no message of that name.
const ORIGINAL_LABEL: &str = "Original";

/// The state of the replacements the bridge keeps.
#[derive(Default)]
pub(super) struct ReplaceState {
    /// The sets the loaded mods declare.
    catalog: replace::Catalog,
    /// The sets in effect, as the addresses of their `Arc`s (a set read again is another `Arc`).
    published: Vec<usize>,
    /// Calls left before the manifests are read again, while a chosen value has no set.
    retry_in: u32,
    /// The old trees profile turned into a set, and the profile it was made from.
    legacy: Option<(Arc<ModelSwapProfile>, Arc<ReplaceSet>)>,
    /// What became of each set at the last publish, for the report.
    status: Vec<(String, String)>,
}

/// A set the choices make, with the mod that provides it and its place in the load order.
struct Selected {
    owner: String,
    order: usize,
    set: Arc<ReplaceSet>,
}

impl Bridge {
    /// Read the replacement manifests of `mods` and add each kind's values to its option (`<kind>_variant`). The option is created
    /// when no earlier mod or the game has it; a value an option already has is left as it is. Call once the mods' folders are known.
    pub fn declare_replacements(&self, mods: &[ModDir]) {
        let catalog = replace::Catalog::scan(mods);
        for kind in catalog.kinds() {
            let mut owners: Vec<&str> = Vec::new();
            for value in &kind.values {
                if !owners.contains(&value.owner.as_str()) {
                    owners.push(&value.owner);
                }
            }
            for owner in owners {
                let mut values = vec![ValueSpec { id: ORIGINAL, label: ORIGINAL_LABEL }];
                values.extend(
                    kind.values
                        .iter()
                        .filter(|value| value.owner == owner)
                        .map(|value| ValueSpec { id: &value.id, label: &value.label }),
                );
                let spec = OptionSpec {
                    id: &kind.option,
                    kind: OptionKind::List,
                    label: &kind.label,
                    values,
                    default_on: false,
                };
                if let Err(error) = self.options.borrow_mut().add_option(owner, &spec) {
                    log::warn!(
                        "mod {owner}: the option {:?} of the replacement sets is refused: {error:?}",
                        kind.option
                    );
                }
            }
        }
        self.option_choices.borrow_mut().clear();
        self.replace.borrow_mut().catalog = catalog;
    }

    /// The kinds the choices are read for: the game's (the trees) and every kind a set declares.
    fn replace_kinds(&self) -> Vec<String> {
        let mut kinds = vec![LEGACY_KIND.to_owned()];
        for record in &self.replace.borrow().catalog.sets {
            if !record.set.always && !kinds.contains(&record.set.kind) {
                kinds.push(record.set.kind.clone());
            }
        }
        kinds
    }

    /// The sets the player's choices make: for each kind the set of the chosen value (none for `original`), and the `always` sets.
    /// The second value says that a chosen value has no set yet (a mod may still be writing it).
    fn selected_sets(&self) -> (Vec<Selected>, bool) {
        let mut selected = Vec::new();
        let mut missing = false;
        for kind in self.replace_kinds() {
            let option = format!("{kind}_variant");
            let Some(value) = settings::chosen(self, &option) else { continue };
            if value == ORIGINAL {
                continue;
            }
            let declared = self.replace.borrow().catalog.find(&kind, &value).cloned();
            if let Some(record) = declared {
                selected.push(Selected { owner: record.owner, order: record.order, set: record.set });
                continue;
            }
            let owner = self.options.borrow().value_owner(&option, &value).map(str::to_owned);
            let legacy = (kind == LEGACY_KIND).then(|| self.legacy_set(&value)).flatten();
            match (legacy, owner) {
                (Some(set), Some(owner)) => {
                    let order = content::mod_dirs().iter().position(|m| m.id == owner).unwrap_or(0);
                    selected.push(Selected { owner, order, set });
                }
                _ => missing = true,
            }
        }
        let always = self.replace.borrow().catalog.always().cloned().collect::<Vec<_>>();
        selected.extend(always.into_iter().map(|record| Selected {
            owner: record.owner,
            order: record.order,
            set: record.set,
        }));
        (selected, missing)
    }

    /// The old trees profile of the chosen value as a set (made once for each profile read).
    fn legacy_set(&self, value: &str) -> Option<Arc<ReplaceSet>> {
        let profile = self.trees_profile()?;
        let mut state = self.replace.borrow_mut();
        if let Some((have, set)) = &state.legacy
            && Arc::ptr_eq(have, &profile)
        {
            return Some(set.clone());
        }
        let set = Arc::new(from_model_swap(value, &profile));
        state.legacy = Some((profile, set.clone()));
        Some(set)
    }

    /// Make the sets the player chose the replacements the world applies. Runs every frame and works only when the choices, or the
    /// sets read for them, changed since the last call. A set that cannot be read is logged and left out; the rest stand.
    pub fn publish_replacements(&self) {
        let (selected, missing) = self.selected_sets();
        if missing {
            let mut state = self.replace.borrow_mut();
            state.retry_in = state.retry_in.saturating_sub(1);
            if state.retry_in == 0 {
                state.retry_in = PROFILE_RETRY_CALLS;
                state.catalog.rescan(content::mod_dirs());
            }
        }
        let signature: Vec<usize> = selected.iter().map(|chosen| Arc::as_ptr(&chosen.set) as usize).collect();
        if signature == self.replace.borrow().published {
            return;
        }
        let Some(dir) = self.install() else { return };
        self.replace.borrow_mut().published = signature;
        let mut loaded = Vec::new();
        let mut status = Vec::new();
        for chosen in selected {
            let label = format!("{}:{}", chosen.owner, chosen.set.id);
            match replace::load_set(&dir, chosen.set.clone(), &chosen.owner, chosen.order) {
                Ok(set) => {
                    status.push((label, "chosen".to_owned()));
                    loaded.push(set);
                }
                Err(error) => {
                    log::warn!("replacements: the set {label} is not read, the game's own stays: {error:#}");
                    status.push((label, format!("not read: {error:#}")));
                }
            }
        }
        self.replace.borrow_mut().status = status;
        if loaded.is_empty() {
            replace::set_active(None);
            return;
        }
        let active = replace::Active::build(loaded);
        log::info!(
            "replacements: {} ({} models, {} textures replaced, {} textures added)",
            active.sets().join(", "),
            active.swaps(),
            active.replaced_textures(),
            active.textures().len()
        );
        replace::set_active(Some(Arc::new(active)));
    }

    /// One line for every set the mod `owner` declares: whether it is chosen, in effect, refused or not read (`bb.content` 1.2, `report`).
    pub fn replacement_report(&self, owner: &str) -> String {
        let state = self.replace.borrow();
        let mut lines = Vec::new();
        for refused in state.catalog.refused.iter().filter(|refused| refused.owner == owner) {
            let name = refused.path.file_name().and_then(|name| name.to_str()).unwrap_or("?");
            lines.push(format!("{name}: refused: {}", refused.reason));
        }
        for record in state.catalog.sets.iter().filter(|record| record.owner == owner) {
            let label = record.label();
            let what = match state.status.iter().find(|(known, _)| *known == label) {
                Some((_, what)) => what.clone(),
                None => "not chosen".to_owned(),
            };
            lines.push(format!("{}: {what}", record.set.id));
        }
        lines.join("\n")
    }
}

#[cfg(test)]
#[path = "replace_tests.rs"]
mod tests;

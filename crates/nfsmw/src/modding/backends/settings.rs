//! `bb.settings`: the options of the gameplay settings that mods add to, and the value the player chose for each.
//!
//! The game keeps an [`OptionsModel`] on the bridge, holding the options it has (the nitrous variant, which lists only
//! "Original", and the rows it draws itself, known by id and kind so that a clash of kinds is seen). A mod's
//! `add_option` is applied by the model's rules (`blackbox_modding::host::options`). The chosen value of an option is
//! kept in the config under `[mod_values.nfsmw]`, key `option_<id>`, as text, and is read back by [`chosen`]; a value
//! that the options no longer have (its mod is not loaded) reads as the first one.
//!
//! The Gameplay page draws the options that are visible (a list with more than one value, a toggle) after its own rows
//! (`frontend/mod_options.rs`), and a player's change comes back through [`change`].

use std::rc::Rc;

use blackbox_modding::abi::{BB_ERR_ARG, BB_ERR_EXISTS, BB_ERR_NOT_FOUND};
use blackbox_modding::host::{
    AddError, BackendResult, Merge, OptionDef, OptionKind, OptionSpec, SettingHandler, SettingsBackend,
};
use toml::Value;

use super::bridge::Bridge;

/// The owner the game keeps its own saved values under (also the mod theme choice, `ui.rs`).
const STORE_OWNER: &str = "nfsmw";

/// The id of the nitrous variant option.
pub const NITROUS_VARIANT: &str = "nitrous_variant";
/// The value id of the 2005 game's own nitrous effects: the only value the game itself provides.
pub const ORIGINAL: &str = "original";

/// The `bb.settings` port of the game: one per mod table, sharing the bridge.
pub struct SettingsPort(pub(super) Rc<Bridge>);

/// The callback a mod gave for an option.
pub struct OptionHandler {
    pub handle: u64,
    pub option: String,
    /// Shared, so a change can run it while the model is free to change.
    pub on_change: Rc<SettingHandler>,
}

/// The options the game has before any mod is loaded. The nitrous variant is the game's own list with its one value.
pub fn builtin() -> Vec<OptionDef> {
    let nitrous = OptionDef::game_list(
        NITROUS_VARIANT,
        "options-nitrous-variant",
        &[(ORIGINAL, "options-nitrous-variant-original")],
    );
    let lists = ["hud_layout", "ui_style", "ui_theme", "speed_units", "minimap", "language"];
    let toggles = ["rear_view_mirror", "turn_signals", "cop_health_bars"];
    let mut options = vec![nitrous];
    options.extend(lists.iter().map(|id| OptionDef::fixed(id, OptionKind::List)));
    options.extend(toggles.iter().map(|id| OptionDef::fixed(id, OptionKind::Toggle)));
    options
}

/// The key an option's chosen value is saved under.
fn store_key(option: &str) -> String {
    format!("option_{option}")
}

impl SettingsBackend for SettingsPort {
    fn add_option(
        &self,
        owner: &str,
        spec: &OptionSpec<'_>,
        on_change: Option<SettingHandler>,
    ) -> BackendResult<u64> {
        let added = self.0.options.borrow_mut().add_option(owner, spec);
        let (handle, merge) = match added {
            Ok(done) => done,
            Err(AddError::KindMismatch { existing }) => {
                log::warn!(
                    "mod {owner}: the option {:?} is {existing:?} in the game, not {:?}; the mod's option is ignored",
                    spec.id,
                    spec.kind
                );
                return Err(BB_ERR_EXISTS);
            }
            Err(AddError::Invalid(why)) => {
                log::warn!("mod {owner}: the option {:?} is refused: {why}", spec.id);
                return Err(BB_ERR_ARG);
            }
        };
        if merge == Merge::Fixed {
            log::warn!("mod {owner}: the values of {:?} are fixed by the game; the mod's values are ignored", spec.id);
        }
        self.0.option_choices.borrow_mut().remove(spec.id);
        if let Some(handler) = on_change {
            let record = OptionHandler { handle, option: spec.id.to_owned(), on_change: Rc::new(handler) };
            self.0.option_handlers.borrow_mut().push(record);
        }
        Ok(handle)
    }

    fn remove_option(&self, owner: &str, handle: u64) -> BackendResult<()> {
        if !self.0.options.borrow_mut().remove(owner, handle) {
            return Err(BB_ERR_NOT_FOUND);
        }
        self.0.option_handlers.borrow_mut().retain(|record| record.handle != handle);
        // The values that left may have been the chosen one, so every choice is read again.
        self.0.option_choices.borrow_mut().clear();
        Ok(())
    }

    fn get(&self, _owner: &str, option: &str) -> BackendResult<String> {
        chosen(&self.0, option).ok_or(BB_ERR_NOT_FOUND)
    }
}

/// The value the player chose for `option` (saved, or the default when nothing valid is saved). `None` when no option has
/// that id.
pub fn chosen(bridge: &Bridge, option: &str) -> Option<String> {
    if let Some(known) = bridge.option_choices.borrow().get(option) {
        return Some(known.clone());
    }
    let value = {
        let model = bridge.options.borrow();
        let def = model.get(option)?;
        let saved = bridge.store.saved_text(STORE_OWNER, &store_key(option));
        def.resolve(saved.as_deref())
    };
    bridge.option_choices.borrow_mut().insert(option.to_owned(), value.clone());
    Some(value)
}

/// The nitrous variant in effect: the chosen value, and the mod that provides it (`None` for the game's own "Original").
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VariantChoice {
    pub value: String,
    pub owner: Option<String>,
}

/// The nitrous variant the player chose. Without a mod that adds one, it is always the original.
pub fn nitrous_variant(bridge: &Bridge) -> VariantChoice {
    let value = chosen(bridge, NITROUS_VARIANT).unwrap_or_else(|| ORIGINAL.to_owned());
    let owner = bridge.options.borrow().value_owner(NITROUS_VARIANT, &value).map(str::to_owned);
    VariantChoice { value, owner }
}

/// An option as the Gameplay page shows it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OptionInfo {
    /// The option's position in the model, which the page uses to name the row while it is open.
    pub index: usize,
    pub label: String,
    pub toggle: bool,
    /// A toggle's state.
    pub on: bool,
    /// A list's chosen value, as text in the player's language.
    pub text: String,
}

/// A text of the game (a catalogue message id) or of a mod (shown from the mod's catalogue, else as written).
fn shown_text(owner: Option<&str>, text: &str) -> String {
    match owner {
        Some(owner) => crate::i18n::mod_label(owner, text),
        None => crate::i18n::text(text, &crate::i18n::Args::new()),
    }
}

/// The positions of the options the page draws, in order: those the rule shows (`OptionDef::visible`).
pub fn visible(bridge: &Bridge) -> Vec<usize> {
    let model = bridge.options.borrow();
    model.options().iter().enumerate().filter(|(_, option)| option.visible()).map(|(at, _)| at).collect()
}

/// The option at `index` as the page shows it, or `None` when it is gone.
pub fn info(bridge: &Bridge, index: usize) -> Option<OptionInfo> {
    let (id, label, owner, kind, values) = {
        let model = bridge.options.borrow();
        let def = model.options().get(index)?;
        (def.id.clone(), def.label.clone(), def.owner.clone(), def.kind, def.values.clone())
    };
    let value = chosen(bridge, &id)?;
    let text = values
        .iter()
        .find(|known| known.id == value)
        .map(|known| shown_text(known.owner.as_deref(), &known.label))
        .unwrap_or_default();
    Some(OptionInfo {
        index,
        label: shown_text(owner.as_deref(), &label),
        toggle: kind == OptionKind::Toggle,
        on: value == "on",
        text,
    })
}

/// A player's change to the option at `index`: one step forward or back. The value is kept and saved, and the callbacks the
/// mods gave for the option run with the new value.
pub fn change(bridge: &Bridge, index: usize, forward: bool) {
    let (id, next) = {
        let model = bridge.options.borrow();
        let Some(def) = model.options().get(index) else { return };
        let Some(current) = chosen(bridge, &def.id) else { return };
        (def.id.clone(), def.step(&current, forward))
    };
    if next.is_empty() {
        return;
    }
    bridge.option_choices.borrow_mut().insert(id.clone(), next.clone());
    if let Err(error) = bridge.store.save(STORE_OWNER, &store_key(&id), Value::String(next.clone())) {
        log::warn!("the choice for {id:?} is not saved: {error}");
    }
    let handlers: Vec<Rc<SettingHandler>> = bridge
        .option_handlers
        .borrow()
        .iter()
        .filter(|record| record.option == id)
        .map(|record| Rc::clone(&record.on_change))
        .collect();
    for handler in handlers {
        (*handler)(&next);
    }
}

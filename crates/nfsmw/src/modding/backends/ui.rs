//! `bb.ui`: the themes a mod registers, and the draw specs of the slots they override.
//!
//! The port records each theme and its slots as plain data on the bridge. It draws nothing: the theme registry
//! (`ui/theme/modded`) reads the records each frame and makes them `blackbox_ui` themes. A slot is refused unless it is in
//! the game's element catalogue ([`ElementId`]), so a mod can only replace elements the game has named.

use std::rc::Rc;

use blackbox_modding::abi::{BB_ERR_ARG, BB_ERR_EXISTS, BB_ERR_NOT_FOUND};
use blackbox_modding::host::{BackendResult, MAX_PRIMITIVES, UiBackend, UiPrim, UiThemeSpec, UiTokens};
use blackbox_ui::{SlotId, Theme, ThemeId, Tokens};
use toml::Value;

use super::bridge::Bridge;
use super::is_plain_name;
use crate::ui::theme::ElementId;
use crate::ui::theme::modded::ModTheme;

/// The `bb.ui` port of the game: one per mod table, sharing the bridge.
pub struct UiPort(pub(super) Rc<Bridge>);

/// A theme a mod registered: its handle, the mod that owns it, and the theme itself.
pub struct UiRecord {
    pub handle: u64,
    pub owner: String,
    pub theme: ModTheme,
}

/// The id a theme of `owner` named `name` has in the registry: a 32-bit FNV-1a hash of `owner/name`, so it is the same in
/// every run and does not depend on the order the mods loaded in.
pub fn theme_id(owner: &str, name: &str) -> ThemeId {
    let mut hash: u32 = 0x811c_9dc5;
    for byte in owner.bytes().chain(std::iter::once(b'/')).chain(name.bytes()) {
        hash ^= u32::from(byte);
        hash = hash.wrapping_mul(0x0100_0193);
    }
    ThemeId::Mod(hash)
}

/// The owner the game keeps its own saved values under, in the config's `[mod_values]` table, and the row that holds
/// the mod theme choice. The value is `owner/name` of the chosen theme, or empty for the built-in style.
const UI_OWNER: &str = "nfsmw";
const CHOICE_ROW: &str = "ui_style_mod";

/// The stable identity of a mod theme: `owner/name`. The id hash is derived from it (`theme_id`), so the same theme
/// keeps the same id in every run, and the saved choice names the theme by this text.
fn stable_key(owner: &str, name: &str) -> String {
    format!("{owner}/{name}")
}

/// The themes the mods registered, as the registry holds them.
pub fn themes(bridge: &Bridge) -> Vec<ModTheme> {
    bridge.ui_themes.borrow().iter().map(|record| record.theme.clone()).collect()
}

/// Whether any mod registered a theme. The Mods page offers the theme choice only then.
pub fn any_theme(bridge: &Bridge) -> bool {
    !bridge.ui_themes.borrow().is_empty()
}

/// The id of the mod theme the player chose, or `None` for the built-in style.
pub fn choice(bridge: &Bridge) -> Option<ThemeId> {
    bridge.ui_theme_choice.get()
}

/// The display text of the chosen mod theme, or `None` when the built-in style is chosen.
pub fn chosen_display(bridge: &Bridge) -> Option<String> {
    let id = bridge.ui_theme_choice.get()?;
    let records = bridge.ui_themes.borrow();
    records.iter().find(|record| record.theme.id() == id).map(|record| record.theme.display().to_owned())
}

/// Moves the theme choice one step, forward or back: the built-in style, then each mod theme in registration order,
/// and round again. The choice is saved (`[mod_values.nfsmw] ui_style_mod`), so a later run selects it again.
pub fn step_choice(bridge: &Bridge, forward: bool) {
    let (choice, saved) = {
        let records = bridge.ui_themes.borrow();
        let count = records.len() + 1;
        let at = bridge
            .ui_theme_choice
            .get()
            .and_then(|id| records.iter().position(|record| record.theme.id() == id))
            .map_or(0, |index| index + 1);
        let next = match forward {
            true => (at + 1) % count,
            false => (at + count - 1) % count,
        };
        match next.checked_sub(1).and_then(|index| records.get(index)) {
            Some(record) => (Some(record.theme.id()), stable_key(&record.owner, record.theme.name())),
            None => (None, String::new()),
        }
    };
    bridge.ui_theme_choice.set(choice);
    if let Err(error) = bridge.store.save(UI_OWNER, CHOICE_ROW, Value::String(saved)) {
        log::warn!("the mod theme choice is not saved: {error}");
    }
}

/// The tokens the mod sent, as the theme registry holds them.
fn to_tokens(tokens: &UiTokens) -> Tokens {
    Tokens {
        primary: tokens.primary,
        secondary: tokens.secondary,
        background: tokens.background,
        border: tokens.border,
        muted: tokens.muted,
        radius: tokens.radius,
        padding: tokens.padding,
        // The mod ABI has no text scale, so a mod's text keeps its own size.
        text_scale: 1.0,
    }
}

/// A font or texture name a primitive may use: empty for none, else a name with no whitespace.
fn name_ok(name: &str) -> bool {
    name.is_empty() || is_plain_name(name)
}

/// Whether a primitive names only things the host can look up. Shape (sizes, finite numbers, kinds) was checked by the
/// table; this checks the names.
fn prim_ok(prim: &UiPrim) -> bool {
    match prim {
        UiPrim::Panel { texture, .. } => texture.as_deref().is_none_or(name_ok),
        UiPrim::Text { font, .. } => name_ok(font),
        UiPrim::Image { texture, .. } => is_plain_name(texture),
    }
}

impl UiBackend for UiPort {
    fn register_theme(&self, owner: &str, theme: &UiThemeSpec<'_>) -> BackendResult<u64> {
        if !is_plain_name(theme.name) || theme.display.is_empty() {
            return Err(BB_ERR_ARG);
        }
        let id = theme_id(owner, theme.name);
        let mut records = self.0.ui_themes.borrow_mut();
        let known = records
            .iter()
            .any(|record| (record.owner == owner && record.theme.name() == theme.name) || record.theme.id() == id);
        if known {
            return Err(BB_ERR_EXISTS);
        }
        let handle = self.0.next_handle();
        let new_theme = ModTheme::new(id, theme.name, theme.display);
        records.push(UiRecord { handle, owner: owner.to_owned(), theme: new_theme });
        // A choice saved by an earlier run selects its theme again, once the theme has registered. A choice for a theme
        // that is absent never matches, so the built-in style stays.
        let saved = self.0.store.saved_text(UI_OWNER, CHOICE_ROW);
        if saved.as_deref() == Some(stable_key(owner, theme.name).as_str()) {
            self.0.ui_theme_choice.set(Some(id));
        }
        Ok(handle)
    }

    fn set_slot(&self, owner: &str, theme: u64, slot: u32, prims: &[UiPrim]) -> BackendResult<()> {
        let element = ElementId::from_slot(SlotId::new(slot)).ok_or(BB_ERR_ARG)?;
        if prims.is_empty() || prims.len() > MAX_PRIMITIVES || !prims.iter().all(prim_ok) {
            return Err(BB_ERR_ARG);
        }
        let mut records = self.0.ui_themes.borrow_mut();
        let record = records.iter_mut().find(|record| record.handle == theme && record.owner == owner);
        let record = record.ok_or(BB_ERR_NOT_FOUND)?;
        record.theme.set_slot(element.slot(), prims.to_vec());
        Ok(())
    }

    fn set_tokens(&self, owner: &str, theme: u64, tokens: &UiTokens) -> BackendResult<()> {
        let mut records = self.0.ui_themes.borrow_mut();
        let record = records.iter_mut().find(|record| record.handle == theme && record.owner == owner);
        let record = record.ok_or(BB_ERR_NOT_FOUND)?;
        record.theme.set_tokens(to_tokens(tokens));
        Ok(())
    }

    fn unregister_theme(&self, owner: &str, theme: u64) -> BackendResult<()> {
        let mut records = self.0.ui_themes.borrow_mut();
        let before = records.len();
        records.retain(|record| !(record.handle == theme && record.owner == owner));
        match records.len() == before {
            true => Err(BB_ERR_NOT_FOUND),
            false => Ok(()),
        }
    }
}

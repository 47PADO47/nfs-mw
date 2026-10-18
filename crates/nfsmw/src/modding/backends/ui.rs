//! `bb.ui`: the themes a mod registers, and the draw specs of the slots they override.
//!
//! The port records each theme and its slots as plain data on the bridge. It draws nothing: the theme registry
//! (`ui/theme/modded`) reads the records each frame and makes them `blackbox_ui` themes. A slot is refused unless it is in
//! the game's element catalogue ([`ElementId`]), so a mod can only replace elements the game has named.

use std::rc::Rc;

use blackbox_modding::abi::{BB_ERR_ARG, BB_ERR_EXISTS, BB_ERR_NOT_FOUND};
use blackbox_modding::host::{BackendResult, MAX_PRIMITIVES, UiBackend, UiPrim, UiThemeSpec};
use blackbox_ui::{SlotId, Theme, ThemeId};

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

/// The themes the mods registered, as the registry holds them.
pub fn themes(bridge: &Bridge) -> Vec<ModTheme> {
    bridge.ui_themes.borrow().iter().map(|record| record.theme.clone()).collect()
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

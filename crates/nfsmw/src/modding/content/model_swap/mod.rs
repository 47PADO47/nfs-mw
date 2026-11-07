//! Replacing models of the world by a mod's (the trees variant, `docs/modding/effect-variants.md`).
//!
//! The player's choice of a trees variant (`trees_variant`) names a [`Pack`]: models and textures a mod ships, and which solid
//! of the game each replaces. [`set_active`] says which pack is in effect; the loader threads of the world's stream call
//! [`rewrite`] on every section they have parsed (`Streamer::set_rewrite`), and a section's solids that the pack replaces are
//! swapped for the fitted models before the section is decoded and uploaded. A section that is already resident keeps what it
//! has: a change of the choice shows in the sections that load after it, and in all of them at the next drive.

mod fit;
mod pack;
#[cfg(test)]
mod tests;

use std::sync::{Arc, RwLock};

use nfsmw_data::world::SectionData;

pub use pack::Pack;

/// The pack in effect: `None` for the game's own models.
static ACTIVE: RwLock<Option<Arc<Pack>>> = RwLock::new(None);

/// Make `pack` the pack in effect, or none.
pub fn set_active(pack: Option<Arc<Pack>>) {
    if let Ok(mut slot) = ACTIVE.write() {
        *slot = pack;
    }
}

/// The pack in effect.
pub fn active() -> Option<Arc<Pack>> {
    ACTIVE.read().ok().and_then(|slot| slot.clone())
}

/// What a loader thread runs on a section it has parsed: the pack in effect, if there is one, replaces the solids it has
/// models for.
pub fn rewrite(data: &mut SectionData) {
    let Some(pack) = active() else { return };
    pack.apply(data);
}

/// Tests that change the pack in effect hold this first: the slot is one for the whole process, and tests run side by side.
#[cfg(test)]
pub(crate) static TEST_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

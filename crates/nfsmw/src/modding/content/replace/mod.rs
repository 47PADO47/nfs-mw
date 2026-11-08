//! Replacing and adding models and textures of the world by mods' (`docs/modding/api-replace.md`, `docs/specs/mod-content-replacement.md`):
//! the one mechanism behind the trees variant, the other kinds of model sets and the texture replacements.
//!
//! The player's choice for each kind (`<kind>_variant`) names a set (`blackbox_modding::content::ReplaceSet`); the sets in effect are
//! read, merged ([`Active::build`]) and made the [`Active`] replacements. The loader threads of the world's stream call [`rewrite`] on
//! every section they have parsed (`Streamer::set_rewrite`): the solids the sets replace are swapped for the fitted models and the
//! textures replaced by the sets' pictures before the section is decoded and uploaded. The textures the sets **add** are not put into the
//! sections: the residency uploads them once, into the shared sets (`scenes/world/residency/replace.rs`), and every section's solids
//! resolve them there. A section that is already resident is streamed again when the replacements change, so a change of the choice
//! shows in a few seconds.
//!
//! | File | What |
//! |---|---|
//! | `catalog.rs` | the manifests of the loaded mods, grouped by kind |
//! | `load.rs` | one set read from its files |
//! | `active.rs` | the merged replacements, and what a section gets |
//! | `textures.rs` | pictures: PNG, DDS and texture packs as textures, and the rules of a replacing texture |
//! | `fit.rs`, `place.rs` | fitting a model into the solid it replaces: `height`, `row`, `stand`, `exact` |

mod active;
mod catalog;
mod fit;
mod load;
mod place;
#[cfg(test)]
mod real_tests;
#[cfg(test)]
mod tests;
mod textures;

use std::sync::{Arc, RwLock};

use nfsmw_data::world::{GlobalTextures, SectionData};

pub use active::Active;
pub use catalog::Catalog;
pub use load::load_set;

/// The replacements in effect: `None` for the game's own models and textures.
static ACTIVE: RwLock<Option<Arc<Active>>> = RwLock::new(None);

/// Make `active` the replacements in effect, or none.
pub fn set_active(active: Option<Arc<Active>>) {
    if let Ok(mut slot) = ACTIVE.write() {
        *slot = active;
    }
}

/// The replacements in effect.
pub fn active() -> Option<Arc<Active>> {
    ACTIVE.read().ok().and_then(|slot| slot.clone())
}

/// What a loader thread runs on a section it has parsed: the replacements in effect, if there are any, are applied to it.
pub fn rewrite(data: &mut SectionData) {
    let Some(active) = active() else { return };
    active.apply(data);
}

/// The global textures with the replacements in effect applied (the ones the sets replace) and the textures the sets add.
pub fn apply_to_globals(globals: &mut GlobalTextures) {
    let Some(active) = active() else { return };
    active.replace_textures(&mut globals.textures);
    globals.textures.extend(active.textures().iter().cloned());
}

/// Tests that change the replacements in effect hold this first: the slot is one for the whole process, and tests run side by side.
#[cfg(test)]
pub(crate) static TEST_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

//! Handing the trees variant to the world's loaders (`docs/modding/effect-variants.md`, "Profile format of the trees").
//!
//! The choice of the `trees_variant` option is read like the other variants' (`Bridge::trees_profile`), but the world does not
//! take it every frame: it swaps solids in the sections its loader threads parse (`content::model_swap`). So [`Bridge::publish_trees`]
//! reads the pack the profile names once for each chosen value and makes it the pack in effect; the original is no pack.

use std::sync::Arc;

use super::bridge::Bridge;
use crate::modding::content::model_swap::{self, Pack};

impl Bridge {
    /// Make the models of the trees variant the player chose the pack the world swaps in, or none for the game's own. Runs every
    /// frame and works only when the choice, or the profile read for it, changed since the last call. A pack that cannot be read
    /// is logged and leaves the game's own trees.
    pub(super) fn publish_trees(&self) {
        let profile = self.trees_profile();
        let unchanged = match (&*self.trees_published.borrow(), &profile) {
            (None, None) => true,
            (Some(have), Some(wanted)) => Arc::ptr_eq(have, wanted),
            _ => false,
        };
        if unchanged {
            return;
        }
        let Some(profile) = profile else {
            *self.trees_published.borrow_mut() = None;
            model_swap::set_active(None);
            return;
        };
        let Some(dir) = self.install() else { return };
        *self.trees_published.borrow_mut() = Some(profile.clone());
        match Pack::load(&dir, &profile) {
            Ok(pack) => {
                log::info!("trees: {} solids of the world take the models of {}", pack.swaps(), profile.geometry);
                model_swap::set_active(Some(Arc::new(pack)));
            }
            Err(error) => {
                log::warn!("trees: the variant's models are not read, the game's own trees stay: {error:#}");
                model_swap::set_active(None);
            }
        }
    }
}

#[cfg(test)]
#[path = "trees_tests.rs"]
mod tests;

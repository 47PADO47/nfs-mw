//! A mod's replacement sets in the residency (`docs/modding/api-replace.md`).
//!
//! The loader threads swap the models and the textures the sets replace in the sections they load
//! ([`crate::modding::content::replace`]); the textures the sets add are not in those sections. They go to the GPU **once**, into the
//! shared sets, and a tile's solids find them there like the textures of the shared library: no tile holds a copy. When the
//! replacements in effect change (the player chose another value of a kind, in a drive too) the resident tiles are streamed again, so the
//! new models and textures show within seconds and no drive has to restart. The shared sets and the global textures were rewritten when
//! they loaded: the textures of the shared sets follow at once, by a redirect (`shared_textures.rs`); their models and the global textures
//! follow at the next start.

use std::sync::Arc;

use super::*;
use crate::modding::content::replace::{self, Active};

/// What tells one set of replacements from another while it lives: where it is. `None` for none.
fn active_id(active: Option<&Arc<Active>>) -> Option<usize> {
    active.map(|active| Arc::as_ptr(active) as usize)
}

/// Add the replacements in effect to the global textures the shared sets upload (the ones they replace are replaced, the ones they add
/// come before the shared models in the order of the upload, and the models of the shared library are rewritten like the tiles').
pub(super) fn add_textures(globals: &mut GlobalTextures) {
    replace::apply_to_globals(globals);
}

impl Residency {
    /// Runs every frame once the shared sets are up, before the arrived tiles are uploaded: when the replacements in effect are not
    /// the ones last seen, uploads the textures they add into the shared sets (a texture that is there already is not uploaded again)
    /// and has the resident tiles streamed again.
    pub(super) fn sync_replace(&mut self, renderer: &mut dyn RenderBackend) {
        let active = replace::active();
        let id = active_id(active.as_ref());
        if id == self.replace_seen {
            return;
        }
        self.replace_seen = id;
        if let Some(active) = &active {
            self.shared.upload_texture_list(renderer, active.textures(), &[]);
        }
        self.plan_shared_textures(active.as_deref());
        self.refresh_resident();
    }

    /// Ask the streamer for every resident map tile again: the sections come back rewritten with the replacements now in effect and
    /// replace what the tiles hold (`upload_arrived`). The collision and the shared sets stay as they are; a multiplayer session
    /// (lockstep) keeps its tiles, because a tile that is uploaded again is not the same moment on every machine.
    fn refresh_resident(&mut self) {
        if self.lockstep {
            return;
        }
        let mod_base = self.mod_base;
        let resident = self
            .tiles
            .iter()
            .filter(|(index, state)| **index < mod_base && matches!(state, TileState::Resident { .. }))
            .map(|(index, _)| *index);
        let mut again: Vec<usize> = resident.collect();
        again.sort_unstable();
        for index in again {
            self.refresh.insert(index);
            self.streamer.request(index);
        }
    }

    /// Whether the arrived section `index` is a tile that was streamed again and is still resident: it replaces the tile.
    pub(super) fn take_refresh(&mut self, index: usize) -> bool {
        self.refresh.remove(&index) && matches!(self.tiles.get(&index), Some(TileState::Resident { .. }))
    }
}

#[cfg(test)]
mod tests {
    use super::super::shadow_tests::fixture;
    use super::*;
    use crate::modding::content::replace::TEST_LOCK;

    /// Replacements with no model and no texture: enough to be in effect.
    fn empty_active() -> Arc<Active> {
        Arc::new(Active::empty())
    }

    #[test]
    fn a_set_of_replacements_is_told_by_where_it_is_and_none_is_none() {
        let (first, second) = (empty_active(), empty_active());
        assert_eq!(active_id(None), None);
        assert_eq!(active_id(Some(&first)), active_id(Some(&first)));
        assert_ne!(active_id(Some(&first)), active_id(Some(&second)));
    }

    #[test]
    fn changed_replacements_stream_the_resident_tiles_again_and_the_same_ones_do_not() {
        let _lock = TEST_LOCK.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
        let (mut residency, mut renderer) = fixture();
        residency.tiles.insert(3, TileState::Resident { resources: Default::default(), placed: vec![], packs: vec![] });
        replace::set_active(None);
        residency.replace_seen = None;
        residency.sync_replace(&mut renderer);
        assert!(residency.refresh.is_empty(), "nothing before and nothing now");
        replace::set_active(Some(empty_active()));
        residency.sync_replace(&mut renderer);
        assert!(residency.refresh.contains(&3), "replacements came: the tile is streamed again");
        assert!(residency.take_refresh(3));
        assert!(!residency.take_refresh(3), "once");
        residency.refresh.clear();
        residency.sync_replace(&mut renderer);
        assert!(residency.refresh.is_empty(), "the same replacements are not looked at twice");
        replace::set_active(None);
        residency.sync_replace(&mut renderer);
        assert!(residency.refresh.contains(&3), "back to the original: streamed again");
        residency.lockstep = true;
        residency.refresh.clear();
        replace::set_active(Some(empty_active()));
        residency.sync_replace(&mut renderer);
        assert!(residency.refresh.is_empty(), "a multiplayer session keeps its tiles");
        replace::set_active(None);
    }

    #[test]
    fn a_refresh_of_a_tile_that_is_gone_is_not_taken() {
        let (mut residency, _) = fixture();
        residency.refresh.insert(77);
        assert!(!residency.take_refresh(77), "no such resident tile");
        assert!(residency.refresh.is_empty());
    }
}

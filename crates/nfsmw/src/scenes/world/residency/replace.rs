//! A mod's trees variant in the residency (`docs/modding/effect-variants.md`, "Profile format of the trees").
//!
//! The loader threads swap the models of the trees in the sections they load ([`crate::modding::content::model_swap`]); the
//! textures of the new models are not in those sections. They go to the GPU **once**, into the shared sets, and a tile's
//! solids find them there like the textures of the shared library: no tile holds a copy. When the pack in effect changes
//! (the player chose another trees variant, in a drive too) the resident tiles are streamed again, so the new models show
//! within seconds and no drive has to restart.

use std::sync::Arc;

use super::*;
use crate::modding::content::model_swap::{self, Pack};

/// What tells one pack from another while it lives: where it is. `None` for no pack.
fn pack_id(pack: Option<&Arc<Pack>>) -> Option<usize> {
    pack.map(|pack| Arc::as_ptr(pack) as usize)
}

/// Add the textures of the pack in effect, if there is one, to the global textures the shared sets upload (they come
/// before the shared models in the order of the upload, and the models of the shared library are rewritten like the tiles').
pub(super) fn add_textures(globals: &mut GlobalTextures) {
    let Some(pack) = model_swap::active() else { return };
    globals.textures.extend(pack.textures().iter().cloned());
}

impl Residency {
    /// Runs every frame once the shared sets are up, before the arrived tiles are uploaded: when the pack in effect is not the
    /// one last seen, uploads its textures into the shared sets (a texture that is there already is not uploaded again) and has
    /// the resident tiles streamed again.
    pub(super) fn sync_trees(&mut self, renderer: &mut dyn RenderBackend) {
        let pack = model_swap::active();
        let id = pack_id(pack.as_ref());
        if id == self.trees_seen {
            return;
        }
        self.trees_seen = id;
        if let Some(pack) = &pack {
            self.shared.upload_texture_list(renderer, pack.textures(), &[]);
        }
        self.refresh_resident();
    }

    /// Ask the streamer for every resident map tile again: the sections come back rewritten with the pack now in effect and
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
    use crate::modding::content::model_swap::TEST_LOCK;

    /// A pack with no model and no texture: enough to be a pack in effect.
    fn empty_pack() -> Arc<Pack> {
        let profile = blackbox_modding::content::parse_model_swap_profile(
            "format = 1
geometry = \"A.BIN\"
textures = \"B.BIN\"
[[swap]]
solid = \"S\"
model = \"M\"
",
        )
        .unwrap();
        Arc::new(Pack::build(&profile, Vec::new(), Vec::new()))
    }

    #[test]
    fn a_pack_is_told_by_where_it_is_and_none_is_none() {
        let (first, second) = (empty_pack(), empty_pack());
        assert_eq!(pack_id(None), None);
        assert_eq!(pack_id(Some(&first)), pack_id(Some(&first)));
        assert_ne!(pack_id(Some(&first)), pack_id(Some(&second)));
    }

    #[test]
    fn a_changed_pack_streams_the_resident_tiles_again_and_the_same_pack_does_not() {
        let _lock = TEST_LOCK.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
        let (mut residency, mut renderer) = fixture();
        residency.tiles.insert(3, TileState::Resident { resources: Default::default(), placed: vec![], packs: vec![] });
        model_swap::set_active(None);
        residency.trees_seen = None;
        residency.sync_trees(&mut renderer);
        assert!(residency.refresh.is_empty(), "no pack before and none now");
        model_swap::set_active(Some(empty_pack()));
        residency.sync_trees(&mut renderer);
        assert!(residency.refresh.contains(&3), "the pack came: the tile is streamed again");
        assert!(residency.take_refresh(3));
        assert!(!residency.take_refresh(3), "once");
        residency.refresh.clear();
        residency.sync_trees(&mut renderer);
        assert!(residency.refresh.is_empty(), "the same pack is not looked at twice");
        model_swap::set_active(None);
        residency.sync_trees(&mut renderer);
        assert!(residency.refresh.contains(&3), "back to the original: streamed again");
        residency.lockstep = true;
        residency.refresh.clear();
        model_swap::set_active(Some(empty_pack()));
        residency.sync_trees(&mut renderer);
        assert!(residency.refresh.is_empty(), "a multiplayer session keeps its tiles");
        model_swap::set_active(None);
    }

    #[test]
    fn a_refresh_of_a_tile_that_is_gone_is_not_taken() {
        let (mut residency, _) = fixture();
        residency.refresh.insert(77);
        assert!(!residency.take_refresh(77), "no such resident tile");
        assert!(residency.refresh.is_empty());
    }
}

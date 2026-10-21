//! The ICE camera packs of `GLOBAL/InGameB.bun`: the NIS pack (`0x8003B200`) and the FMV pack (`0x8003B201`). Each pack
//! holds one set per scene, keyed by the scene name's `bStringHash`, and each set holds the scene's camera tracks named
//! `Track 0`, `Track 1`... Layouts: `docs/formats/animation.md` ("ICE chunk dispatch", "ICE camera records", "ICE camera
//! keys"). The parsing is `blackbox_chunk::ice`; this module only groups its output by scene.

use std::collections::BTreeMap;
use std::sync::Arc;

use anyhow::{Context, Result};
use blackbox_chunk::ice::{Track, parse_camera_pack};
use blackbox_chunk::{find_all, ids};
use blackbox_hash::bstring_hash;
use game_install::GameDir;

use crate::{memo, shared};

/// The sets of one pack: each set's tracks in file order, by the set's key.
type Sets = BTreeMap<u32, Vec<Track>>;

/// The camera tracks of the NIS and FMV packs of `InGameB.bun`.
#[derive(Debug, Clone, Default)]
pub struct CameraLibrary {
    nis: Sets,
    fmv: Sets,
}

impl CameraLibrary {
    /// Reads both packs from the unwrapped bytes of `GLOBAL/InGameB.bun`. A record that does not parse is skipped; a pack
    /// that does not parse is an error. Bytes without a pack give an empty pack.
    pub fn from_bytes(ingame: &[u8]) -> Result<Self> {
        Ok(Self {
            nis: read_pack(ingame, ids::ICE_NIS_CAMERAS, "NIS")?,
            fmv: read_pack(ingame, ids::ICE_FMV_CAMERAS, "FMV")?,
        })
    }

    /// The install's library, read once per session.
    pub fn load(dir: &GameDir) -> Result<Arc<Self>> {
        let ingame = shared::ingame_b(dir)?;
        memo(dir, &[shared::INGAME_B_FILE], || Self::from_bytes(&ingame))
    }

    /// Adds a NIS set under `key` (the `bStringHash` of the scene name), replacing any set with that key. Lets a caller
    /// build a library by hand, as the tests do.
    pub fn insert_nis(&mut self, key: u32, tracks: Vec<Track>) {
        self.nis.insert(key, tracks);
    }

    /// The NIS set of the scene, or `None` when the pack has none for it.
    pub fn nis(&self, scene: &str) -> Option<&[Track]> {
        self.nis_set(bstring_hash(scene))
    }

    /// The NIS set with this key (the `bStringHash` of the scene name).
    pub fn nis_set(&self, key: u32) -> Option<&[Track]> {
        self.nis.get(&key).map(Vec::as_slice)
    }

    /// The FMV set of the scene, or `None` when the pack has none for it.
    pub fn fmv(&self, scene: &str) -> Option<&[Track]> {
        self.fmv_set(bstring_hash(scene))
    }

    /// The FMV set with this key (the `bStringHash` of the scene name).
    pub fn fmv_set(&self, key: u32) -> Option<&[Track]> {
        self.fmv.get(&key).map(Vec::as_slice)
    }

    /// Track `n` (`Track <n>`) of the scene's NIS set.
    pub fn track(&self, scene: &str, n: usize) -> Option<&Track> {
        track_numbered(self.nis(scene)?, n)
    }

    /// Track `n` (`Track <n>`) of the scene's FMV set.
    pub fn fmv_track(&self, scene: &str, n: usize) -> Option<&Track> {
        track_numbered(self.fmv(scene)?, n)
    }

    /// Every NIS set, as (key, tracks), in key order.
    pub fn nis_sets(&self) -> impl Iterator<Item = (u32, &[Track])> + '_ {
        self.nis.iter().map(|(key, tracks)| (*key, tracks.as_slice()))
    }

    /// Every FMV set, as (key, tracks), in key order.
    pub fn fmv_sets(&self) -> impl Iterator<Item = (u32, &[Track])> + '_ {
        self.fmv.iter().map(|(key, tracks)| (*key, tracks.as_slice()))
    }

    /// How many sets the NIS pack holds.
    pub fn nis_count(&self) -> usize {
        self.nis.len()
    }

    /// How many sets the FMV pack holds.
    pub fn fmv_count(&self) -> usize {
        self.fmv.len()
    }
}

/// The track named `Track <n>` in a set.
pub fn track_numbered(set: &[Track], n: usize) -> Option<&Track> {
    let wanted = format!("Track {n}");
    set.iter().find(|t| t.name == wanted)
}

/// Every set of the packs with chunk id `id`, grouped by key. A later set with the same key replaces an earlier one.
fn read_pack(ingame: &[u8], id: u32, name: &str) -> Result<Sets> {
    let mut sets = Sets::new();
    for chunk in find_all(ingame, id) {
        let pack = parse_camera_pack(&chunk).with_context(|| format!("reading the {name} camera pack"))?;
        for set in pack.sets {
            let tracks = set.records.iter().filter_map(Track::from_record).collect();
            sets.insert(set.key, tracks);
        }
    }
    Ok(sets)
}

#[cfg(test)]
#[path = "cameras_tests.rs"]
mod tests;

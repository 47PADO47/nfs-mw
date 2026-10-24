//! What the end of a race shows in the world: the flares at the finish line and the camera of the win photo. Spec:
//! `docs/specs/race-finish-effects.md`.
//!
//! The finish line's `ParticleEffect` names an `emittergroup` collection (`fxgame_flare_red`, four emitters: a core, a
//! glow, smoke and sparks). The win photo's camera is a track of the replay pack of `GLOBAL/InGameB.bun` (the ICE camera
//! pack `0x8003B203`), found by the name a race's `FinishCamera` field gives.

use std::collections::BTreeMap;
use std::sync::Arc;

use anyhow::{Context, Result};
use blackbox_attrib::Database;
use blackbox_chunk::ice::{Track, parse_camera_pack};
use blackbox_chunk::{find_all, ids};
use game_install::GameDir;

use crate::car::exhaust::{GroupEmitter, group_emitters};
use crate::{memo, shared};

/// The replay camera a race with no (or an unknown) `FinishCamera` uses.
pub const DEFAULT_FINISH_CAMERA: &str = "DefaultFinish";
/// The replay camera of a drag race's photo when its data names none (the install's drag races name `DefaultFinish`
/// or a numbered one; `DragFinish` is in the pack all the same).
pub const DRAG_FINISH_CAMERA: &str = "DragFinish";

/// The emitters of the `emittergroup` collection `name` (empty when there is none): the effect a trigger's
/// `ParticleEffect` field names.
pub fn trigger_effect_emitters(db: &Database, name: &str) -> Vec<GroupEmitter> {
    let Some(group) = db.collection("emittergroup", name) else { return Vec::new() };
    group_emitters(db, group)
}

/// The replay cameras of the install, by track name.
#[derive(Debug, Clone, Default)]
pub struct ReplayCameras {
    tracks: BTreeMap<String, Track>,
}

impl ReplayCameras {
    /// Reads the replay camera pack of `InGameB.bun`. A record that does not parse is skipped.
    pub fn read(ingame: &[u8]) -> Result<Self> {
        let mut tracks = BTreeMap::new();
        for chunk in find_all(ingame, ids::ICE_REPLAY_CAMERAS) {
            let pack = parse_camera_pack(&chunk).context("reading the replay camera pack")?;
            for set in pack.sets {
                for track in set.records.iter().filter_map(Track::from_record) {
                    tracks.entry(track.name.clone()).or_insert(track);
                }
            }
        }
        Ok(Self { tracks })
    }

    /// The track of this name, as the pack spells it.
    pub fn track(&self, name: &str) -> Option<&Track> {
        self.tracks.get(name)
    }

    /// How many tracks the pack holds.
    pub fn len(&self) -> usize {
        self.tracks.len()
    }

    pub fn is_empty(&self) -> bool {
        self.tracks.is_empty()
    }

    /// The track a race frames its win photo with: the one its `FinishCamera` names, else the default of its kind.
    pub fn finish_track(&self, named: Option<&str>, drag: bool) -> Option<&Track> {
        let fallback = if drag { DRAG_FINISH_CAMERA } else { DEFAULT_FINISH_CAMERA };
        named.and_then(|name| self.track(name)).or_else(|| self.track(fallback))
    }
}

/// The install's replay cameras, read once per session.
pub fn load_replay_cameras(dir: &GameDir) -> Result<Arc<ReplayCameras>> {
    let ingame = shared::ingame_b(dir)?;
    memo(dir, &[shared::ingame_b_file(dir)], || ReplayCameras::read(&ingame))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn install() -> Option<GameDir> {
        GameDir::open(std::env::var_os("NFSMW_GAME_DIR")?).ok()
    }

    #[test]
    fn a_missing_track_falls_back_to_the_kinds_default() {
        let mut cameras = ReplayCameras::default();
        assert!(cameras.finish_track(Some("PHoldFin_01"), false).is_none());
        let track = Track { name: DEFAULT_FINISH_CAMERA.into(), start: 0.0, length: 3.0, keys: Vec::new() };
        cameras.tracks.insert(track.name.clone(), track);
        assert_eq!(cameras.finish_track(Some("PHoldFin_01"), false).map(|t| t.name.as_str()), Some("DefaultFinish"));
        assert_eq!(cameras.finish_track(None, false).map(|t| t.name.as_str()), Some("DefaultFinish"));
        assert!(cameras.finish_track(None, true).is_none(), "a drag race wants DragFinish, which this set lacks");
    }

    #[test]
    #[ignore = "needs the game (set NFSMW_GAME_DIR)"]
    fn the_install_has_the_finish_cameras_and_the_flare_group() {
        let Some(dir) = install() else { return };
        let cameras = load_replay_cameras(&dir).expect("replay cameras");
        for name in ["DefaultFinish", "DragFinish", "SprintFinish", "TollFinish", "PHoldFin_00", "PHoldFin_28"] {
            let track = cameras.track(name).unwrap_or_else(|| panic!("{name} is in the replay pack"));
            assert!(!track.keys.is_empty());
        }
        let sprint = cameras.track("SprintFinish").unwrap();
        assert_eq!(sprint.length, 3.0);
        assert_eq!(sprint.keys.len(), 2);
        let db = crate::shared::attributes(&dir).unwrap();
        assert_eq!(trigger_effect_emitters(&db, "fxgame_flare_red").len(), 4, "core, glow, smoke and sparks");
        assert!(trigger_effect_emitters(&db, "no_such_effect").is_empty());
    }
}

//! The data of a pursuit breaker's cinematic moment: which marker kind each moment scene plays at, where the track
//! puts those markers, and the camera tracks of the scenes. Spec: `docs/specs/pursuit-breaker-moment.md`.
//!
//! Three installed sources: the scene candidate table in `GLOBAL/InGameB.bun` (`SceneCandidateData`, the moment list
//! at 0xC9C), the camera packs of the same file (NIS pack, one set per scene hash), and the track position markers of
//! `TRACKS/L2RA.BUN` (`TrackPositionMarkers`, 0x30-byte records).

use std::collections::BTreeMap;
use std::sync::Arc;

use anyhow::{Context, Result};
use blackbox_chunk::ice::{Track, parse_camera_pack};
use blackbox_chunk::{find, find_all, ids};
use blackbox_hash::bstring_hash;
use game_install::GameDir;
use glam::Vec3;

use crate::memo;
use crate::shared;

/// The candidate table inside the anim directory.
const CANDIDATE_DATA: u32 = 0x0003_7070;
/// The moment list of the candidate table: a count at 0x14, the records (scene hash, marker kind) from 0xC9C.
const MOMENT_COUNT: usize = 0x14;
const MOMENT_LIST: usize = 0xC9C;
const MOMENT_LIMIT: usize = 64;
/// `TrackPositionMarkers`.
const POSITION_MARKERS: u32 = 0x0003_4146;
const MARKER_LEN: usize = 0x30;

/// The marker names by kind. The first column of the moment list indexes it.
pub const MARKER_NAMES: [&str; 21] = [
    "CameraCutMoment",
    "GasStationMoment",
    "FirehallMoment",
    "PoliceStationMoment",
    "WaterTowerMoment",
    "PorchCollapseMoment",
    "SignageCollapseMoment",
    "ScaffoldingCollapseMoment",
    "GazeboCollapseMoment",
    "FishMarketMoment",
    "BoatKnockdownMoment",
    "CopHitsObstructionMoment",
    "BustedMomentMarker",
    "TollboothMarker",
    "GarageMarker",
    "PoliceStationGroundMoment",
    "DriveInMoment",
    "TrailerParkMoment",
    "TorusRoofMoment",
    "AmphitheatreMoment",
    "StripMallMoment",
];

/// One placed marker. The position is in render space (x forward, y left, z up).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Marker {
    pub position: Vec3,
    /// 16-bit angle, 65536 per turn.
    pub angle: u16,
}

/// What the installed files say about moments.
#[derive(Debug, Clone, Default)]
pub struct MomentData {
    /// Scene hash to marker kind (an index of [`MARKER_NAMES`]).
    pub kinds: BTreeMap<u32, u32>,
    /// Marker name hash to the markers of that name, in file order.
    pub markers: BTreeMap<u32, Vec<Marker>>,
    /// Scene hash to its camera tracks, in file order.
    pub cameras: BTreeMap<u32, Vec<Track>>,
}

/// The hash a scene name has in the files.
pub fn scene_hash(name: &str) -> u32 {
    bstring_hash(name)
}

/// The two inputs of a marker's score: how far it is and how much it faces another way. The score adds up to 12.5 m
/// for a marker turned right around from the heading.
const ANGLE_BONUS: f32 = 25.0;

impl MomentData {
    /// The installed data: candidates and cameras from `InGameB.bun`, markers from `L2RA.BUN`.
    pub fn read(ingame: &[u8], l2ra: &[u8]) -> Result<Self> {
        let mut data = Self::default();
        let candidates = find(ingame, CANDIDATE_DATA).context("no scene candidate table")?;
        data.read_kinds(candidates.payload)?;
        for chunk in [ids::ICE_NIS_CAMERAS] {
            for pack in find_all(ingame, chunk) {
                let pack = parse_camera_pack(&pack).context("reading the NIS camera pack")?;
                for set in pack.sets {
                    let tracks = set.records.iter().filter_map(Track::from_record).collect();
                    data.cameras.insert(set.key, tracks);
                }
            }
        }
        for chunk in find_all(l2ra, POSITION_MARKERS) {
            data.read_markers(chunk.aligned_payload(16));
        }
        Ok(data)
    }

    fn read_kinds(&mut self, table: &[u8]) -> Result<()> {
        let word = |at: usize| -> Option<u32> { Some(u32::from_le_bytes(table.get(at..at + 4)?.try_into().ok()?)) };
        let count = word(MOMENT_COUNT).context("candidate table too short")? as usize;
        for i in 0..count.min(MOMENT_LIMIT) {
            let at = MOMENT_LIST + 8 * i;
            let (Some(scene), Some(kind)) = (word(at), word(at + 4)) else { break };
            self.kinds.insert(scene, kind);
        }
        Ok(())
    }

    fn read_markers(&mut self, payload: &[u8]) {
        let wanted: BTreeMap<u32, &str> = MARKER_NAMES.iter().map(|name| (bstring_hash(name), *name)).collect();
        let float = |record: &[u8], at: usize| f32::from_le_bytes(record[at..at + 4].try_into().expect("four bytes"));
        for record in payload.chunks_exact(MARKER_LEN) {
            let name = u32::from_le_bytes(record[8..12].try_into().expect("four bytes"));
            if !wanted.contains_key(&name) {
                continue;
            }
            let position = Vec3::new(float(record, 0x10), float(record, 0x14), float(record, 0x18));
            let angle = u16::from_le_bytes([record[0x20], record[0x21]]);
            self.markers.entry(name).or_default().push(Marker { position, angle });
        }
    }

    /// The marker kind a scene plays at.
    pub fn kind_of(&self, scene: u32) -> Option<u32> {
        self.kinds.get(&scene).copied()
    }

    /// The marker the scene plays at nearest to `position` (render space), facing most like `heading` (a 16-bit
    /// angle). The score is the distance plus [`ANGLE_BONUS`] times the turn between the two angles as a share of a
    /// turn (at most half). Returns the marker and its score.
    pub fn closest_marker(&self, scene: u32, position: Vec3, heading: u16) -> Option<(Marker, f32)> {
        let name = MARKER_NAMES.get(self.kind_of(scene)? as usize)?;
        let markers = self.markers.get(&bstring_hash(name))?;
        markers
            .iter()
            .map(|m| (*m, m.position.distance(position) + turn(m.angle, heading) * ANGLE_BONUS))
            .min_by(|a, b| a.1.total_cmp(&b.1))
    }

    /// Camera track `index` of the scene (`Track 0`, `Track 1`...).
    pub fn track(&self, scene: u32, index: usize) -> Option<&Track> {
        let wanted = format!("Track {index}");
        self.cameras.get(&scene)?.iter().find(|t| t.name == wanted)
    }
}

/// How far apart two 16-bit angles are, as a share of a turn: 0 for the same, 0.5 for opposite.
fn turn(a: u16, b: u16) -> f32 {
    let d = a.wrapping_sub(b);
    f32::from(d.min(d.wrapping_neg())) / 65536.0
}

/// The moment data of the install, read once per session.
pub fn load(dir: &GameDir) -> Result<Arc<MomentData>> {
    let ingame = shared::ingame_b(dir)?;
    let l2ra = shared::l2ra(dir)?;
    memo(dir, &[shared::INGAME_B_FILE, shared::L2RA_FILE], || MomentData::read(&ingame, &l2ra))
}

#[cfg(test)]
#[path = "moment_tests.rs"]
mod tests;

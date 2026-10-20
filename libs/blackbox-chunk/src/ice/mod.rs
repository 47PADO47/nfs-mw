//! Readers for the ICE camera packs and shake tracks.
//!
//! Layouts are reverse engineered from the `speed.exe` 1.3 disassembly (Ghidra) and documented in
//! `docs/formats/animation.md` ("ICE chunk dispatch") and `docs/provenance/ice-camera-dispatch.md`.
//! The camera pack readers expose the raw 0x84-byte elements; [`Track`] and [`Key`] give the decoded key fields and
//! evaluate a track to a camera ([`CameraSample`], `docs/specs/pursuit-breaker-moment.md`).
//!
//! Not yet decoded: the 24-byte shake element. Its bytes are returned unparsed, so callers do not depend on a
//! guessed layout.

mod bytes;
mod camera;
mod error;
mod shake;
mod track;

#[cfg(test)]
mod tests;

pub use camera::{
    CAMERA_ELEMENT_LEN, CAMERA_RECORD_HEADER_LEN, CameraPack, CameraRecord, CameraSet, element_enabled, element_time,
    parse_camera_pack,
};
pub use error::{IceError, IceResult};
pub use shake::{SHAKE_ELEMENT_LEN, SHAKE_RECORD_HEADER_LEN, ShakeRecord, ShakeTracks, parse_shake_tracks};
pub use track::{CameraSample, Key, SENSOR_HALF_WIDTH, Space, Track, fov_of_lens};

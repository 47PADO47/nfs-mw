//! Readers for the ICE camera packs and shake tracks.
//!
//! Layouts are reverse engineered from the `speed.exe` 1.3 disassembly (Ghidra) and documented in
//! `docs/formats/animation.md` ("ICE chunk dispatch") and `docs/provenance/ice-camera-dispatch.md`.
//! Only the fields the engine's own loops read are exposed. Everything else stays raw bytes.
//!
//! Not yet decoded: the meaning of the 0x84-byte camera element beyond the flag byte and the time
//! key, and the 24-byte shake element. Their bytes are returned unparsed, so callers do not
//! depend on a guessed layout.

mod bytes;
mod camera;
mod error;
mod shake;

#[cfg(test)]
mod tests;

pub use camera::{
    CAMERA_ELEMENT_LEN, CAMERA_RECORD_HEADER_LEN, CameraPack, CameraRecord, CameraSet, element_enabled, element_time,
    parse_camera_pack,
};
pub use error::{IceError, IceResult};
pub use shake::{SHAKE_ELEMENT_LEN, SHAKE_RECORD_HEADER_LEN, ShakeRecord, ShakeTracks, parse_shake_tracks};

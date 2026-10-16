//! `Audio`: play one-shot sounds by bank and event name (catalogue: docs/modding/api-world.md#audio).

use super::ffi::audio::{self as host_audio, AudioFns};
use super::{ModResult, SoundId};

/// The `bb.audio` interface of the host, as a mod uses it. Obtained from `Ctx::audio`.
#[derive(Clone, Copy)]
pub struct Audio {
    fns: AudioFns,
}

impl Audio {
    pub(crate) fn new(fns: AudioFns) -> Self {
        Audio { fns }
    }

    /// Play event `event` of bank `bank` once, at `volume` (0 to 1). The host answers `Unsupported` until event names
    /// are looked up (see the catalogue).
    pub fn play(&self, bank: &str, event: &str, volume: f32) -> ModResult<SoundId> {
        host_audio::play(&self.fns, bank, event, volume).map(SoundId::from_raw)
    }
}

//! `bb.audio`: a one-shot sound by bank and event name. The game plays sounds by bank and index in its sound viewer and has no
//! lookup by event name yet, so every play answers `BB_ERR_UNSUPPORTED`. The table is offered so the mods' code is ready.

use blackbox_modding::host::{AudioBackend, BackendResult};

/// The `bb.audio` port of the game. It has no state.
pub struct AudioPort;

impl AudioBackend for AudioPort {
    fn play(&self, _owner: &str, _bank: &str, _event: &str, _volume: f32) -> BackendResult<u64> {
        Err(blackbox_modding::abi::BB_ERR_UNSUPPORTED)
    }
}

//! `nfsmw.races` on the host side: the backend trait and the builder. The race runtime is not built yet, so every method has
//! a default body that answers `BB_ERR_UNSUPPORTED`.

mod table;

use blackbox_modding::abi::BB_ERR_UNSUPPORTED;

use crate::abi::{BbRaceDef, BbRaceInfo, BbRaceResult, BbRaceState, BbVec3};

use super::BackendResult;

pub use table::{RacesTable, races_table};

/// What the game implements for `nfsmw.races`. `owner` is the id of the mod that made the call.
#[allow(unused_variables)] // The defaults ignore their arguments.
pub trait RacesBackend {
    /// The number of races.
    fn race_count(&self, owner: &str) -> BackendResult<u32> {
        Err(BB_ERR_UNSUPPORTED)
    }
    /// The race at `index`.
    fn race_at(&self, owner: &str, index: u32) -> BackendResult<BbRaceInfo> {
        Err(BB_ERR_UNSUPPORTED)
    }
    /// The number of checkpoints of race `id`.
    fn race_checkpoint_count(&self, owner: &str, id: u32) -> BackendResult<u32> {
        Err(BB_ERR_UNSUPPORTED)
    }
    /// Checkpoint `index` of race `id`.
    fn race_checkpoint_at(&self, owner: &str, id: u32, index: u32) -> BackendResult<BbVec3> {
        Err(BB_ERR_UNSUPPORTED)
    }
    /// Start race `id`.
    fn race_start(&self, owner: &str, id: u32) -> BackendResult<()> {
        Err(BB_ERR_UNSUPPORTED)
    }
    /// Abort the running race.
    fn race_abort(&self, owner: &str) -> BackendResult<()> {
        Err(BB_ERR_UNSUPPORTED)
    }
    /// The running race; `active` is zero when none runs.
    fn race_current(&self, owner: &str) -> BackendResult<BbRaceState> {
        Err(BB_ERR_UNSUPPORTED)
    }
    /// Register a custom race with its checkpoints; returns its id.
    fn race_register(&self, owner: &str, def: &BbRaceDef, checkpoints: &[BbVec3]) -> BackendResult<u32> {
        Err(BB_ERR_UNSUPPORTED)
    }
    /// Set the opponents' skill of race `id`, 0 to 1.
    fn race_set_opponent_skill(&self, owner: &str, id: u32, skill: f32) -> BackendResult<()> {
        Err(BB_ERR_UNSUPPORTED)
    }
    /// Set the cash for a win of race `id`.
    fn race_set_reward(&self, owner: &str, id: u32, cash: u32) -> BackendResult<()> {
        Err(BB_ERR_UNSUPPORTED)
    }
    /// The result of race `id`.
    fn race_results(&self, owner: &str, id: u32) -> BackendResult<BbRaceResult> {
        Err(BB_ERR_UNSUPPORTED)
    }
}

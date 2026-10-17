//! `nfsmw.pursuit` on the host side: the backend the game implements, and the table the builder makes of it.
//!
//! Every backend method is called on the main thread inside a mod callback (ADR 0005 section 4). `owner` is the id of the
//! mod that made the call. The backend's errors become the mod's status; use the `BB_ERR_*` codes of `blackbox-modding`.

mod check;
mod table;

use super::BackendResult;
use crate::abi::{BbCopRecord, BbHeatRow, CopTable, EndMode};

pub use table::{PursuitTable, pursuit_table};

/// One cop wave as the backend gives or takes it: the header and the records, `row.cop_count` of them.
#[derive(Debug, Clone, PartialEq)]
pub struct HeatRowSpec {
    /// The header. The host sets `cop_count` from `cops` before it sends the wave to a mod.
    pub row: BbHeatRow,
    /// The cop records of the wave, in order.
    pub cops: Vec<BbCopRecord>,
}

/// What the game implements for `nfsmw.pursuit`.
pub trait PursuitBackend {
    /// The heat, 1 to 10 with its fraction.
    fn heat_get(&self, owner: &str) -> BackendResult<f32>;
    /// Set the heat. The value is already checked to be finite; the game clamps it to the pursuit's base and maximum.
    fn heat_set(&self, owner: &str, heat: f32) -> BackendResult<()>;
    /// The cop wave of `table` at `level` (1 to 10), as the install defines it.
    fn cop_table_get(&self, owner: &str, table: CopTable, level: u32) -> BackendResult<HeatRowSpec>;
    /// Replace the cop wave of `table` at `level`. Already checked: the level, the finite and non-negative intervals, the
    /// names and the chances. It applies to the next wave.
    fn cop_table_set(&self, owner: &str, table: CopTable, level: u32, spec: &HeatRowSpec) -> BackendResult<()>;
    /// Start a pursuit at `level`. `near` is a vehicle handle the cops come from, or `None`.
    fn start(&self, owner: &str, level: u32, near: Option<u64>) -> BackendResult<()>;
    /// End the running pursuit.
    fn end(&self, owner: &str, mode: EndMode) -> BackendResult<()>;
    /// Whether a chase runs.
    fn is_pursuing(&self, owner: &str) -> BackendResult<bool>;
    /// The busted meter, 0 to 1.
    fn busted_get(&self, owner: &str) -> BackendResult<f32>;
    /// Cops in the running pursuit.
    fn cops_in_pursuit(&self, owner: &str) -> BackendResult<u32>;
}

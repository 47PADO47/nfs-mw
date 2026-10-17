//! The save screens' rules: the autosave gate and triggers, the file write plan, the load checks and what saving
//! changes (`docs/specs/career-save-screens.md`, `docs/specs/career-save-screens-2.md`).
//!
//! No I/O: the caller reads and writes files, shows the prompts and owns the profile database bytes. Where the
//! spec reconstructed the order of prompts from the library (it was not seen in game), the prompts are exposed as
//! an enum list ([`SavePanel`], [`ContinuePrompt`]) and no logic depends on their order.
//!
//! - [`autosave`]: the gate, the pending / armed bits, the triggers (roaming start, event start, pursuit evaded),
//!   the run and the failure handling. It replaces the earlier `safehouse::autosave` model.
//! - [`write`]: the three ordered chunks of the file and the rule that only a failed open is reported.
//! - [`load`]: [`check_load`], the load checks in the original order.
//! - [`state`]: the alias rules (31 characters, listed when shorter than 8 and without a space) and the bytes a
//!   save changes.

pub mod autosave;
pub mod load;
pub mod state;
pub mod write;

pub use autosave::{
    AutosaveManager, ContinueOutcome, ContinuePrompt, CurrentNode, GateInputs, RunDecision, ScreenAutosave, ScreenStep,
    Trigger, screen_state_b0,
};
pub use load::{BlobStatus, LoadError, LoadReport, check_load};
pub use state::{
    ProfileState, SavePanel, alias_dir_path, alias_file_path, is_listable_alias, list_aliases, manual_save_panels,
    truncate_alias,
};
pub use write::{Chunk, Part, WriteOutcome, WriteSteps, write_plan};

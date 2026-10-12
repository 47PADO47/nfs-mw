//! Save slots in one folder, and an alias index that gives slots friendly names.
//!
//! A slot is a file `<slot>.sav` whose name is checked (letters, digits, `-` and `_`, up to 64 characters), so a
//! slot can never reach outside the folder. A write goes to a temporary file that then replaces the slot, so a
//! crash mid-write leaves the old save whole. The alias index is `aliases.toml` in the same folder. The crate
//! reads and writes bytes only; what a save holds is the caller's business.

mod aliases;
pub mod digest;
mod store;

pub use aliases::{AliasError, Aliases};
pub use store::{SaveStore, StoreError, check_slot_name};

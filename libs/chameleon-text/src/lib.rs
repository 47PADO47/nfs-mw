//! Localised string tables of the Chameleon engine. Spec: `docs/formats/chameleon-language.md`.
//!
//! The `UI/LANGUAGE/NNNN.BNDL` bundles each hold one `LocalisedText` resource (type 0x201): a sorted list of numeric
//! string ids and their UTF-16 texts, all in the main block. This crate reads the block, edits it and writes it back
//! byte for byte; getting the block out of the bundle is `chameleon-bundle`'s job.
//!
//! This crate takes `&[u8]` and never opens files.

mod error;
mod table;

#[cfg(test)]
mod tests;

pub use error::{Error, Result};
pub use table::{StringEntry, StringTable};

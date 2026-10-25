//! `bnd2` resource bundles of the Chameleon engine. Spec: `docs/formats/chameleon-bundle.md`.
//!
//! A bundle is a table of typed resources. Each resource has up to four memory blocks (PC files use the first two:
//! main memory and graphics memory), stored as zlib streams, and an import table: pointers in the main block that the
//! loader patches with the addresses of other resources, by 64-bit id. Resources reference each other across bundles
//! (a car's materials name textures that live in a shared texture bundle), so a reader resolves ids over every bundle
//! it has opened.
//!
//! This crate takes `&[u8]` and never opens files.

mod bundle;
mod entry;
mod error;
mod header;
mod pool;
mod types;

#[cfg(test)]
mod tests;

pub use bundle::{Bundle, Resource};
pub use entry::{Entry, Import};
pub use error::{Error, Result};
pub use header::{BLOCKS, ENTRY_SIZE, FLAG_COMPRESSED, FLAG_HAS_DEBUG_DATA, HEADER_SIZE, Header};
pub use pool::Pool;
pub use types::ResourceKind;

//! FEng, the user-interface engine of EA Black Box games: package and font readers, the script and message
//! runtime, and a retained tree of drawable nodes. No rendering, no windowing.
//! Specs: `docs/formats/frontend.md`, `docs/specs/feng-runtime.md`.

mod error;
pub mod font;
mod hash;
pub mod ids;
pub mod package;
pub mod runtime;
pub mod tree;

pub use error::{Error, Result};
pub use font::Font;
pub use hash::{fe_hash_upper, resource_handle};
pub use package::Package;
pub use runtime::{ObjectRef, Outgoing, PackageCommandKind, PackageId, Runtime};
pub use tree::{NodeKind, UiNode, UiTree};

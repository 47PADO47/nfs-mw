//! EAGL4 animation of EA Black Box games: animation objects, skeletons, animation banks, channel codecs and
//! pose evaluation.
//!
//! Every reader takes the payload as `&[u8]` and never opens files; the caller supplies the bytes (for
//! example from the game install). Nothing here depends on a renderer or on a particular game.
//!
//! The crate is being built. The formats are in `docs/formats/animation.md` and the runtime behaviour in
//! `docs/specs/eagl-animation.md`.

mod error;

pub use error::{Error, Result};

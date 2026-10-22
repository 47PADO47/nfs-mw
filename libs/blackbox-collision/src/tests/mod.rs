//! Synthetic data built in memory (`pack`, `query`, `grid`, `bounds`), and an `#[ignore]` test
//! against a real install (`install`).

mod bounds;
mod face_cache;
mod filtered;
mod grid;
mod install;
mod install_filtered;
mod install_kept;
mod pack;
mod query;
pub(crate) mod synth;
mod ungridded;
mod window;

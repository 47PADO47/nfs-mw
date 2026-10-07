//! Locating, validating and reading a user's own install of a (Windows-era)
//! game, for engine reimplementations that load the original data at runtime.
//!
//! Game-agnostic: everything game-specific (environment variable, registry
//! keys, required files, known executables) comes from a [`GameSpec`].
//!
//! Lookup order of [`discover`] (first hit wins):
//! 1. an explicit path (typically a `--game-dir` command-line option);
//! 2. the spec's environment variable;
//! 3. `<ENV_VAR>=<path>` in a `.env` file in the working directory or next to the executable;
//! 4. `game_dir` in the per-user config file ([`config_file_path`]);
//! 5. on Windows, the spec's registry values.
//!
//! [`GameDir`] then gives case-insensitive, read-only access to the files, so
//! the same code works on Windows and on case-sensitive Linux filesystems.

mod discover;
mod spec;
mod validate;
mod vfs;

pub use discover::{DiscoverError, Discovered, Source, config_file_path, discover};
pub use spec::{GameSpec, KnownBuild, RegistryValue};
pub use validate::Validation;
pub use vfs::{GameDir, OpenError};

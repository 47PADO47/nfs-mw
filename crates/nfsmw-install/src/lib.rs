//! Locating, validating and reading the user's own NFS: Most Wanted install.
//!
//! This is the only crate that opens game files. Everything else works on byte
//! slices. See `docs/architecture.md` ("Finding the install").
//!
//! Lookup order (first hit wins):
//! 1. an explicit path (the `--game-dir` command-line option);
//! 2. the `NFSMW_GAME_DIR` environment variable;
//! 3. `NFSMW_GAME_DIR=` in a `.env` file in the working directory or next to the executable;
//! 4. `game_dir` in the per-user config file ([`config_file_path`]);
//! 5. on Windows, the registry key the retail installer writes.

mod discover;
mod vfs;

pub use discover::{DiscoverError, Discovered, ENV_VAR, Source, config_file_path, discover};
pub use vfs::{GameDir, OpenError};

/// SHA-256 of known `speed.exe` builds.
pub const KNOWN_EXECUTABLES: &[(&str, &str)] = &[(
    "80774c2e5d619b4f120b48d4462896fd504c263399d203a238769cffde1d253c",
    "PC v1.3 (retail patch 1.3 / Black Edition)",
)];

/// Files that must exist for the install to be usable (relative, case-insensitive).
pub const REQUIRED_FILES: &[&str] = &[
    "GLOBAL/GLOBALB.LZC",
    "GLOBAL/ATTRIBUTES.BIN",
    "TRACKS/L2RA.BUN",
    "TRACKS/STREAML2RA.BUN",
    "CARS/TEXTURES.BIN",
    "LANGUAGES/ENGLISH.BIN",
];

/// Result of [`GameDir::validate`].
#[derive(Debug, Clone)]
pub struct Validation {
    pub missing: Vec<&'static str>,
    /// SHA-256 of `speed.exe`, if present.
    pub exe_sha256: Option<String>,
    /// Name of the matching entry in [`KNOWN_EXECUTABLES`].
    pub exe_version: Option<&'static str>,
}

impl Validation {
    pub fn is_usable(&self) -> bool {
        self.missing.is_empty()
    }
}

//! What `game-install` needs to know about NFS: Most Wanted.

use std::path::Path;

use anyhow::{Context, Result, bail};
use game_install::{GameDir, GameSpec, KnownBuild, RegistryValue};

/// The installer's registry key: the install folder and the installed language are values under it.
pub const REGISTRY_KEY: &str = r"SOFTWARE\EA GAMES\Need for Speed Most Wanted";

pub const SPEC: GameSpec = GameSpec {
    name: "NFS: Most Wanted",
    env_var: "NFSMW_GAME_DIR",
    app_dir: "nfsmw",
    // Written by the retail installer.
    registry: &[RegistryValue { key: REGISTRY_KEY, value: "Install Dir" }],
    required_files: &[
        "GLOBAL/GLOBALB.LZC",
        "GLOBAL/ATTRIBUTES.BIN",
        "TRACKS/L2RA.BUN",
        "TRACKS/STREAML2RA.BUN",
        "CARS/TEXTURES.BIN",
        "LANGUAGES/ENGLISH.BIN",
    ],
    executable: Some("speed.exe"),
    known_builds: &[KnownBuild {
        sha256: "80774c2e5d619b4f120b48d4462896fd504c263399d203a238769cffde1d253c",
        label: "PC v1.3 (retail patch 1.3 / Black Edition)",
    }],
};

/// Find, index and check the install. Runs on every launch, so it does not hash
/// `speed.exe`; only `check-install` needs the build identity (see [`GameDir::validate`]).
pub fn open_install(explicit: Option<&Path>) -> Result<GameDir> {
    let found = game_install::discover(&SPEC, explicit)?;
    log::info!("install: {} (from {})", found.path.display(), found.source);
    let dir = GameDir::open(&found.path).with_context(|| format!("indexing {}", found.path.display()))?;
    let missing = dir.missing_files(&SPEC);
    if !missing.is_empty() {
        bail!(
            "{} does not look like an NFS: Most Wanted install; missing: {}",
            dir.root().display(),
            missing.join(", ")
        );
    }
    Ok(dir)
}

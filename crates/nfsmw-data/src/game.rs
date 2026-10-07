//! What `game-install` needs to know about NFS: Most Wanted.

use std::path::Path;

use anyhow::{Context, Result, bail};
use game_install::{GameDir, GameSpec, KnownBuild, RegistryValue};

pub const SPEC: GameSpec = GameSpec {
    name: "NFS: Most Wanted",
    env_var: "NFSMW_GAME_DIR",
    app_dir: "nfsmw",
    // Written by the retail installer.
    registry: &[RegistryValue { key: r"SOFTWARE\EA GAMES\Need for Speed Most Wanted", value: "Install Dir" }],
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

/// Find, index and validate the install.
pub fn open_install(explicit: Option<&Path>) -> Result<GameDir> {
    let found = game_install::discover(&SPEC, explicit)?;
    log::info!("install: {} (from {})", found.path.display(), found.source);
    let dir = GameDir::open(&found.path).with_context(|| format!("indexing {}", found.path.display()))?;
    let v = dir.validate(&SPEC);
    if !v.is_usable() {
        bail!(
            "{} does not look like an NFS: Most Wanted install; missing: {}",
            dir.root().display(),
            v.missing.join(", ")
        );
    }
    Ok(dir)
}

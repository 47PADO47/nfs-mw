//! What the game offers the mods, which mods may load, where they are looked for and how the runtime runs.

use std::path::{Path, PathBuf};

use blackbox_modding::host::{HostOffer, Policy, RunMode};

use crate::settings::mods::{ModsMode, ModsSettings};

/// The game id a mod names in `[games] supported`.
pub const GAME_ID: &str = "nfsmw";

/// The folder under the config directory that holds the mods (ADR 0005 section 3).
pub const MODS_FOLDER: &str = "mods";

/// The host's offer. The interfaces are added when the runtime starts (`ModRuntime::start_with`), so a mod that requires
/// one of them loads only when the game offers it.
pub fn offer() -> HostOffer {
    HostOffer::new(GAME_ID, env!("CARGO_PKG_VERSION"))
}

/// The policy the settings ask for: `off` loads nothing, `allowlist` the listed ids, `all` every mod except the ones
/// switched off in the mod manager (`disabled`). The switched-off list is not used in `allowlist` mode: there the
/// manager edits the allowlist itself.
pub fn policy(settings: &ModsSettings) -> Policy {
    match settings.mode {
        ModsMode::Off => Policy::Off,
        ModsMode::All if settings.disabled.is_empty() => Policy::All,
        ModsMode::All => Policy::Except(settings.disabled.iter().cloned().collect()),
        ModsMode::Allowlist => Policy::Allowlist(settings.allowlist.iter().cloned().collect()),
    }
}

/// The folders to scan: `mods` next to the config file, then the `--mods-dir` folder when one is given.
pub fn roots(config_file: Option<&Path>, extra: Option<&Path>) -> Vec<PathBuf> {
    let mut roots: Vec<PathBuf> =
        config_file.and_then(Path::parent).map(|dir| dir.join(MODS_FOLDER)).into_iter().collect();
    roots.extend(extra.map(Path::to_path_buf));
    roots
}

/// A normal run, or with `--mod-dev` a developer run that loads copies of the libraries.
pub fn run_mode(dev: bool) -> RunMode {
    if !dev {
        return RunMode::Normal;
    }
    RunMode::Dev { cache_dir: dev_cache_dir() }
}

/// The folder of the library copies of a developer run: next to the log folder, in the local data folder. The temp
/// folder when the per-user folders are unknown.
fn dev_cache_dir() -> PathBuf {
    game_install::log_dir(&nfsmw_data::game::SPEC)
        .as_deref()
        .and_then(Path::parent)
        .map_or_else(std::env::temp_dir, Path::to_path_buf)
        .join("mod-cache")
}

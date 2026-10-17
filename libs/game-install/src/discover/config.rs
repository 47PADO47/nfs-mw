//! `game_dir` in the per-user config file.

use std::path::{Path, PathBuf};

use super::{Discovered, Source};
use crate::GameSpec;

/// Per-user config file: `<config dir>/<app_dir>/config.toml`
/// (`%APPDATA%\<app_dir>\config\config.toml` on Windows, `~/.config/<app_dir>/config.toml` on Linux).
/// `BLACKBOX_PROFILE_DIR` overrides this with `<root>/<app_dir>/config/config.toml`.
pub fn config_file_path(spec: &GameSpec) -> Option<PathBuf> {
    if let Some(root) = std::env::var_os("BLACKBOX_PROFILE_DIR") {
        return profile_dir(Path::new(&root), spec.app_dir, "config").map(|d| d.join("config.toml"));
    }
    directories::ProjectDirs::from("", "", spec.app_dir).map(|d| d.config_dir().join("config.toml"))
}

/// Per-user log folder: `<local data dir>/<app_dir>/logs` (`%LOCALAPPDATA%\<app_dir>\data\logs` on Windows,
/// `~/.local/share/<app_dir>/logs` on Linux). The folder may not exist yet.
/// `BLACKBOX_PROFILE_DIR` overrides this with `<root>/<app_dir>/logs`.
pub fn log_dir(spec: &GameSpec) -> Option<PathBuf> {
    if let Some(root) = std::env::var_os("BLACKBOX_PROFILE_DIR") {
        return profile_dir(Path::new(&root), spec.app_dir, "logs");
    }
    directories::ProjectDirs::from("", "", spec.app_dir).map(|d| d.data_local_dir().join("logs"))
}

/// Reject a relative override instead of silently writing into the normal profile.
fn profile_dir(root: &Path, app_dir: &str, subdir: &str) -> Option<PathBuf> {
    root.is_absolute().then(|| root.join(app_dir).join(subdir))
}

#[derive(serde::Deserialize, Default)]
struct ConfigFile {
    game_dir: Option<PathBuf>,
}

pub(super) fn lookup(spec: &GameSpec) -> Option<Discovered> {
    let file = config_file_path(spec)?;
    let text = std::fs::read_to_string(&file).ok()?;
    match toml::from_str::<ConfigFile>(&text) {
        Ok(cfg) => cfg.game_dir.map(|path| Discovered { path, source: Source::ConfigFile(file) }),
        Err(e) => {
            log::warn!("ignoring {}: {e}", file.display());
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn isolated_profile_paths_share_the_explicit_root() {
        let root = std::env::temp_dir().join("isolated-profile");
        let config = profile_dir(&root, "example-game", "config").unwrap().join("config.toml");
        let logs = profile_dir(&root, "example-game", "logs").unwrap();
        assert_eq!(config, root.join("example-game/config/config.toml"));
        assert_eq!(logs, root.join("example-game/logs"));
        assert!(config.starts_with(&root));
        assert!(logs.starts_with(&root));
    }

    #[test]
    fn relative_or_empty_profile_overrides_do_not_resolve() {
        for root in [Path::new("relative-profile"), Path::new("")] {
            assert!(profile_dir(root, "example-game", "config").is_none());
            assert!(profile_dir(root, "example-game", "logs").is_none());
        }
    }
}

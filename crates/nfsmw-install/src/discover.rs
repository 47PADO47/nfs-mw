//! Where the install is. See the crate docs for the lookup order.

use std::path::{Path, PathBuf};

/// Environment variable (and `.env` key) naming the install directory.
pub const ENV_VAR: &str = "NFSMW_GAME_DIR";

/// Where a game directory came from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Source {
    CommandLine,
    Environment,
    DotEnv(PathBuf),
    ConfigFile(PathBuf),
    Registry(String),
}

impl std::fmt::Display for Source {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::CommandLine => write!(f, "--game-dir"),
            Self::Environment => write!(f, "${ENV_VAR}"),
            Self::DotEnv(p) => write!(f, "{}", p.display()),
            Self::ConfigFile(p) => write!(f, "{}", p.display()),
            Self::Registry(k) => write!(f, "registry {k}"),
        }
    }
}

#[derive(Debug, Clone)]
pub struct Discovered {
    pub path: PathBuf,
    pub source: Source,
}

#[derive(Debug, thiserror::Error)]
pub enum DiscoverError {
    #[error(
        "could not find the NFS: Most Wanted install. Pass --game-dir <PATH>, set {ENV_VAR}, \
         add `{ENV_VAR}=<PATH>` to a .env file, or put `game_dir = \"<PATH>\"` in {config}"
    )]
    NotFound { config: String },
    #[error("{origin}: {path} is not a directory")]
    NotADirectory { path: PathBuf, origin: Source },
}

/// Per-user config file: `<config dir>/nfsmw/config.toml`
/// (`%APPDATA%\nfsmw\config\config.toml` on Windows, `~/.config/nfsmw/config.toml` on Linux).
pub fn config_file_path() -> Option<PathBuf> {
    directories::ProjectDirs::from("", "", "nfsmw").map(|d| d.config_dir().join("config.toml"))
}

#[derive(serde::Deserialize, Default)]
struct ConfigFile {
    game_dir: Option<PathBuf>,
}

fn from_dotenv(dir: &Path) -> Option<Discovered> {
    let file = dir.join(".env");
    let text = std::fs::read_to_string(&file).ok()?;
    text.lines().find_map(|line| {
        let line = line.trim();
        let (k, v) = line.strip_prefix("export ").unwrap_or(line).split_once('=')?;
        (k.trim() == ENV_VAR).then(|| Discovered {
            path: PathBuf::from(v.trim().trim_matches('"').trim_matches('\'')),
            source: Source::DotEnv(file.clone()),
        })
    })
}

fn from_config() -> Option<Discovered> {
    let file = config_file_path()?;
    let text = std::fs::read_to_string(&file).ok()?;
    match toml::from_str::<ConfigFile>(&text) {
        Ok(cfg) => cfg.game_dir.map(|path| Discovered { path, source: Source::ConfigFile(file) }),
        Err(e) => {
            log::warn!("ignoring {}: {e}", file.display());
            None
        }
    }
}

#[cfg(windows)]
fn from_registry() -> Option<Discovered> {
    use winreg::RegKey;
    use winreg::enums::{HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE, KEY_READ, KEY_WOW64_32KEY};
    // Written by the retail installer (32-bit view on 64-bit Windows).
    const KEY: &str = r"SOFTWARE\EA GAMES\Need for Speed Most Wanted";
    for (hive, hive_name) in [(HKEY_LOCAL_MACHINE, "HKLM"), (HKEY_CURRENT_USER, "HKCU")] {
        let Ok(key) = RegKey::predef(hive).open_subkey_with_flags(KEY, KEY_READ | KEY_WOW64_32KEY) else {
            continue;
        };
        if let Ok(dir) = key.get_value::<String, _>("Install Dir") {
            return Some(Discovered {
                path: PathBuf::from(dir),
                source: Source::Registry(format!(r"{hive_name}\{KEY}\Install Dir")),
            });
        }
    }
    None
}

#[cfg(not(windows))]
fn from_registry() -> Option<Discovered> {
    None
}

/// Find the install directory. `explicit` is the `--game-dir` option, if given.
pub fn discover(explicit: Option<&Path>) -> Result<Discovered, DiscoverError> {
    let exe_dir = std::env::current_exe().ok().and_then(|p| p.parent().map(Path::to_path_buf));
    let cwd = std::env::current_dir().ok();

    let found = explicit
        .map(|p| Discovered { path: p.to_path_buf(), source: Source::CommandLine })
        .or_else(|| {
            std::env::var_os(ENV_VAR)
                .filter(|v| !v.is_empty())
                .map(|v| Discovered { path: PathBuf::from(v), source: Source::Environment })
        })
        .or_else(|| cwd.as_deref().and_then(from_dotenv))
        .or_else(|| exe_dir.as_deref().and_then(from_dotenv))
        .or_else(from_config)
        .or_else(from_registry);

    match found {
        Some(d) if d.path.is_dir() => Ok(d),
        Some(d) => Err(DiscoverError::NotADirectory { path: d.path, origin: d.source }),
        None => Err(DiscoverError::NotFound {
            config: config_file_path().map_or_else(|| "the config file".into(), |p| p.display().to_string()),
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dotenv_parsing() {
        let dir = std::env::temp_dir().join(format!("nfsmw-dotenv-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join(".env"), "# comment\nOTHER=1\nexport NFSMW_GAME_DIR=\"D:/Games/NFSMW\"\n").unwrap();
        let d = from_dotenv(&dir).unwrap();
        assert_eq!(d.path, PathBuf::from("D:/Games/NFSMW"));
        assert!(matches!(d.source, Source::DotEnv(_)));
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn explicit_path_must_be_a_directory() {
        let err = discover(Some(Path::new("/definitely/not/here"))).unwrap_err();
        assert!(matches!(err, DiscoverError::NotADirectory { origin: Source::CommandLine, .. }));
    }
}

//! Where the install is. See the crate docs for the lookup order.

mod config;
mod dotenv;
mod registry;
mod source;

use std::path::{Path, PathBuf};

pub use config::config_file_path;
pub use source::{Discovered, Source};

use crate::GameSpec;

#[derive(Debug, thiserror::Error)]
pub enum DiscoverError {
    #[error(
        "could not find the {game} install. Pass --game-dir <PATH>, set {env_var}, \
         add `{env_var}=<PATH>` to a .env file, or put `game_dir = \"<PATH>\"` in {config}"
    )]
    NotFound { game: &'static str, env_var: &'static str, config: String },
    #[error("{origin}: {path} is not a directory")]
    NotADirectory { path: PathBuf, origin: Source },
}

/// Find the install directory. `explicit` is a path given on the command line, if any.
pub fn discover(spec: &GameSpec, explicit: Option<&Path>) -> Result<Discovered, DiscoverError> {
    let exe_dir = std::env::current_exe().ok().and_then(|p| p.parent().map(Path::to_path_buf));
    let cwd = std::env::current_dir().ok();

    let found = explicit
        .map(|p| Discovered { path: p.to_path_buf(), source: Source::CommandLine })
        .or_else(|| {
            std::env::var_os(spec.env_var)
                .filter(|v| !v.is_empty())
                .map(|v| Discovered { path: PathBuf::from(v), source: Source::Environment(spec.env_var) })
        })
        .or_else(|| cwd.as_deref().and_then(|d| dotenv::lookup(d, spec.env_var)))
        .or_else(|| exe_dir.as_deref().and_then(|d| dotenv::lookup(d, spec.env_var)))
        .or_else(|| config::lookup(spec))
        .or_else(|| registry::lookup(spec));

    match found {
        Some(d) if d.path.is_dir() => Ok(d),
        Some(d) => Err(DiscoverError::NotADirectory { path: d.path, origin: d.source }),
        None => Err(DiscoverError::NotFound {
            game: spec.name,
            env_var: spec.env_var,
            config: config_file_path(spec).map_or_else(|| "the config file".into(), |p| p.display().to_string()),
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SPEC: GameSpec = GameSpec {
        name: "Test Game",
        env_var: "GAME_INSTALL_TEST_DIR_UNSET",
        app_dir: "game-install-test",
        registry: &[],
        required_files: &[],
        executable: None,
        known_builds: &[],
    };

    #[test]
    fn explicit_path_must_be_a_directory() {
        let err = discover(&SPEC, Some(Path::new("/definitely/not/here"))).unwrap_err();
        assert!(matches!(err, DiscoverError::NotADirectory { origin: Source::CommandLine, .. }));
    }

    #[test]
    fn explicit_path_wins() {
        let dir = std::env::temp_dir();
        let found = discover(&SPEC, Some(&dir)).unwrap();
        assert_eq!(found.source, Source::CommandLine);
    }
}

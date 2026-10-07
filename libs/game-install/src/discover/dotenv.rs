//! `<ENV_VAR>=<path>` in a `.env` file.

use std::path::{Path, PathBuf};

use super::{Discovered, Source};

pub(super) fn lookup(dir: &Path, env_var: &str) -> Option<Discovered> {
    let file = dir.join(".env");
    let text = std::fs::read_to_string(&file).ok()?;
    text.lines().find_map(|line| {
        let line = line.trim();
        let (k, v) = line.strip_prefix("export ").unwrap_or(line).split_once('=')?;
        (k.trim() == env_var).then(|| Discovered {
            path: PathBuf::from(v.trim().trim_matches('"').trim_matches('\'')),
            source: Source::DotEnv(file.clone()),
        })
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_quoted_and_exported_values() {
        let dir = std::env::temp_dir().join(format!("game-install-dotenv-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join(".env"), "# comment\nOTHER=1\nexport MY_GAME_DIR=\"D:/Games/My Game\"\n").unwrap();
        let d = lookup(&dir, "MY_GAME_DIR").unwrap();
        assert_eq!(d.path, PathBuf::from("D:/Games/My Game"));
        assert!(matches!(d.source, Source::DotEnv(_)));
        assert!(lookup(&dir, "MISSING").is_none());
        std::fs::remove_dir_all(dir).unwrap();
    }
}

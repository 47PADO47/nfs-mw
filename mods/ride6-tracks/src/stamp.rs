//! A fingerprint of everything an import's output depends on, so a start with nothing changed does not convert again.

use std::fs;
use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;

use crate::config::Config;

/// Bump when the importer changes what it writes for the same input.
const CONVERSION: u32 = 4;
/// The name of the stamp file, in the mod's folder.
pub const FILE: &str = ".import-stamp";

/// The track packs inside `source`: the folders that hold a `pack.json`, in name order.
pub fn discover(source: &Path) -> Vec<PathBuf> {
    let mut found: Vec<PathBuf> = fs::read_dir(source)
        .into_iter()
        .flatten()
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| path.join("pack.json").is_file())
        .collect();
    found.sort();
    found
}

/// The fingerprint for these settings and packs: the settings, the converter's version, and the name, size and modification
/// time of each pack's files.
pub fn of(config: &Config, packs: &[PathBuf]) -> String {
    let mut parts = vec![format!("conversion {CONVERSION}"), format!("{config:?}")];
    for pack in packs {
        for name in ["pack.json", "ground.r6m"] {
            parts.push(fingerprint(&pack.join(name), pack.file_name()));
        }
        let mut pictures: Vec<String> = fs::read_dir(pack.join("tex"))
            .into_iter()
            .flatten()
            .flatten()
            .map(|entry| fingerprint(&entry.path(), pack.file_name()))
            .collect();
        pictures.sort();
        parts.extend(pictures);
    }
    parts.join("\n")
}

fn fingerprint(path: &Path, owner: Option<&std::ffi::OsStr>) -> String {
    let name = path.file_name().map(|name| name.to_string_lossy().to_ascii_lowercase()).unwrap_or_default();
    let owner = owner.map(|owner| owner.to_string_lossy().to_ascii_lowercase()).unwrap_or_default();
    let Ok(meta) = fs::metadata(path) else { return format!("{owner}/{name} missing") };
    let modified =
        meta.modified().ok().and_then(|time| time.duration_since(UNIX_EPOCH).ok()).map_or(0, |d| d.as_secs());
    format!("{owner}/{name} {} {modified}", meta.len())
}

/// Whether the stamp left by the last complete run is `stamp`.
pub fn is_current(mod_dir: &Path, stamp: &str) -> bool {
    fs::read_to_string(mod_dir.join(FILE)).is_ok_and(|saved| saved == stamp)
}

pub fn clear(mod_dir: &Path) {
    let _ = fs::remove_file(mod_dir.join(FILE));
}

pub fn save(mod_dir: &Path, stamp: &str) -> std::io::Result<()> {
    fs::write(mod_dir.join(FILE), stamp)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("ride6-tracks-stamp-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn a_stamp_is_current_only_when_it_matches() {
        let dir = temp("current");
        assert!(!is_current(&dir, "a"));
        save(&dir, "a").unwrap();
        assert!(is_current(&dir, "a"));
        assert!(!is_current(&dir, "b"));
        clear(&dir);
        assert!(!is_current(&dir, "a"));
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn packs_are_the_folders_with_a_pack_json_and_inputs_change_the_stamp() {
        let dir = temp("inputs");
        fs::create_dir_all(dir.join("okayama")).unwrap();
        fs::write(dir.join("okayama/pack.json"), "{}").unwrap();
        fs::create_dir_all(dir.join("notapack")).unwrap();
        let packs = discover(&dir);
        assert_eq!(packs, vec![dir.join("okayama")]);
        let base = of(&Config::default(), &packs);
        assert_eq!(base, of(&Config::default(), &packs), "the same input gives the same stamp");
        let smaller = Config { max_texture: 256, ..Config::default() };
        assert_ne!(base, of(&smaller, &packs));
        fs::write(dir.join("okayama/ground.r6m"), "x").unwrap();
        assert_ne!(base, of(&Config::default(), &packs));
        fs::remove_dir_all(&dir).unwrap();
    }
}

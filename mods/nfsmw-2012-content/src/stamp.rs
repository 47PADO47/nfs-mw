//! A fingerprint of everything a run's output depends on, so a start with nothing changed does not convert again.

use std::fs;
use std::path::Path;
use std::time::UNIX_EPOCH;

use crate::config::Config;

/// Bump when the conversion changes what it writes for the same input.
const CONVERSION: u32 = 8;
pub const FILE: &str = ".import-stamp";

/// The fingerprint for these settings and the install: the settings, the converter's version, and the names, sizes and
/// modification times of the source files.
pub fn of(config: &Config, source: &Path) -> String {
    let mut parts = vec![format!("conversion {CONVERSION}"), format!("{config:?}")];
    let mut sources =
        vec![fingerprint(&source.join("VEHICLES/VEHICLETEX.BNDL")), fingerprint(&source.join("SHADERS.BNDL"))];
    for entry in fs::read_dir(source.join("GAMELOGIC")).into_iter().flatten().flatten() {
        sources.push(fingerprint(&entry.path()));
    }
    for entry in fs::read_dir(source.join("VEHICLES")).into_iter().flatten().flatten() {
        let name = entry.file_name().to_string_lossy().to_ascii_uppercase();
        if name.starts_with("VEH_") && name.ends_with("_HI.BNDL") {
            sources.push(fingerprint(&entry.path()));
        }
    }
    sources.sort();
    parts.extend(sources);
    parts.join("\n")
}

fn fingerprint(path: &Path) -> String {
    let name = path.file_name().map(|name| name.to_string_lossy().to_ascii_uppercase()).unwrap_or_default();
    let Ok(meta) = fs::metadata(path) else { return format!("{name} missing") };
    let modified =
        meta.modified().ok().and_then(|time| time.duration_since(UNIX_EPOCH).ok()).map_or(0, |d| d.as_secs());
    format!("{name} {} {modified}", meta.len())
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

    #[test]
    fn a_stamp_is_current_only_when_it_matches() {
        let dir = std::env::temp_dir().join(format!("nfsmw-import2012-stamp-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        assert!(!is_current(&dir, "a"));
        save(&dir, "a").unwrap();
        assert!(is_current(&dir, "a"));
        assert!(!is_current(&dir, "b"));
        clear(&dir);
        assert!(!is_current(&dir, "a"));
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn settings_change_the_stamp() {
        let nowhere = Path::new("definitely/not/here");
        let a = of(&Config::default(), nowhere);
        let b = of(&Config { max_texture_size: 512, ..Config::default() }, nowhere);
        assert_ne!(a, b);
    }
}

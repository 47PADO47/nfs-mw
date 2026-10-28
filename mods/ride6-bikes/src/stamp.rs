//! A fingerprint of everything an import's output depends on, so a start with nothing changed does not convert again.

use std::fs;
use std::path::Path;
use std::time::UNIX_EPOCH;

use crate::config::Config;

/// Bump when the importer changes what it writes for the same input.
const CONVERSION: u32 = 1;
/// The name of the stamp file, in the mod's folder.
pub const FILE: &str = ".import-stamp";

/// The fingerprint for these settings and inputs: the settings, the converter's version, and the names, sizes and
/// modification times of each bike's `parts.json` (and whether its meshes are there), of the physics files and of the
/// sound tables.
pub fn of(config: &Config, bikes: &[std::path::PathBuf], physics: &Path, sound: Option<&Path>) -> String {
    let mut parts = vec![format!("conversion {CONVERSION}"), format!("{config:?}")];
    let mut inputs: Vec<String> = bikes
        .iter()
        .map(|bike| {
            let meshes = if bike.join("glb").is_dir() { "meshes" } else { "no meshes" };
            format!("{} {meshes}", fingerprint(&bike.join("parts.json"), bike.file_name()))
        })
        .collect();
    inputs.extend(files_below(&physics.join("vehicles")));
    if let Some(dir) = sound {
        inputs.extend(files_below(&dir.join("cars")));
    }
    inputs.sort();
    parts.extend(inputs);
    parts.join("\n")
}

/// The fingerprints of the files one and two levels below `dir`.
fn files_below(dir: &Path) -> Vec<String> {
    let mut found = Vec::new();
    for entry in fs::read_dir(dir).into_iter().flatten().flatten() {
        let path = entry.path();
        if !path.is_dir() {
            found.push(fingerprint(&path, None));
            continue;
        }
        for inner in fs::read_dir(&path).into_iter().flatten().flatten() {
            found.push(fingerprint(&inner.path(), path.file_name()));
        }
    }
    found
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

    fn temp(name: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!("ride6-bikes-stamp-{name}-{}", std::process::id()));
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
    fn settings_and_inputs_change_the_stamp() {
        let dir = temp("inputs");
        let bike = dir.join("ktm450");
        fs::create_dir_all(&bike).unwrap();
        fs::write(bike.join("parts.json"), "{}").unwrap();
        let bikes = vec![bike.clone()];
        let nowhere = dir.join("none");
        let base = of(&Config::default(), &bikes, &nowhere, None);
        assert_eq!(base, of(&Config::default(), &bikes, &nowhere, None), "the same input gives the same stamp");
        let smaller = Config { max_texture: 512, ..Config::default() };
        assert_ne!(base, of(&smaller, &bikes, &nowhere, None));
        // The meshes being there counts: a cleaned folder is another input.
        fs::create_dir_all(bike.join("glb")).unwrap();
        assert_ne!(base, of(&Config::default(), &bikes, &nowhere, None));
        // So does a physics file.
        let with_glb = of(&Config::default(), &bikes, &nowhere, None);
        fs::create_dir_all(dir.join("dump/vehicles/KTM")).unwrap();
        fs::write(dir.join("dump/vehicles/KTM/a.json"), "{}").unwrap();
        assert_ne!(with_glb, of(&Config::default(), &bikes, &dir.join("dump"), None));
        fs::remove_dir_all(&dir).unwrap();
    }
}

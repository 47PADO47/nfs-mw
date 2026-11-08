//! A fingerprint of everything an import's output depends on, so a start with nothing changed does not convert again.

use std::fs;
use std::path::Path;
use std::time::UNIX_EPOCH;

use crate::config::Config;

/// Bump when the importer changes what it writes for the same input.
/// 2: the rider's contact points per car (`RIDER/contacts.json`).
/// 4: the shop's options: the stock exhaust, rims, discs and bar pad as solids of their own, `components/`, `PARTS/R6/`.
/// 3: glass blended as its transmittance, metals spared most of the baked occlusion, more stock parts, liveries, the finest level
/// of detail whose solids fit, colours not veiled by the reflection, the idle panel in the instrument clusters' displays.
/// 5: the rider's gear catalogue (`files/RIDER/GEAR`).
/// 6: the clusters' screens composed from RIDE 6's gauge widgets (`tex/*_dash.png`), the livery paint's occlusion floor, the hue-keeping
/// lift and satin metal drawn as painted metal.
/// 7: the finest level of detail for every body (the frame is as many solids as it needs, `BODY2`, ...), the pictures of the old rims,
/// silencers and headers whose master names them differently, the far levels' paint taken from the chassis' livery.
/// 8: the road equipment (side mirrors, homologation kit) as solids of their own with an optional slot, group `road`
/// (`docs/modding/ride6-road-equipment.md`).
const CONVERSION: u32 = 8;
/// The name of the stamp file, in the mod's folder.
pub const FILE: &str = ".import-stamp";

/// The fingerprint for these settings and inputs: the settings, the converter's version, and the names, sizes and
/// modification times of each bike's `parts.json` (and whether its meshes are there), of the physics files and of the
/// sound tables.
pub fn of(
    config: &Config,
    bikes: &[std::path::PathBuf],
    physics: &Path,
    sound: Option<&Path>,
    rider: Option<&Path>,
    gear: Option<&Path>,
) -> String {
    let mut parts = vec![format!("conversion {CONVERSION}"), format!("{config:?}")];
    let mut inputs: Vec<String> = bikes
        .iter()
        .map(|bike| {
            let meshes = if bike.join("glb").is_dir() { "meshes" } else { "no meshes" };
            format!("{} {meshes}", fingerprint(&bike.join("parts.json"), bike.file_name()))
        })
        .collect();
    inputs.extend(files_below(&physics.join("vehicles")));
    inputs
        .extend(files_below(&physics.join("resolved").join("content").join("assets").join("data").join("datatables")));
    if let Some(dir) = sound {
        inputs.extend(files_below(&dir.join("cars")));
    }
    if let Some(dir) = rider {
        inputs.extend(files_below(dir));
    }
    if let Some(dir) = gear {
        inputs.extend(files_below(dir));
    }
    inputs.sort();
    parts.extend(inputs);
    parts.join("\n")
}

/// `stamp` with the shop's inputs: the catalogue and the shared part table of the folder of the optional parts, when there is one.
pub fn with_parts(stamp: String, parts: Option<&Path>) -> String {
    let Some(dir) = parts else { return stamp };
    let files = ["catalogue.json", "parts.json"].map(|name| fingerprint(&dir.join(name), dir.file_name()));
    format!("{stamp}\n{}", files.join("\n"))
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
    fn the_optional_parts_change_the_stamp() {
        let dir = temp("parts");
        assert_eq!(with_parts("s".to_owned(), None), "s");
        let without = with_parts("s".to_owned(), Some(&dir));
        fs::write(dir.join("catalogue.json"), "{}").unwrap();
        let with = with_parts("s".to_owned(), Some(&dir));
        assert_ne!(without, with);
        assert_eq!(with, with_parts("s".to_owned(), Some(&dir)));
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
        let base = of(&Config::default(), &bikes, &nowhere, None, None, None);
        assert_eq!(
            base,
            of(&Config::default(), &bikes, &nowhere, None, None, None),
            "the same input gives the same stamp"
        );
        let smaller = Config { max_texture: 512, ..Config::default() };
        assert_ne!(base, of(&smaller, &bikes, &nowhere, None, None, None));
        // The meshes being there counts: a cleaned folder is another input.
        fs::create_dir_all(bike.join("glb")).unwrap();
        assert_ne!(base, of(&Config::default(), &bikes, &nowhere, None, None, None));
        // So does a physics file.
        let with_glb = of(&Config::default(), &bikes, &nowhere, None, None, None);
        fs::create_dir_all(dir.join("dump/vehicles/KTM")).unwrap();
        fs::write(dir.join("dump/vehicles/KTM/a.json"), "{}").unwrap();
        assert_ne!(with_glb, of(&Config::default(), &bikes, &dir.join("dump"), None, None, None));
        fs::remove_dir_all(&dir).unwrap();
    }
}

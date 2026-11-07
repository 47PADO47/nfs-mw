//! The files the mod writes into its folder, all of them small text: the world's manifest, the manifest of each unit (a map pack
//! that the file places, `docs/modding/api-content.md`) and the stub of each unit's lazy file. Nothing of the city is on disk.

use std::fmt::Write as _;
use std::path::Path;

use anyhow::{Context, Result};

use crate::pvs::Zone;

/// The world's id: `--world fairhaven`.
pub const WORLD_ID: &str = "fairhaven";
/// The id of the mod, which is also the owner a stub names.
pub const OWNER: &str = "fairhaven";
/// The folder of the mod that holds the world manifests (`blackbox_modding::content::WORLDS_FOLDER`).
pub const WORLDS_FOLDER: &str = "worlds";
/// The folder of the unit files under the mod's `files/`.
pub const FILES_FOLDER: &str = "WORLDS/FAIRHAVEN";
/// What a unit's anchor covers beyond the unit's own rectangle is the game's 150 m; the anchor circle is the rectangle's, scaled.
pub const ANCHOR_SCALE: f32 = 1.0;

/// The section number of the unit at `index` in the zone list order of the units sorted by number: the free band 2700 and up,
/// 40 numbers (last two digits 0 to 39) in each hundred (`docs/specs/mod-world-content.md`, 1.3).
pub fn section_number(index: usize) -> u32 {
    2700 + (index / 40) as u32 * 100 + (index % 40) as u32
}

/// The units sorted by number, each with its section number.
pub fn numbered(zones: &[Zone]) -> Vec<(Zone, u32)> {
    let mut sorted = zones.to_vec();
    sorted.sort_by_key(|zone| zone.unit);
    sorted.into_iter().enumerate().map(|(index, zone)| (zone, section_number(index))).collect()
}

/// The manifest key of a unit: its file name without `.toml`.
pub fn unit_key(unit: u32) -> String {
    format!("u{unit}")
}

/// The unit's file under the mod's `files/`.
pub fn unit_file(unit: u32) -> String {
    format!("{FILES_FOLDER}/U{unit}.BUN")
}

/// The key a unit's stub holds, and the unit a key names.
pub fn lazy_key(unit: u32) -> String {
    format!("unit/{unit}")
}

pub fn unit_of_key(key: &str) -> Option<u32> {
    key.strip_prefix("unit/")?.parse().ok()
}

/// The text of a unit's manifest: it places its own objects, and loads while the camera is within the game's margin of the
/// circle that holds the zone's rectangle (render space).
pub fn unit_manifest(zone: &Zone, section: u32) -> String {
    let [x, y] = zone.centre_render();
    let mut text = String::new();
    let _ = writeln!(text, "[pack]");
    let _ = writeln!(text, "file = \"{}\"", unit_file(zone.unit));
    let _ = writeln!(text, "section = {section}");
    let _ = writeln!(text, "anchor = [{x:.1}, {y:.1}, 0.0, {:.1}]", zone.radius() * ANCHOR_SCALE);
    let _ = writeln!(text, "placed = \"file\"");
    text
}

/// The text of the world's manifest: its name and where the player starts (render space, heading in degrees from +x).
pub fn world_manifest(name: &str, start: [f32; 3], heading: f32) -> String {
    format!(
        "[world]\nname = \"{name}\"\nstart = {{ pos = [{:.2}, {:.2}, {:.2}], heading = {heading:.1} }}\n",
        start[0], start[1], start[2]
    )
}

/// Writes `bytes` to `path` unless the file already holds them; makes the folder.
fn write_if_changed(path: &Path, bytes: &[u8]) -> Result<bool> {
    if std::fs::read(path).is_ok_and(|current| current == bytes) {
        return Ok(false);
    }
    if let Some(folder) = path.parent() {
        std::fs::create_dir_all(folder).with_context(|| format!("creating {}", folder.display()))?;
    }
    std::fs::write(path, bytes).with_context(|| format!("writing {}", path.display()))?;
    Ok(true)
}

/// What [`write`] did.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Written {
    /// Files that were new or changed; 0 on a start after the first.
    pub changed: usize,
    pub units: usize,
}

/// Writes the manifests and stubs of every unit of `zones` into the mod folder `mod_dir`. The world's own manifest is written
/// only when `start` is given (it is made once, so it stays the same afterwards).
pub fn write(mod_dir: &Path, zones: &[Zone], world: Option<(&str, [f32; 3], f32)>) -> Result<Written> {
    let mut written = Written::default();
    let pack_folder = mod_dir.join(WORLDS_FOLDER).join(WORLD_ID);
    for (zone, section) in numbered(zones) {
        let manifest = pack_folder.join(format!("{}.toml", unit_key(zone.unit)));
        written.changed += usize::from(write_if_changed(&manifest, unit_manifest(&zone, section).as_bytes())?);
        let stub = mod_dir.join("files").join(unit_file(zone.unit));
        let bytes = game_install::lazy_stub(OWNER, &lazy_key(zone.unit));
        written.changed += usize::from(write_if_changed(&stub, &bytes)?);
        written.units += 1;
    }
    if let Some((name, start, heading)) = world {
        let path = mod_dir.join(WORLDS_FOLDER).join(format!("{WORLD_ID}.toml"));
        written.changed += usize::from(write_if_changed(&path, world_manifest(name, start, heading).as_bytes())?);
    }
    Ok(written)
}

#[cfg(test)]
mod tests {
    use super::*;
    use blackbox_modding::content::{parse_scenery, parse_world};

    fn zone(unit: u32) -> Zone {
        Zone { unit, district: 1, min: [0.0, 100.0], max: [300.0, 500.0] }
    }

    #[test]
    fn section_numbers_stay_in_the_free_band_and_never_repeat() {
        let numbers: Vec<u32> = (0..169).map(section_number).collect();
        assert_eq!(numbers[0], 2700);
        assert_eq!(numbers[40], 2800);
        assert!(numbers.iter().all(|n| (0..=39).contains(&(n % 100))));
        let mut sorted = numbers.clone();
        sorted.dedup();
        assert_eq!(sorted.len(), 169);
    }

    #[test]
    fn units_are_numbered_by_their_own_number_whatever_the_list_order() {
        let numbered = numbered(&[zone(300), zone(5), zone(40)]);
        let order: Vec<(u32, u32)> = numbered.iter().map(|(zone, section)| (zone.unit, *section)).collect();
        assert_eq!(order, [(5, 2700), (40, 2701), (300, 2702)]);
    }

    #[test]
    fn the_manifests_parse_with_the_game_schemas() {
        let pack = parse_scenery("u161", &unit_manifest(&zone(161), 2705)).unwrap();
        assert!(pack.placed_by_file && pack.places.is_empty());
        assert_eq!((pack.section, pack.file.as_str()), (2705, "WORLDS/FAIRHAVEN/U161.BUN"));
        assert_eq!(pack.anchor[0..2], [300.0, 150.0]);
        assert_eq!(pack.anchor[3], 250.0);
        let world =
            parse_world(WORLD_ID, &world_manifest("Fairhaven (NFS Most Wanted 2012)", [1.0, 2.0, 3.0], 90.0)).unwrap();
        assert_eq!(world.start, [1.0, 2.0, 3.0]);
    }

    #[test]
    fn a_stub_names_the_unit_and_a_second_write_changes_nothing() {
        let dir = std::env::temp_dir().join(format!("fairhaven-manifest-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let zones = [zone(7), zone(8)];
        let first = write(&dir, &zones, Some(("Fairhaven", [0.0, 0.0, 10.0], 0.0))).unwrap();
        assert_eq!((first.units, first.changed), (2, 2 * 2 + 1));
        let stub = std::fs::read(dir.join("files").join(unit_file(7))).unwrap();
        assert_eq!(unit_of_key(std::str::from_utf8(&stub).unwrap().lines().nth(2).unwrap()), Some(7));
        let second = write(&dir, &zones, Some(("Fairhaven", [0.0, 0.0, 10.0], 0.0))).unwrap();
        assert_eq!(second.changed, 0);
        assert_eq!(unit_of_key("unit/12x"), None);
        std::fs::remove_dir_all(&dir).unwrap();
    }
}

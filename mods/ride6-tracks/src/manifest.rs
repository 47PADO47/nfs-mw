//! The manifests of a world: `worlds/<id>.toml` and one `worlds/<id>/g<nnn>.toml` per cell (`docs/specs/second-world.md`).
//! A cell's manifest is a `scenery/` pack manifest: its solids are placed where their centres are, the first one carries the
//! cell's collision.

use std::fmt::Write;

/// The folder of the mod that holds the world manifests (`blackbox_modding::content::WORLDS_FOLDER`).
pub const WORLDS_FOLDER: &str = "worlds";

/// The section number of cell `index`: the free band 2700 and up, 40 numbers (last two digits 0 to 39) in each hundred
/// (`docs/specs/mod-world-content.md`, 1.3).
pub fn section_number(index: usize) -> u32 {
    2700 + (index / 40) as u32 * 100 + (index % 40) as u32
}

/// The most cells a world can have with section numbers that fit the streaming index (`i16`).
pub const MAX_CELLS: usize = ((32_767 - 2700) / 100) * 40;

/// The id a world gets from the track's name: lower case letters and digits.
pub fn world_id(track: &str) -> String {
    track.chars().filter(char::is_ascii_alphanumeric).map(|c| c.to_ascii_lowercase()).collect()
}

/// The key of cell `index` (the manifest's file name).
pub fn cell_key(index: usize) -> String {
    format!("g{index:03}")
}

/// The pack file of cell `index` of world `id`, under the mod's `files/`.
pub fn cell_file(id: &str, index: usize) -> String {
    format!("WORLDS/{}/G{index:03}.BUN", id.to_ascii_uppercase())
}

/// One placed solid of a cell.
pub struct Place<'a> {
    pub model: &'a str,
    pub pos: [f32; 3],
    /// Whether the placement carries the pack's static collision.
    pub collision: bool,
    /// Whether the model is also its own low-detail model: the game draws nothing for a placement with an empty low-detail slot
    /// where the density rule picks it, and the far ground is meant to be seen from far.
    pub own_coarse: bool,
}

/// The text of a cell's manifest.
pub fn cell_manifest(file: &str, section: u32, anchor: [f32; 4], places: &[Place<'_>]) -> String {
    let mut text = String::new();
    let _ = writeln!(text, "[pack]");
    let _ = writeln!(text, "file = \"{file}\"");
    let _ = writeln!(text, "section = {section}");
    let _ = writeln!(text, "anchor = [{:.1}, {:.1}, {:.1}, {:.1}]", anchor[0], anchor[1], anchor[2], anchor[3]);
    for place in places {
        let _ = writeln!(text, "\n[[place]]");
        let _ = writeln!(text, "model = \"{}\"", place.model);
        let _ = writeln!(text, "pos = [{:.1}, {:.1}, {:.1}]", place.pos[0], place.pos[1], place.pos[2]);
        if place.own_coarse {
            let _ = writeln!(text, "coarse = \"{}\"", place.model);
        }
        if place.collision {
            let _ = writeln!(text, "collision = \"static\"");
        }
    }
    text
}

/// The text of the world's manifest.
pub fn world_manifest(name: &str, start: [f32; 3], heading: f32) -> String {
    format!(
        "[world]\nname = \"{name}\"\nstart = {{ pos = [{:.2}, {:.2}, {:.2}], heading = {heading:.1} }}\n",
        start[0], start[1], start[2]
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use blackbox_modding::content::{Collision, parse_scenery, parse_world};

    #[test]
    fn section_numbers_stay_in_the_free_band_and_never_repeat() {
        let numbers: Vec<u32> = (0..200).map(section_number).collect();
        assert_eq!(numbers[0], 2700);
        assert_eq!(numbers[39], 2739);
        assert_eq!(numbers[40], 2800);
        assert!(numbers.iter().all(|n| (0..=39).contains(&(n % 100))));
        let mut sorted = numbers.clone();
        sorted.dedup();
        assert_eq!(sorted.len(), 200);
        assert!(section_number(MAX_CELLS - 1) <= 32_767);
    }

    #[test]
    fn ids_and_names_follow_the_rules_of_the_game() {
        assert_eq!(world_id("Blue Wave-Circuit"), "bluewavecircuit");
        assert_eq!(cell_key(7), "g007");
        assert_eq!(cell_file("okayama", 7), "WORLDS/OKAYAMA/G007.BUN");
    }

    #[test]
    fn the_manifests_parse_with_the_game_schemas() {
        let places = [
            Place { model: "G007_0", pos: [105.0, -20.5, 7.0], collision: true, own_coarse: true },
            Place { model: "G007_1", pos: [105.0, -20.5, 7.0], collision: false, own_coarse: false },
        ];
        let text = cell_manifest("WORLDS/OKAYAMA/G007.BUN", 2707, [105.0, -20.5, 7.0, 220.0], &places);
        let pack = parse_scenery("g007", &text).unwrap();
        assert_eq!((pack.section, pack.anchor[3]), (2707, 220.0));
        assert_eq!(pack.places.len(), 2);
        assert_eq!(pack.places[0].collision, Collision::Static);
        assert_eq!(pack.places[1].collision, Collision::None);
        assert_eq!(pack.places[0].coarse.as_deref(), Some("G007_0"));
        assert_eq!(pack.places[1].coarse, None);
        let world = parse_world("okayama", &world_manifest("Okayama (RIDE 6)", [-228.0, -214.7, 26.1], 95.1)).unwrap();
        assert_eq!(world.name, "Okayama (RIDE 6)");
        assert_eq!(world.start[2], 26.1);
    }
}

//! The licence plate of a 2012 car as the `[customization.plate]` table of its manifest (docs/formats/chameleon-plate.md).
//!
//! The plate text of the 2012 game is the player's registration, which defaults to one of five strings of the executable
//! by the game pack the car belongs to. The car carries the mesh (eight quads) and the glyph sheet. The mod gives each car
//! its default text, the sheet under a name of the car's own, and the layout of the sheet; the game then draws the sheet
//! again with any text the player types (docs/modding/car-customization.md, "License plates").

use std::path::Path;

use chameleon_bundle::Bundle;
use chameleon_vehicle::{
    GamePack, PLATE_COLUMNS, PLATE_GLYPHS, PLATE_ROWS, PLATE_SLOTS, PlateDesign, game_packs, is_plate_technique,
    plate_default_for, plate_default_texts, plate_designs, plate_registration,
};

use crate::source::SourceCar;

/// The executable of the install, which holds the default texts.
const EXECUTABLE: &str = "NFS13.exe";
/// The gameplay bundle, which lists the game packs.
const GAMEPLAY: &str = "GAMELOGIC/GAMEPLAY.BNDL";

/// The default plate texts of an install and the game packs that choose among them.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Plates {
    packs: Vec<GamePack>,
    texts: [String; 5],
    /// The plate designs of Autolog, in the order of the game's list (`crate::plate_designs`); empty when they do not read.
    designs: Vec<PlateDesign>,
}

impl Plates {
    /// Read the texts and the packs of the install at `root`. `None`, with a warning, when the executable does not hold the
    /// texts (another version of the game) or the gameplay bundle does not read.
    pub fn read(root: &Path) -> Option<Self> {
        let found = Self::try_read(root);
        if found.is_none() {
            log::warn!(
                "{}: the default licence plate texts cannot be read: the cars keep the sheet as drawn",
                root.display()
            );
        }
        found
    }

    fn try_read(root: &Path) -> Option<Self> {
        let texts = plate_default_texts(&std::fs::read(root.join(EXECUTABLE)).ok()?)?;
        let gameplay = std::fs::read(root.join(GAMEPLAY)).ok()?;
        let bundle = Bundle::parse(&gameplay).ok()?;
        let packs = game_packs(&bundle).ok()?;
        // The designs are an extra: an install whose list does not read still has its texts.
        let designs = plate_designs(&bundle).unwrap_or_default();
        Some(Self { packs, texts, designs })
    }

    /// The text a car wears by default when its vehicle list entry names the game pack `pack` (`GamePackID`): the text of the
    /// pack, as the plate keeps it. A pack the gameplay bundle does not list counts as the base game.
    pub fn text(&self, pack: u32) -> String {
        plate_registration(plate_default_for(&self.texts, self.pack_number(pack)))
    }

    /// The plate designs of the install, in the order of the game's list.
    pub fn designs(&self) -> &[PlateDesign] {
        &self.designs
    }

    /// The number of the game pack with id `pack` (0 for the base game, 1 to 4 for the DLC packs); a pack the gameplay bundle
    /// does not list counts as the base game.
    pub fn pack_number(&self, pack: u32) -> u16 {
        self.packs.iter().find(|candidate| candidate.id == pack).map_or(0, |found| found.number)
    }
}

/// The name of the car's glyph sheet in its own `TEXTURES.BIN`.
pub fn picture_name(folder: &str) -> String {
    format!("{folder}_PLATE")
}

/// Whether the car has a plate whose glyph sheet can be carried over: a plate mesh whose diffuse texture the old engine reads.
pub fn present(car: &SourceCar) -> bool {
    car.vehicle.body.iter().flatten().any(|mesh| {
        let sheet = mesh.appearance.diffuse_texture.and_then(|id| car.textures.get(&id));
        is_plate_technique(&mesh.appearance.technique) && sheet.is_some_and(crate::textures::can_carry)
    })
}

/// The `[customization.plate]` table, to follow the `[customization]` table of the car's manifest (which it needs: a plate
/// is part of a car's own customization). `text` is the car's default text, left out when it is not known.
pub fn manifest_table(text: Option<&str>, picture: &str) -> String {
    let mut table = String::from("\n[customization.plate]\n");
    if let Some(text) = text {
        table.push_str(&format!("text = \"{text}\"\n"));
    }
    table.push_str(&format!(
        "picture = \"{picture}\"\nglyphs = \"{PLATE_GLYPHS}\"\ncolumns = {PLATE_COLUMNS}\nrows = {PLATE_ROWS}\nslots = {PLATE_SLOTS}\n"
    ));
    // The game draws a typed zero as the letter O, which is how the sheet has no zero.
    table.push_str("aliases = { \"0\" = \"O\" }\n");
    table
}

#[cfg(test)]
mod tests {
    use super::*;

    fn plates() -> Plates {
        let texts = ["NEED4SPD", "ULTIMATE", "VELOCITY", "MOVILGND", "NFS HERO"].map(String::from);
        let packs = [(0, 0x42E7), (1, 0x2045BB), (2, 0x21836A), (3, 0x2184B4), (4, 0x2184B5)]
            .map(|(number, id)| GamePack { id, number })
            .to_vec();
        Plates { packs, texts, designs: Vec::new() }
    }

    #[test]
    fn a_car_wears_the_text_of_its_game_pack() {
        let plates = plates();
        assert_eq!(plates.text(0x42E7), "NEED4SPD");
        assert_eq!(plates.text(0x2045BB), "ULTIMATE");
        assert_eq!(plates.text(0x21836A), "VELOCITY");
        assert_eq!(plates.text(0x2184B4), "MOVILGND");
        assert_eq!(plates.text(0x2184B5), "NFS HERO");
    }

    #[test]
    fn a_pack_the_game_does_not_list_wears_the_base_game_text() {
        assert_eq!(plates().text(0), "NEED4SPD");
        assert_eq!(plates().text(0xDEAD), "NEED4SPD");
    }

    #[test]
    fn the_table_gives_the_sheet_its_layout_and_the_text() {
        let table = manifest_table(Some("VELOCITY"), "POR911_PLATE");
        assert!(table.starts_with("\n[customization.plate]\n"));
        assert!(table.contains("text = \"VELOCITY\"\n"));
        assert!(table.contains("picture = \"POR911_PLATE\"\n"));
        assert!(table.contains("glyphs = \"ABCDEFGHIJKLMNOPQRSTUVWXYZ 123456789\"\n"));
        assert!(table.contains("columns = 9\nrows = 4\nslots = 8\n"));
        assert!(table.contains("aliases = { \"0\" = \"O\" }"));
    }

    #[test]
    fn a_car_without_a_known_text_keeps_the_sheet_as_drawn() {
        assert!(!manifest_table(None, "X_PLATE").contains("text ="));
    }

    #[test]
    fn the_sheet_is_named_for_the_car() {
        assert_eq!(picture_name("POR911"), "POR911_PLATE");
    }
}

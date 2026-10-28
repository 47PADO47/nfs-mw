//! The corona records of a car in full: the lamp records of every `VehicleVfxBehaviour` object of the bundle (the ordinary
//! lamps and, on a police car, the siren object `<Car>_Cop_Coronas`), each with the references to its `Corona` object and
//! to its flash pattern. [`crate::read_lamps`] reads the first object's records for the lamp markers; this reads all of them
//! with the references. Spec: `docs/formats/chameleon-corona.md`, `docs/formats/chameleon-lamps.md`.

use chameleon_bundle::{Bundle, ResourceKind};

use crate::error::Result;
use crate::genesys::TypeNames;

const CORONA_TYPE: &str = "Genesys.Gen.VehicleVfxBehaviour.Corona";
const TYPE_FLAG: u32 = 0x8000_0000;
/// A record is 0x50 bytes.
const RECORD_LEN: usize = 0x50;
/// Where an object keeps its name.
const NAME_AT: usize = 0x30;
const MAX_NAME: usize = 96;

/// One corona record of a car.
#[derive(Debug, Clone, PartialEq)]
pub struct CoronaRecord {
    /// The name of the object that holds the record (`Porsche_911_Targa4S_Coronas`, `Dodge_Charger_SRT8_Cop_Coronas`).
    pub object: String,
    /// The locator group the record sits at.
    pub group: u32,
    /// The bit of the vehicle's light-state word that lights it.
    pub state_bit: u16,
    pub mode: u16,
    /// The record's colour factor.
    pub tint: [f32; 4],
    /// +0x34: an intensity factor.
    pub intensity: f32,
    /// +0x38 and +0x3C: the size factors.
    pub size: [f32; 2],
    /// +0x44.
    pub flag: u8,
    /// The `Corona` object (+0x28), an id of the car's bundle.
    pub corona: Option<u64>,
    /// The flash pattern object (+0x20) of a siren, an id of the car's bundle.
    pub flash: Option<u64>,
}

/// Every corona record of the bundle, object by object, in memory order.
pub fn read_corona_records(bundle: &Bundle) -> Result<Vec<CoronaRecord>> {
    let types = TypeNames::read(bundle)?;
    let kind = ResourceKind::GenesysObject.id();
    let mut out = Vec::new();
    for (index, entry) in bundle.entries().iter().enumerate() {
        if entry.kind != kind || entry.import_count == 0 {
            continue;
        }
        let resource = bundle.resource(index)?;
        let main = &resource.blocks[0];
        let mut at: Vec<usize> = resource
            .imports
            .iter()
            .filter(|import| import.offset & TYPE_FLAG != 0 && types.name(import.id) == Some(CORONA_TYPE))
            .map(|import| (import.offset & !TYPE_FLAG) as usize)
            .filter(|&at| at + RECORD_LEN <= main.len())
            .collect();
        if at.is_empty() {
            continue;
        }
        at.sort_unstable();
        let object = name_of(main);
        let target = |slot: usize| resource.imports.iter().find(|i| i.slot() as usize == slot).map(|i| i.id);
        for at in at {
            out.push(CoronaRecord {
                object: object.clone(),
                group: word(main, at + 0x30),
                state_bit: half(main, at + 0x40),
                mode: half(main, at + 0x42),
                tint: [0x10, 0x14, 0x18, 0x1C].map(|o| float(main, at + o)),
                intensity: float(main, at + 0x34),
                size: [float(main, at + 0x38), float(main, at + 0x3C)],
                flag: main[at + 0x44],
                corona: target(at + 0x28),
                flash: target(at + 0x20),
            });
        }
    }
    Ok(out)
}

/// The zero-terminated name at the start of an object's text.
fn name_of(main: &[u8]) -> String {
    let Some(tail) = main.get(NAME_AT..) else { return String::new() };
    let end = tail.iter().take(MAX_NAME).position(|&byte| byte == 0).unwrap_or(0);
    String::from_utf8_lossy(&tail[..end]).into_owned()
}

fn word(block: &[u8], at: usize) -> u32 {
    u32::from_le_bytes([block[at], block[at + 1], block[at + 2], block[at + 3]])
}

fn half(block: &[u8], at: usize) -> u16 {
    u16::from_le_bytes([block[at], block[at + 1]])
}

fn float(block: &[u8], at: usize) -> f32 {
    f32::from_bits(word(block, at))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_name_ends_at_its_terminator() {
        let mut main = vec![0u8; 0x60];
        main[0x30..0x30 + 11].copy_from_slice(b"Some_Car_Co");
        assert_eq!(name_of(&main), "Some_Car_Co");
        assert_eq!(name_of(&[0u8; 8]), "");
    }

    /// Every car has its coronas with references to corona objects of its own bundle, and a police car's siren records
    /// have flash patterns (`CHAMELEON_GAME_DIR`).
    #[test]
    #[ignore = "reads the game install (CHAMELEON_GAME_DIR)"]
    fn every_car_has_corona_records_with_corona_objects_and_a_cop_has_flashing_sirens() {
        let Some(dir) = std::env::var_os("CHAMELEON_GAME_DIR") else { return };
        let (mut cars, mut records, mut flashing) = (0, 0, 0);
        for entry in std::fs::read_dir(std::path::Path::new(&dir).join("VEHICLES")).unwrap().flatten() {
            let file = entry.file_name().to_string_lossy().to_uppercase();
            if !(file.starts_with("VEH_") && file.ends_with("_HI.BNDL")) {
                continue;
            }
            let data = std::fs::read(entry.path()).unwrap();
            let bundle = Bundle::parse(&data).unwrap();
            let found = read_corona_records(&bundle).unwrap_or_else(|error| panic!("{file}: {error}"));
            assert!(found.iter().all(|r| r.corona.is_some_and(|id| bundle.find(id).is_some())), "{file}");
            cars += 1;
            records += found.len();
            flashing += found.iter().filter(|r| r.flash.is_some()).count();
        }
        println!("{cars} cars, {records} corona records, {flashing} with a flash pattern");
        assert!(cars > 50 && records > 900 && flashing >= 16, "{cars} {records} {flashing}");
    }
}

//! The licence plates of the converted cars, with the real install: each car's manifest names its default text (that of its
//! game pack) and its glyph sheet, the sheet is in the car's `TEXTURES.BIN`, and the game's own plate code draws another text
//! on it. Set `MW2012_GAME_DIR` (the 2012 install) and run with `--ignored`.

use std::path::{Path, PathBuf};

use blackbox_hash::bstring_hash;
use blackbox_modding::content::parse_car;
use blackbox_tpk::read_texture_packs;
use nfsmw_2012_content::config::Config;
use nfsmw_data::car::mods::custom::Plate;

fn convert(source: &Path, cars: &str) -> (PathBuf, nfsmw_2012_content::import::Report) {
    let dir = std::env::temp_dir().join(format!("nfsmw-2012-content-plate-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let config = Config::parse(&format!("cars = [{cars}]\n")).unwrap();
    let report = nfsmw_2012_content::import::run(&config, source, &dir, 2).unwrap();
    assert!(report.skipped.is_empty(), "skipped: {:?}", report.skipped);
    (dir, report)
}

fn plate_of(dir: &Path, id: &str) -> blackbox_modding::content::Plate {
    let text = std::fs::read_to_string(dir.join("cars").join(format!("{}.toml", id.to_ascii_lowercase()))).unwrap();
    parse_car(id, &text).unwrap().customization.unwrap().plate.expect("the car has a plate")
}

#[test]
#[ignore = "reads the 2012 install (MW2012_GAME_DIR)"]
fn a_car_wears_the_text_of_its_game_pack_on_a_sheet_of_its_own() {
    let Some(source) = std::env::var_os("MW2012_GAME_DIR") else {
        return;
    };
    let (dir, report) = convert(Path::new(&source), "'POR911CARR12', 'BMWM3E46GT01'");
    let text_of = |id: &str| plate_of(&dir, id).text;
    let porsche = report.imported.iter().find(|car| car.id == "POR911CARR12").unwrap();
    let hero = report.imported.iter().find(|car| car.id == "BMWM3E46GT01").unwrap();
    assert_eq!(text_of(&porsche.id).as_deref(), Some("NEED4SPD"), "a base game car");
    assert_eq!(text_of(&hero.id).as_deref(), Some("NFS HERO"), "a car of the NFS Heroes pack");

    let table = plate_of(&dir, &porsche.id);
    assert_eq!((table.columns, table.rows, table.slots), (9, 4, 8));
    assert_eq!(table.picture, format!("{}_PLATE", porsche.id));
    let textures = std::fs::read(dir.join("files/CARS").join(&porsche.id).join("TEXTURES.BIN")).unwrap();
    let packs = read_texture_packs(&textures).unwrap();
    let sheet = packs
        .iter()
        .flat_map(|pack| &pack.textures)
        .find(|texture| texture.name_hash == bstring_hash(&table.picture))
        .expect("the glyph sheet is in the car's textures");
    assert_eq!((sheet.width, sheet.height), (512, 512));

    // The game's plate code draws another text on the sheet.
    let plate = Plate {
        text: table.text.clone(),
        picture: table.picture.clone(),
        glyphs: table.glyphs.clone(),
        columns: table.columns,
        rows: table.rows,
        slots: table.slots,
        aliases: table.aliases.clone(),
        designs: table.designs.clone(),
        default_design: table.default_design.clone(),
    };
    let drawn = plate.compose(sheet, "VELOCITY").unwrap();
    assert_eq!((drawn.width, drawn.height, drawn.mip_levels), (512, 512, 10));
    assert_eq!(drawn.name_hash, sheet.name_hash);
    let plain = plate.compose(sheet, "ABCDEFGH").unwrap();
    assert_ne!(drawn.data, plain.data, "another text is another picture");
    let again = plate.compose(sheet, "ABCDEFGH").unwrap();
    assert_eq!(plain.data, again.data);
}

#[test]
#[ignore = "reads the 2012 install (MW2012_GAME_DIR)"]
fn the_vehicle_list_gives_every_entry_one_of_five_game_packs() {
    let Some(source) = std::env::var_os("MW2012_GAME_DIR") else {
        return;
    };
    let source = Path::new(&source);
    let list = std::fs::read(source.join("VEHICLES/VEHICLELIST.BNDL")).unwrap();
    let bundle = chameleon_bundle::Bundle::parse(&list).unwrap();
    let kind = chameleon_bundle::ResourceKind::VehicleList.id();
    let index = bundle.entries().iter().position(|entry| entry.kind == kind).unwrap();
    let block = bundle.resource(index).unwrap().blocks[0].clone();
    let word = |at: usize| u32::from_le_bytes(block[at..at + 4].try_into().unwrap());
    let mut counts = std::collections::BTreeMap::new();
    for n in 0..word(4) as usize {
        *counts.entry(word(word(12) as usize + n * 0x70 + 0x2C)).or_insert(0) += 1;
    }
    println!("entries by GamePackID: {counts:x?}");
    // The base game, and the four packs of 5, 5, 5 and 25 entries (with the liveries of the heroes).
    assert_eq!(counts.len(), 5, "{counts:x?}");
    assert_eq!(counts.values().sum::<i32>(), 110);
}

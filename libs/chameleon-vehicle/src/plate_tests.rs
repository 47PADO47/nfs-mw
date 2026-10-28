use std::path::Path;

use chameleon_bundle::{Bundle, Pool};

use super::*;

#[test]
fn the_glyph_sheet_has_one_glyph_per_cell() {
    assert_eq!(GLYPHS.chars().count(), (COLUMNS * ROWS) as usize);
    assert_eq!(GLYPHS.as_bytes()[BLANK as usize], b' ');
    assert_eq!(GLYPHS.find('1'), Some(27), "the digits fill the last row");
}

#[test]
fn a_registration_is_upper_case_with_o_for_zero_and_spaces_for_the_unknown() {
    assert_eq!(registration("need4spd"), "NEED4SPD");
    assert_eq!(registration("R0ADS"), "ROADS");
    assert_eq!(registration("NFS_HERO"), "NFS HERO");
    assert_eq!(registration("a-b"), "A B", "the dash is a drawing of the blank, not a character");
    assert_eq!(registration("é!"), "  ");
}

#[test]
fn eight_characters_fill_the_plate_and_fewer_are_centred() {
    let cell = |c: char| GLYPHS.find(c).unwrap() as u8;
    let full = cells("VELOCITY");
    assert_eq!(full.map(|n| GLYPHS.as_bytes()[n as usize] as char), ['V', 'E', 'L', 'O', 'C', 'I', 'T', 'Y']);
    let short = cells("ABC");
    assert_eq!(
        short,
        [BLANK, BLANK, cell('A'), cell('B'), cell('C'), BLANK, BLANK, BLANK],
        "(8 - 3) / 2 = 2 blanks first"
    );
    assert_eq!(cells("AB")[3], cell('A'));
    assert_eq!(cells(""), [BLANK; SLOTS]);
}

#[test]
fn a_space_and_an_unknown_character_show_the_blank_cell() {
    assert_eq!(cells("NFS HERO")[3], BLANK);
    assert_eq!(cells("NFS_HERO")[3], BLANK, "the underscore is not on the sheet");
    assert_eq!(cells("A0")[4], GLYPHS.find('O').unwrap() as u8, "zero is drawn as the letter O");
}

#[test]
fn a_text_longer_than_the_plate_is_cut_at_eight() {
    assert_eq!(cells("ABCDEFGHIJ"), cells("ABCDEFGH"));
}

#[test]
fn the_plate_is_found_by_its_technique() {
    assert!(is_plate_technique("VehicleNFS13_Body_Textured_NormalMap_Lightmap_Licenseplate"));
    assert!(!is_plate_technique("VehicleNFS13_Body_Textured_NormalMap_Lightmap"));
}

/// An executable image with the default texts at an odd address, one false start before them.
fn exe(texts: &[&str]) -> Vec<u8> {
    let mut bytes = b"..NEED4SPD\0 not a record..".to_vec();
    for text in texts {
        let mut record = text.as_bytes().to_vec();
        record.resize(RECORD, 0);
        bytes.extend(record);
    }
    bytes.extend_from_slice(b"Normal");
    bytes
}

#[test]
fn the_default_texts_are_the_five_records_after_need4spd() {
    let mut bytes = b"\x01\x02".to_vec();
    bytes.extend(exe(&["NEED4SPD", "ULTIMATE", "VELOCITY", "MOVILGND", "NFS HERO"]));
    let texts = default_texts(&bytes).unwrap();
    assert_eq!(texts.each_ref().map(String::as_str), ["NEED4SPD", "ULTIMATE", "VELOCITY", "MOVILGND", "NFS HERO"]);
}

#[test]
fn an_executable_without_the_records_has_no_default_texts() {
    assert_eq!(default_texts(b"nothing here"), None);
    assert_eq!(default_texts(&exe(&["NEED4SPD", "ULTIMATE"])), None, "cut short");
}

#[test]
fn a_pack_without_a_text_of_its_own_takes_the_base_game_text() {
    let texts = ["BASE", "ONE", "TWO", "THREE", "FOUR"].map(String::from);
    assert_eq!(
        [0, 1, 2, 3, 4, 5, 99].map(|number| default_for(&texts, number)),
        ["BASE", "ONE", "TWO", "THREE", "FOUR", "BASE", "BASE"]
    );
}

fn install() -> Option<std::path::PathBuf> {
    std::env::var_os("CHAMELEON_GAME_DIR").map(Into::into)
}

#[test]
#[ignore = "reads the game install (CHAMELEON_GAME_DIR)"]
fn the_install_has_five_game_packs_numbered_zero_to_four() {
    let Some(dir) = install() else { return };
    let data = std::fs::read(dir.join("GAMELOGIC/GAMEPLAY.BNDL")).unwrap();
    let packs = game_packs(&Bundle::parse(&data).unwrap()).unwrap();
    let ids: Vec<(u16, u32)> = packs.iter().map(|pack| (pack.number, pack.id)).collect();
    assert_eq!(ids, [(0, 0x42E7), (1, 0x2045BB), (2, 0x21836A), (3, 0x2184B4), (4, 0x2184B5)]);
}

#[test]
#[ignore = "reads the game install (CHAMELEON_GAME_DIR)"]
fn the_executable_holds_the_five_default_texts() {
    let Some(dir) = install() else { return };
    let exe = std::fs::read(dir.join("NFS13.exe")).unwrap();
    let texts = default_texts(&exe).unwrap();
    assert_eq!(texts.each_ref().map(String::as_str), ["NEED4SPD", "ULTIMATE", "VELOCITY", "MOVILGND", "NFS HERO"]);
}

/// The distinct values of `values`, sorted.
fn distinct(values: impl Iterator<Item = f32>) -> Vec<f32> {
    let mut all: Vec<f32> = values.collect();
    all.sort_by(f32::total_cmp);
    all.dedup_by(|a, b| (*a - *b).abs() < 2e-3);
    all
}

#[test]
#[ignore = "reads the game install (CHAMELEON_GAME_DIR)"]
fn every_car_has_one_plate_of_eight_quads_showing_the_first_eight_cells() {
    let Some(dir) = install() else { return };
    let textures = std::fs::read(dir.join("VEHICLES/VEHICLETEX.BNDL")).unwrap();
    let shaders = std::fs::read(dir.join("SHADERS.BNDL")).unwrap();
    let (mut cars, mut without) = (0, Vec::new());
    for entry in std::fs::read_dir(dir.join("VEHICLES")).unwrap().flatten() {
        let file = entry.file_name().to_string_lossy().to_uppercase();
        if !(file.starts_with("VEH_") && file.ends_with("_HI.BNDL")) {
            continue;
        }
        let data = std::fs::read(entry.path()).unwrap();
        let bundle = Bundle::parse(&data).unwrap();
        let mut pool = Pool::new();
        pool.add(bundle.clone());
        pool.add(Bundle::parse(&textures).unwrap());
        pool.add(Bundle::parse(&shaders).unwrap());
        let vehicle = crate::load(&pool, crate::find_spec(&bundle).unwrap()).unwrap();
        let plates: Vec<_> =
            vehicle.body[0].iter().filter(|mesh| is_plate_technique(&mesh.appearance.technique)).collect();
        if plates.is_empty() {
            without.push(file);
            continue;
        }
        assert_eq!(plates.len(), 1, "{file}: one plate mesh at the finest level");
        let plate = plates[0];
        assert_eq!((plate.vertices.len(), plate.triangles.len()), (32, 16), "{file}: eight quads");
        let columns = distinct(plate.vertices.iter().map(|v| v.uv[0]));
        assert_eq!(columns.len(), 16, "{file}: two texture columns per quad");
        for slot in 0..SLOTS {
            let left = columns[2 * slot];
            let cell_start = slot as f32 / COLUMNS as f32;
            assert!(
                (cell_start..cell_start + 0.01).contains(&left),
                "{file}: quad {slot} starts at {left}, cell at {cell_start}"
            );
        }
        assert!(plate.vertices.iter().all(|v| (0.0..0.25).contains(&v.uv[1])), "{file}: the first row of the sheet");
        cars += 1;
    }
    println!("{cars} cars with a plate, without: {without:?}");
    assert_eq!((cars, without.len()), (91, 2), "{without:?}");
}

#[test]
#[ignore = "reads the game install (CHAMELEON_GAME_DIR)"]
fn the_glyph_sheet_is_the_plates_diffuse_texture() {
    let Some(dir) = install() else { return };
    let textures = std::fs::read(dir.join("VEHICLES/VEHICLETEX.BNDL")).unwrap();
    let shaders = std::fs::read(dir.join("SHADERS.BNDL")).unwrap();
    let data = std::fs::read(Path::new(&dir).join("VEHICLES/VEH_1085007_HI.BNDL")).unwrap();
    let bundle = Bundle::parse(&data).unwrap();
    let mut pool = Pool::new();
    pool.add(bundle.clone());
    pool.add(Bundle::parse(&textures).unwrap());
    pool.add(Bundle::parse(&shaders).unwrap());
    let vehicle = crate::load(&pool, crate::find_spec(&bundle).unwrap()).unwrap();
    let plate = vehicle.body[0].iter().find(|mesh| is_plate_technique(&mesh.appearance.technique)).unwrap();
    let sheet = crate::load_texture(&pool, plate.appearance.diffuse_texture.unwrap()).unwrap();
    assert_eq!((sheet.width, sheet.height), (512, 512));
}

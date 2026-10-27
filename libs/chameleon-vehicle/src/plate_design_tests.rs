use chameleon_bundle::Bundle;

use super::*;

/// The main block of a design list with the given `(picture, name, pack)` records.
fn list(designs: &[(u32, u32, u32)]) -> Vec<u8> {
    let array = 0x18;
    let first = array + 4 * designs.len();
    let mut main = vec![0u8; first + RECORD * designs.len()];
    main[LIST_ARRAY..LIST_ARRAY + 4].copy_from_slice(&(array as u32).to_le_bytes());
    main[LIST_COUNT..LIST_COUNT + 4].copy_from_slice(&(designs.len() as u32).to_le_bytes());
    for (n, (picture, name, pack)) in designs.iter().enumerate() {
        let at = first + RECORD * n;
        main[array + 4 * n..array + 4 * n + 4].copy_from_slice(&(at as u32).to_le_bytes());
        main[at + PICTURE..at + PICTURE + 4].copy_from_slice(&picture.to_le_bytes());
        main[at + NAME..at + NAME + 4].copy_from_slice(&name.to_le_bytes());
        main[at + PACK..at + PACK + 4].copy_from_slice(&pack.to_le_bytes());
    }
    main
}

#[test]
fn the_list_gives_each_design_its_picture_name_and_pack() {
    let found = parse_list(&list(&[(1863272, 1347000, 0x42E7), (2106485, 1347001, 0x2045BB)])).unwrap();
    assert_eq!(
        found,
        [
            PlateDesign { picture: 1863272, name: 1347000, pack: 0x42E7 },
            PlateDesign { picture: 2106485, name: 1347001, pack: 0x2045BB },
        ]
    );
}

#[test]
fn an_empty_list_has_no_designs() {
    assert_eq!(parse_list(&list(&[])).unwrap(), []);
}

#[test]
fn a_list_cut_short_or_with_too_many_designs_is_an_error() {
    let mut main = list(&[(1, 2, 3), (4, 5, 6)]);
    main.truncate(main.len() - 8);
    assert!(parse_list(&main).is_err(), "the last record is cut");
    assert!(parse_list(&[0; 8]).is_err(), "no header");
    let mut huge = list(&[]);
    huge[LIST_COUNT..LIST_COUNT + 4].copy_from_slice(&5000u32.to_le_bytes());
    assert!(parse_list(&huge).is_err());
}

#[test]
fn a_picture_is_found_in_its_bundle_folder() {
    assert_eq!(design_picture_path(1863272), "UI/IMAGES/STREAMED/1863272.BNDL");
}

#[test]
fn the_band_is_the_strip_shaped_middle_of_the_plate() {
    let (x, y, width, height) = design_band((10, 20, 336, 168));
    assert_eq!((x, width), (10 + 26, 284), "8 percent of the plate off each side");
    assert_eq!(height, 64, "the strip's aspect: 284 / 4.4");
    assert!((width as f32 / height as f32 - STRIP_ASPECT).abs() < 0.1);
    assert_eq!(y, 20 + 87 - 32, "centred a little below the middle of the plate");
}

#[test]
fn the_band_of_a_tiny_or_a_flat_plate_stays_inside_it() {
    for (width, height) in [(1, 1), (8, 2), (64, 8), (512, 40)] {
        let (x, y, band_width, band_height) = design_band((3, 5, width, height));
        assert!(x + band_width <= 3 + width && y + band_height <= 5 + height, "{width}x{height}");
        assert!(x >= 3 && y >= 5 && band_width >= 1 && band_height >= 1);
    }
}

#[test]
fn the_plate_is_where_the_picture_is_solid() {
    // 8 x 4 pixels, solid (alpha 255) in columns 1 to 5 of rows 1 and 2, half transparent next to them.
    let mut rgba = vec![0u8; 8 * 4 * 4];
    for y in 1..3 {
        for x in 1..6 {
            rgba[(y * 8 + x) * 4 + 3] = 255;
        }
        rgba[(y * 8 + 6) * 4 + 3] = 100;
    }
    assert_eq!(design_plate(&rgba, 8, 4), Some((1, 1, 5, 2)));
    assert_eq!(design_plate(&[0; 8 * 4 * 4], 8, 4), None, "nothing solid");
    assert_eq!(design_plate(&[], 8, 4), None, "no pixels at all");
}

fn install() -> Option<std::path::PathBuf> {
    std::env::var_os("CHAMELEON_GAME_DIR").map(Into::into)
}

#[test]
#[ignore = "reads the game install (CHAMELEON_GAME_DIR)"]
fn the_install_lists_666_designs_of_the_five_packs() {
    let Some(dir) = install() else { return };
    let data = std::fs::read(dir.join("GAMELOGIC/GAMEPLAY.BNDL")).unwrap();
    let designs = plate_designs(&Bundle::parse(&data).unwrap()).unwrap();
    assert_eq!(designs.len(), 666);
    let mut packs: Vec<u32> = designs.iter().map(|design| design.pack).collect();
    packs.sort_unstable();
    packs.dedup();
    assert_eq!(packs, [0x42E7, 0x2045BB, 0x21836A, 0x2184B4, 0x2184B5]);
    let mut pictures: Vec<u32> = designs.iter().map(|design| design.picture).collect();
    pictures.sort_unstable();
    pictures.dedup();
    assert_eq!(pictures.len(), 469, "designs share pictures");
    // The first design is the first plate of the Autolog list.
    assert_eq!(designs[0], PlateDesign { picture: 1863272, name: designs[0].name, pack: 0x42E7 });
}

#[test]
#[ignore = "reads the game install (CHAMELEON_GAME_DIR)"]
fn every_picture_is_a_512_by_256_bc3_texture_with_no_mip_levels() {
    let Some(dir) = install() else { return };
    let data = std::fs::read(dir.join("GAMELOGIC/GAMEPLAY.BNDL")).unwrap();
    let designs = plate_designs(&Bundle::parse(&data).unwrap()).unwrap();
    let mut seen = std::collections::BTreeSet::new();
    for design in designs.iter().filter(|design| seen.insert(design.picture)) {
        let file = std::fs::read(dir.join(design_picture_path(design.picture))).unwrap();
        let texture = design_picture(&file, design.picture).unwrap();
        assert_eq!((texture.width, texture.height, texture.mip_levels), (512, 256, 1), "{}", design.picture);
        assert_eq!(texture.format, chameleon_gfx::PixelFormat::Bc3, "{}", design.picture);
    }
    assert_eq!(seen.len(), 469);
}

#[test]
#[ignore = "reads the game install (CHAMELEON_GAME_DIR)"]
fn the_plate_fills_the_top_left_of_every_picture() {
    let Some(dir) = install() else { return };
    let data = std::fs::read(dir.join("GAMELOGIC/GAMEPLAY.BNDL")).unwrap();
    let designs = plate_designs(&Bundle::parse(&data).unwrap()).unwrap();
    let mut areas = std::collections::BTreeSet::new();
    let mut seen = std::collections::BTreeSet::new();
    for design in designs.iter().filter(|design| seen.insert(design.picture)) {
        let file = std::fs::read(dir.join(design_picture_path(design.picture))).unwrap();
        let texture = design_picture(&file, design.picture).unwrap();
        let rgba = texture.to_rgba8().unwrap();
        areas.insert(design_plate(&rgba, 512, 256).unwrap());
    }
    println!("{areas:?}");
    assert!(
        areas.iter().all(|&(x, y, width, height)| x <= 2
            && y <= 2
            && (330..=340).contains(&width)
            && (164..=172).contains(&height)),
        "{areas:?}"
    );
}

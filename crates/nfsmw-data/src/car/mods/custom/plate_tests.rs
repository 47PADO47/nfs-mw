use blackbox_hash::bstring_hash;

use super::*;

const GLYPHS: &str = "ABCDEFGHIJKLMNOPQRSTUVWXYZ 123456789";

fn plate() -> Plate {
    Plate {
        text: Some("NEED4SPD".into()),
        picture: "CAR_PLATE".into(),
        glyphs: GLYPHS.chars().collect(),
        columns: 9,
        rows: 4,
        slots: 8,
        aliases: vec![('0', 'O')],
        designs: Some("PLATES/SET/DESIGNS.TOML".into()),
        default_design: Some("DEFAULT".into()),
    }
}

/// A sheet of 9 x 4 cells of `size` pixels, cell `n` filled with the red value `n * 4 + 1`.
fn sheet(size: usize) -> Texture {
    let (width, height) = (9 * size, 4 * size);
    let mut data = Vec::new();
    for y in 0..height {
        for x in 0..width {
            let cell = (y / size) * 9 + x / size;
            data.extend([0, 0, (cell * 4 + 1) as u8, 255]);
        }
    }
    Texture {
        name: "CAR_PLATE".into(),
        name_hash: bstring_hash("CAR_PLATE"),
        width: width as u32,
        height: height as u32,
        mip_levels: 1,
        format: PixelFormat::Argb8888,
        compression_type: 0,
        alpha_usage: AlphaUsage::None,
        alpha_sorting: false,
        alpha_blend: 0,
        flags: 0,
        data,
        palette: Vec::new(),
    }
}

/// The red value of the pixel at the middle of cell `n` of the composed sheet.
fn red_of(texture: &Texture, n: usize, size: usize) -> u8 {
    let (x, y) = ((n % 9) * size + size / 2, (n / 9) * size + size / 2);
    texture.data[(y * texture.width as usize + x) * 4 + 2]
}

#[test]
fn a_typed_character_is_upper_cased_aliased_and_kept_only_if_the_sheet_has_it() {
    let plate = plate();
    assert_eq!(plate.glyph('a'), Some('A'));
    assert_eq!(plate.glyph('0'), Some('O'));
    assert_eq!(plate.glyph(' '), Some(' '));
    assert_eq!(plate.glyph('_'), None);
    assert_eq!(plate.glyph('é'), None);
}

#[test]
fn a_registration_keeps_the_glyphs_and_blanks_the_rest() {
    let plate = plate();
    assert_eq!(plate.registration("nfs_hero"), "NFS HERO");
    assert_eq!(plate.registration("R0AD"), "ROAD");
    assert_eq!(plate.registration("TOOLONG123"), "TOOLONG1", "at most the slots");
}

#[test]
fn the_text_is_centred_in_the_slots() {
    let plate = plate();
    let cell = |c: char| GLYPHS.find(c).unwrap();
    assert_eq!(plate.cells("ABC"), [26, 26, cell('A'), cell('B'), cell('C'), 26, 26, 26]);
    assert_eq!(plate.cells("VELOCITY").len(), 8);
    assert_eq!(plate.cells("")[0], 26);
}

#[test]
fn the_player_text_wins_over_the_manifest_text() {
    let plate = plate();
    assert_eq!(plate.worn(None), Some("NEED4SPD"));
    assert_eq!(plate.worn(Some("MINE")), Some("MINE"));
    let authored = Plate { text: None, ..plate };
    assert_eq!(authored.worn(None), None, "the sheet as authored");
}

#[test]
fn composing_puts_the_glyph_cells_into_the_first_cells() {
    let plate = plate();
    let composed = plate.compose(&sheet(6), "ABCDEFGH").unwrap();
    // Cell n of the first row already holds glyph n: unchanged.
    for n in 0..8 {
        assert_eq!(red_of(&composed, n, 6), (n * 4 + 1) as u8, "cell {n}");
    }
    let composed = plate.compose(&sheet(6), "ZZ").unwrap();
    let z = GLYPHS.find('Z').unwrap();
    assert_eq!(red_of(&composed, 3, 6), (z * 4 + 1) as u8, "Z at slot 3 of the centred text");
    assert_eq!(red_of(&composed, 4, 6), (z * 4 + 1) as u8);
    assert_eq!(red_of(&composed, 0, 6), (26 * 4 + 1) as u8, "an empty slot shows the blank cell");
    assert_eq!(red_of(&composed, 8, 6), (8 * 4 + 1) as u8, "the cells past the slots are untouched");
    assert_eq!(red_of(&composed, 20, 6), (20 * 4 + 1) as u8);
}

#[test]
fn composing_keeps_the_size_and_name_and_adds_a_full_mip_chain() {
    let composed = plate().compose(&sheet(8), "GO").unwrap();
    assert_eq!((composed.width, composed.height), (72, 32));
    assert_eq!(composed.name_hash, bstring_hash("CAR_PLATE"));
    assert_eq!(composed.format, PixelFormat::Argb8888);
    assert_eq!(composed.mip_levels, 7, "72 x 32 down to 1 x 1");
    let expected: usize = (0..7).map(|level| blackbox_tpk::mip_level_size(PixelFormat::Argb8888, 72, 32, level)).sum();
    assert_eq!(composed.data.len(), expected);
}

#[test]
fn a_sheet_with_cells_of_uneven_width_still_composes() {
    // 512 pixels over 9 columns: the cells are 56 or 57 pixels wide; 50 over 9 is 5 or 6.
    let odd = Texture { width: 50, height: 20, data: vec![255; 50 * 20 * 4], ..sheet(1) };
    assert!(plate().compose(&odd, "AB").is_ok());
}

#[test]
fn a_sheet_smaller_than_its_cells_is_refused() {
    let tiny = Texture { width: 4, height: 2, data: vec![0; 32], ..sheet(1) };
    assert!(plate().compose(&tiny, "A").is_err());
}

/// A light ground sheet with a dark glyph column in cell `n` of the first row (`n` 0 to 8): cell `n` has ink down its middle.
fn inked_sheet(size: usize) -> Texture {
    let mut sheet = sheet(size);
    for pixel in sheet.data.chunks_mut(4) {
        pixel.copy_from_slice(&[200, 220, 230, 255]); // B G R A: a light beige
    }
    for y in 0..size {
        for x in 0..9 * size {
            // The middle third of every cell of the first row is ink.
            if (x % size) * 3 / size == 1 && y < size {
                let at = (y * 9 * size + x) * 4;
                sheet.data[at..at + 4].copy_from_slice(&[10, 10, 10, 255]);
            }
        }
    }
    sheet
}

fn ground(rgb: [u8; 3]) -> Ground {
    let data: Vec<u8> = (0..4 * 2).flat_map(|_| [rgb[2], rgb[1], rgb[0], 255]).collect();
    let picture = Texture { width: 4, height: 2, data, ..sheet(1) };
    Ground::new(&picture, None).unwrap()
}

/// The RGB of the pixel `(x, y)` of a composed texture (stored B G R A).
fn rgb_at(texture: &Texture, x: usize, y: usize) -> [u8; 3] {
    let at = (y * texture.width as usize + x) * 4;
    [texture.data[at + 2], texture.data[at + 1], texture.data[at]]
}

#[test]
fn a_design_replaces_the_ground_of_the_slots_and_keeps_the_ink() {
    let size = 12;
    let sheet = inked_sheet(size);
    let plate = plate();
    let plain = plate.compose_with(&sheet, &sheet, Some("ABCDEFGH"), None).unwrap();
    assert_eq!(rgb_at(&plain, 0, 0), [230, 220, 200], "no design: the sheet's own ground");
    // A light yellow picture: the glyphs take the dark ink, the ground is the picture eased toward white.
    let yellow = plate.compose_with(&sheet, &sheet, Some("ABCDEFGH"), Some(&ground([240, 200, 60]))).unwrap();
    let ground_pixel = rgb_at(&yellow, 1, 2);
    assert!(ground_pixel[0] > 235 && ground_pixel[2] < 160, "yellow ground, not beige: {ground_pixel:?}");
    let ink_pixel = rgb_at(&yellow, size / 2, 2);
    assert!(ink_pixel.iter().all(|&c| c < 60), "the glyph stays dark: {ink_pixel:?}");
}

#[test]
fn the_cells_past_the_slots_keep_the_sheets_own_ground() {
    let size = 12;
    let sheet = inked_sheet(size);
    let drawn = plate().compose_with(&sheet, &sheet, Some("AB"), Some(&ground([0, 0, 255]))).unwrap();
    // Cell 8 is the last of the first row; the plate shows 8 slots (cells 0 to 7).
    assert_eq!(rgb_at(&drawn, 8 * size + 1, 2), [230, 220, 200]);
    assert_eq!(rgb_at(&drawn, 1, size + 2), [230, 220, 200], "the second row is untouched");
}

#[test]
fn a_dark_picture_gets_light_ink_and_a_light_one_dark() {
    assert_eq!(ground([10, 10, 10]).ink, [245, 245, 238]);
    assert_eq!(ground([240, 240, 240]).ink, [20, 20, 24]);
    let picture = Texture { width: 1, height: 1, data: vec![5, 6, 7, 255], ..sheet(1) };
    assert_eq!(Ground::new(&picture, Some([1, 2, 3])).unwrap().ink, [1, 2, 3], "the design's own ink wins");
}

#[test]
fn no_text_with_a_design_shows_the_sheet_cells_as_they_are() {
    let sheet = inked_sheet(12);
    let a = plate().compose_with(&sheet, &sheet, None, Some(&ground([30, 90, 200]))).unwrap();
    let b = plate().compose_with(&sheet, &sheet, Some("ABCDEFGH"), Some(&ground([30, 90, 200]))).unwrap();
    assert_eq!(a.data, b.data, "the first eight cells of this sheet hold the same pixels in either order");
}

#[test]
fn a_sheet_with_no_contrast_takes_no_design() {
    let flat = Texture { data: vec![180; 9 * 4 * 4], width: 9, height: 4, ..sheet(1) };
    let drawn = plate().compose_with(&flat, &flat, Some("A"), Some(&ground([0, 0, 255]))).unwrap();
    assert!(drawn.data.chunks(4).take(4).all(|p| p[..3] == [180, 180, 180]));
}

#[test]
fn a_glyph_sheet_of_another_size_is_refused() {
    let (own, other) = (sheet(8), sheet(6));
    assert!(plate().compose_with(&own, &other, Some("A"), None).is_err());
}

#[test]
fn a_plate_with_more_slots_than_cells_is_refused_not_panicked_on() {
    let tall = Plate { slots: 16, columns: 3, rows: 4, glyphs: "ABCDEFGHIJK ".chars().collect(), ..plate() };
    assert!(tall.compose(&sheet(6), "A").is_err());
}

#[test]
fn the_design_worn_is_the_pick_then_the_kept_one_then_the_default() {
    let plate = plate();
    assert_eq!(plate.design_worn(Some("P"), Some("K")).as_deref(), Some("P"));
    assert_eq!(plate.design_worn(None, Some("K")).as_deref(), Some("K"));
    assert_eq!(plate.design_worn(None, None).as_deref(), Some("DEFAULT"));
    assert_eq!(plate.design_worn(Some(""), Some("K")), None, "the plain plate, picked on purpose");
    assert_eq!(plate.design_worn(None, Some("")), None, "and kept on purpose, over the default");
}

#[test]
fn a_plate_without_an_index_wears_no_design() {
    let none = Plate { designs: None, ..plate() };
    assert_eq!(none.design_worn(Some("P"), Some("K")), None);
    let plain = Plate { default_design: None, ..plate() };
    assert_eq!(plain.design_worn(None, None), None);
}

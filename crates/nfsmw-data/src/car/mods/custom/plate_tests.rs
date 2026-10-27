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

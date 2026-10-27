use nfsmw_data::car::mods::custom::plate_design::parse_index;

use super::*;

fn design(picture: u32, name: u32, pack: u32) -> PlateDesign {
    PlateDesign { picture, name, pack }
}

fn names(id: u32) -> Option<String> {
    Some(match id {
        1 => "START ME UP".to_owned(),
        2 => "RUNNING IN".to_owned(),
        3 => "  ".to_owned(),
        4 => "TAKEDOWN MASTER".to_owned(),
        _ => return None,
    })
}

fn group(pack: u32) -> &'static str {
    GROUPS[usize::from(pack == 0x2045BB)]
}

#[test]
fn a_design_is_a_picture_named_by_its_first_design() {
    let found = entries(&[design(10, 1, 0x42E7), design(11, 2, 0x42E7), design(10, 2, 0x42E7)], names, group);
    assert_eq!(found.len(), 2, "the third design shares the first's picture");
    assert_eq!(found[0], Entry { id: "10".into(), name: "START ME UP".into(), group: "Base game", picture: 10 });
    assert_eq!(found[1].id, "11");
    assert_eq!(found[0].texture(), "PD10");
}

#[test]
fn a_name_that_repeats_is_numbered_and_a_missing_one_is_made_up() {
    let found =
        entries(&[design(1, 4, 0), design(2, 4, 0), design(3, 4, 0), design(4, 3, 0), design(5, 99, 0)], names, group);
    let listed: Vec<&str> = found.iter().map(|entry| entry.name.as_str()).collect();
    assert_eq!(listed, ["TAKEDOWN MASTER", "TAKEDOWN MASTER 2", "TAKEDOWN MASTER 3", "Design 4", "Design 5"]);
}

#[test]
fn a_design_takes_the_label_of_its_pack() {
    let found = entries(&[design(1, 1, 0x42E7), design(2, 2, 0x2045BB)], names, group);
    assert_eq!((found[0].group, found[1].group), ("Base game", "Ultimate Speed Pack"));
}

#[test]
fn the_index_text_is_an_index_the_game_reads_back() {
    let found = entries(&[design(7, 1, 0x42E7), design(8, 2, 0x2045BB)], names, group);
    let set = parse_index(INDEX, &index_text(&found)).unwrap();
    assert_eq!(set.pack, "PLATES/MW2012/DESIGNS.BIN");
    assert_eq!(set.designs.len(), 2);
    assert_eq!(set.designs[0].id, "7");
    assert_eq!(set.designs[0].picture, "PD7");
    assert_eq!(set.designs[1].name, "RUNNING IN");
    assert_eq!(set.designs[1].group.as_deref(), Some("Ultimate Speed Pack"));
}

#[test]
fn quotes_and_backslashes_in_a_name_are_escaped_and_controls_dropped() {
    let entry = Entry { id: "1".into(), name: "SAY \"HI\" \\ BYE\n".into(), group: "Base game", picture: 1 };
    let set = parse_index(INDEX, &index_text(&[entry])).unwrap();
    assert_eq!(set.designs[0].name, "SAY \"HI\" \\ BYE");
}

#[test]
fn the_index_sits_in_the_folder_the_manifest_names() {
    assert!(INDEX.starts_with(FOLDER) && INDEX.ends_with("DESIGNS.TOML"));
    assert!(INDEX.len() <= 128, "the manifest limits the path");
}

/// A 512 x 256 picture the way the install has them: the plate (337 x 170, opaque) at the top left, `color` all over it, and
/// a transparent rest.
fn picture(color: [u8; 3]) -> Vec<u8> {
    let mut rgba = vec![0u8; 512 * 256 * 4];
    for y in 0..170 {
        for x in 0..337 {
            let at = (y * 512 + x) * 4;
            rgba[at..at + 4].copy_from_slice(&[color[0], color[1], color[2], 255]);
        }
    }
    rgba
}

#[test]
fn a_picture_is_cut_to_its_band_scaled_and_stored_as_dxt1() {
    let texture = band_texture("PD1", &picture([200, 30, 30]), 512, 256).unwrap();
    assert_eq!((texture.width, texture.height, texture.mip_count), (256, 64, 1));
    assert_eq!(texture.format, PixelFormat::Dxt1);
    assert_eq!(texture.data.len(), 256 / 4 * (64 / 4) * 8, "8 KB a picture");
    assert_eq!(texture.data.len(), texture.expected_data_len());
    assert_eq!(texture.name, "PD1");
}

#[test]
fn a_picture_with_no_plate_in_it_is_left_out() {
    assert!(band_texture("PD1", &vec![0; 512 * 256 * 4], 512, 256).is_none());
}

#[test]
fn resampling_averages_the_pixels_each_one_covers() {
    // 4 x 2: left half 0, right half 200; scaled to 2 x 1 over the whole picture.
    let mut rgba = Vec::new();
    for _ in 0..2 {
        for x in 0..4 {
            let value = if x < 2 { 0 } else { 200 };
            rgba.extend([value, value, value, 255]);
        }
    }
    let out = resample(&rgba, 4, (0, 0, 4, 2), (2, 1));
    assert_eq!(out, [0, 0, 0, 255, 200, 200, 200, 255]);
    // An area inside the picture, offset: only its pixels count.
    let inner = resample(&rgba, 4, (2, 0, 2, 2), (1, 1));
    assert_eq!(inner, [200, 200, 200, 255]);
}

#[test]
fn the_packed_designs_read_back_as_a_pack_of_256_by_64_textures() {
    let sources: Vec<TextureSource> =
        (1..=3).map(|n| band_texture(&format!("PD{n}"), &picture([n * 60, 100, 10]), 512, 256).unwrap()).collect();
    let pack = write_tpk("designs", &sources).unwrap();
    let read = blackbox_tpk::read_texture_packs(&pack).unwrap();
    let textures: Vec<&blackbox_tpk::Texture> = read.iter().flat_map(|pack| &pack.textures).collect();
    assert_eq!(textures.len(), 3);
    assert!(textures.iter().all(|texture| (texture.width, texture.height) == (256, 64)));
    let size = pack.len();
    assert!(size < 3 * 9000 + 4096, "about 8 KB a picture, plus the pack's own tables: {size}");
}

#[test]
fn no_default_design_is_set_by_the_2012_data() {
    assert_eq!(DEFAULT_DESIGN, None, "the install does not say which design a new player gets");
}

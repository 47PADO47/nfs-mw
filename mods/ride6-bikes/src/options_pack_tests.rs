use super::*;

const TABLE: &str = r#"{
 "schema": 1, "vehicle": "bike-option-parts",
 "parts": [
  {"id": "muffler974_001", "role": "body", "export": "muffler974",
   "anchor": {"bone": null, "translation": [0, 0, 0], "rotation": [0, 0, 0, 1]}, "materials": {}},
  {"id": "muffler974_001b", "role": "body", "export": "muffler974",
   "anchor": {"bone": null, "translation": [0, 0, 0], "rotation": [0, 0, 0, 1]}, "materials": {}},
  {"id": "rim738_001_front_rim", "role": "front_wheel", "export": "f_rim738",
   "anchor": {"bone": null, "translation": [0, 0, 0], "rotation": [0, 0, 0, 1]}, "materials": {}},
  {"id": "rim738_001_back_rim", "role": "back_wheel", "export": "b_rim738",
   "anchor": {"bone": null, "translation": [0, 0, 0], "rotation": [0, 0, 0, 1]}, "materials": {}}
 ]
}"#;

#[test]
fn a_part_pack_is_a_folder_named_after_the_asset() {
    assert_eq!(pack_path("muffler974_001"), "PARTS/R6/MUFFLER974_001");
    assert_eq!(solid_name("rim738_001_front_rim"), "RIM738_001_FRONT_RIM");
}

#[test]
fn an_asset_owns_the_parts_named_after_it_and_no_other() {
    let table = PartTable::parse(TABLE).unwrap();
    assert_eq!(part_ids(&table, "muffler974_001"), ["muffler974_001"]);
    assert_eq!(part_ids(&table, "rim738_001"), ["rim738_001_front_rim", "rim738_001_back_rim"]);
    assert!(part_ids(&table, "muffler974").is_empty());
}

#[test]
fn the_textures_of_small_parts_are_small() {
    let tiny = [Kind::Header, Kind::Muffler, Kind::BarPad, Kind::Rim, Kind::Brake].map(max_texture);
    assert_eq!(tiny, [128, 256, 256, 1024, 1024]);
    // The rims' textures are half of it and the discs' a quarter: 512 and 256 pixels.
    assert_eq!(crate::bake::size_cap(crate::partfile::Kind::Rim, max_texture(Kind::Rim)), 512);
    assert_eq!(crate::bake::size_cap(crate::partfile::Kind::Brake, max_texture(Kind::Brake)), 256);
}

#[test]
fn a_part_the_table_lacks_is_a_reason() {
    let table = PartTable::parse(TABLE).unwrap();
    let dir = std::env::temp_dir().join(format!("ride6-options-pack-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let library = Library { table, exports: Exports::scan(&dir).unwrap(), tex: dir.clone() };
    let error = library.build("nothing", Kind::Muffler, Options::default()).err().unwrap();
    assert!(format!("{error:#}").contains("no part of it"));
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn the_big_parts_have_fewer_levels_and_every_part_has_a_finest() {
    for kind in [Kind::Header, Kind::Muffler, Kind::Rim, Kind::Brake, Kind::BarPad] {
        let levels = part_levels(kind);
        assert_eq!(levels[0].0, 'A', "{kind:?}");
        assert!(levels.windows(2).all(|w| w[0].0 < w[1].0 && w[0].1 < w[1].1), "{kind:?}");
    }
    assert_eq!(part_levels(Kind::Header).len(), 2);
    assert_eq!(part_levels(Kind::Brake).len(), 4);
}

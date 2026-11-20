use blackbox_modding::content::parse_replace_set;

use super::*;

#[test]
fn the_sets_the_example_writes_are_accepted_by_the_game_parser() {
    let lamps = parse_replace_set("street-lamps", &lamp_set()).unwrap();
    assert_eq!((lamps.kind.as_str(), lamps.value.as_str()), ("street_lamps", "slim"));
    assert_eq!(lamps.model_targets(), LAMP_TARGETS.len());
    let road = parse_replace_set("road-textures", &road_set()).unwrap();
    assert_eq!(road.option().as_deref(), Some("road_textures_variant"));
    assert_eq!(road.texture_entries.len(), 1);
}

#[test]
fn the_files_the_sets_name_are_made_and_read_back() {
    let solids = blackbox_solid::read_solids(&lamp_geometry()).unwrap();
    assert_eq!(solids.len(), 1);
    assert_eq!(solids[0].name, LAMP_MODEL);
    assert!(solids[0].bounds_max[2] >= 6.0, "the lamp is 6 m tall");
    let packs = blackbox_tpk::read_texture_packs(&lamp_textures()).unwrap();
    assert_eq!(packs[0].textures[0].name_hash, bstring_hash(LAMP_TEXTURE));
    assert_eq!(&checker_png()[1..4], b"PNG");
    let paths: Vec<&str> = files().iter().map(|file| file.path).collect();
    assert!(paths.contains(&"files/EXAMPLE/CHECKER.png") && paths.last() == Some(&"replace/street-lamps.toml"));
}

#[test]
#[ignore = "writes the mod's files into the folder EXAMPLE_REPLACE_OUT (a content-only copy: mod.toml without a library)"]
fn write_the_mod() {
    let out = std::path::PathBuf::from(std::env::var_os("EXAMPLE_REPLACE_OUT").expect("set EXAMPLE_REPLACE_OUT"));
    // The names of the game's solids and texture to replace can be given: EXAMPLE_LAMP_TARGETS (comma separated), EXAMPLE_ROAD_TEXTURE.
    let lamps: Vec<String> = std::env::var("EXAMPLE_LAMP_TARGETS")
        .map(|list| list.split(',').map(str::to_owned).collect())
        .unwrap_or_else(|_| LAMP_TARGETS.iter().map(|name| (*name).to_owned()).collect());
    let road = std::env::var("EXAMPLE_ROAD_TEXTURE").unwrap_or_else(|_| ROAD_TEXTURE.to_owned());
    for file in files_for(&lamps, &road) {
        let path = out.join(file.path);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, file.bytes).unwrap();
    }
    let manifest = include_str!("../mod.toml");
    let content_only = manifest.split("[library]").next().unwrap();
    std::fs::write(out.join("mod.toml"), content_only).unwrap();
}

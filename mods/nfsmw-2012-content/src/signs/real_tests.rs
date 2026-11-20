//! The signs and traffic lights of the real 2012 install. Ignored without `MW2012_GAME_DIR` (the 2012 install).

use blackbox_modding::content::parse_replace_set;
use blackbox_solid::read_solids;
use blackbox_tpk::read_texture_packs;

use super::*;
use crate::source::Source;

fn install() -> Option<Source> {
    let root = std::env::var_os("MW2012_GAME_DIR")?;
    Source::open(Path::new(&root)).ok()
}

#[test]
#[ignore = "reads the 2012 install (MW2012_GAME_DIR)"]
fn every_model_of_the_table_is_read() {
    let Some(source) = install() else { return };
    let mut readers = Vec::new();
    let signs = read_signs(&source, &mut readers);
    assert_eq!(signs.len(), SIGNS.len(), "a sign could not be read");
    let (arm, head) = read_lights(&source, &mut readers).unwrap();
    assert!(arm.levels.len() >= 3 && !head.levels.is_empty());
}

#[test]
#[ignore = "reads the 2012 install (MW2012_GAME_DIR)"]
fn the_packs_read_back_and_every_swap_has_its_model_and_pictures() {
    let Some(source) = install() else { return };
    let mut readers = Vec::new();
    let signs = read_signs(&source, &mut readers);
    let (arm, head) = read_lights(&source, &mut readers).unwrap();
    let packs = [
        (build::signs(&signs, 1024).unwrap(), table::SIGN_SWAPS.len(), "signs"),
        (build::lights(&arm, &head, 1024).unwrap(), table::ARM_SWAPS.len() + table::HEAD_SWAPS.len(), "lights"),
    ];
    for (pack, swaps, id) in packs {
        assert_eq!(pack.swaps, swaps, "{id}");
        let solids = read_solids(&pack.geometry).unwrap();
        let hashes: Vec<u32> = read_texture_packs(&pack.textures)
            .unwrap()
            .into_iter()
            .flat_map(|texture_pack| texture_pack.textures)
            .map(|texture| texture.name_hash)
            .collect();
        for solid in &solids {
            assert!(solid.texture_hashes.iter().all(|hash| hashes.contains(hash)), "{}", solid.name);
            assert!(solid.groups.iter().all(|group| group.effect_id == 0), "{}", solid.name);
        }
        let set = parse_replace_set(id, &pack.manifest).unwrap();
        assert_eq!(set.models.len(), swaps);
        for entry in &set.models {
            assert!(solids.iter().any(|solid| solid.name == entry.model), "{}", entry.model);
        }
    }
}

#[test]
#[ignore = "reads the 2012 install (MW2012_GAME_DIR) and writes under MW2012_SIGNS_OUT"]
fn the_sets_are_written_where_asked_for_looking_at_them_in_the_game() {
    let Some(source) = install() else { return };
    let out = std::env::var_os("MW2012_SIGNS_OUT").expect("set MW2012_SIGNS_OUT to a folder");
    let started = std::time::Instant::now();
    write(&source, &Config::default(), Path::new(&out));
    println!("signs written in {:?}", started.elapsed());
    for file in ["signs-mw2012.toml", "lights-mw2012.toml"] {
        assert!(Path::new(&out).join("replace").join(file).is_file(), "{file}");
    }
}

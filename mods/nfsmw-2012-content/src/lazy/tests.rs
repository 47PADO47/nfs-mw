//! The keys, the recipe, the stub format the host reads, and the maker's refusals. No 2012 data.

use std::io;
use std::path::Path;
use std::sync::Arc;

use blackbox_modding::sdk::{Error, LazyFiles};
use game_install::{GameDir, LazySource};

use super::*;

fn recipe() -> Recipe {
    let cars = vec![
        CarRecipe {
            id: "POR911CARR12".into(),
            file: 1085007,
            liveries: vec![1085008, 1085009],
            idle_rpm: 1000.0,
            redline_rpm: 7800.0,
        },
        CarRecipe { id: "ALF4C12".into(), file: 1010001, liveries: Vec::new(), idle_rpm: 850.0, redline_rpm: 6900.5 },
    ];
    let config = Config { max_texture_size: 512, driver_poses: false, ..Config::default() };
    Recipe::new(&config, Path::new("Z:/an/install/that/is/not/there"), cars)
}

#[test]
fn every_key_a_run_writes_is_read_back() {
    for part in [Part::Geometry, Part::Textures] {
        assert_eq!(parse_key(&car_key("POR911CARR12", part)), Some(Key::Car("POR911CARR12".into(), part)));
    }
    for kind in [Sound::Engine, Sound::Shift, Sound::Whine, Sound::Blowoff, Sound::Backfire, Sound::Pops] {
        for at in [0, 7, 12] {
            assert_eq!(parse_key(&sound_key("ALF4C12", kind, at)), Some(Key::Sound("ALF4C12".into(), kind, at)));
        }
    }
}

#[test]
fn a_key_that_is_not_one_of_ours_is_refused() {
    for bad in [
        "",
        "car",
        "car/X",
        "car//GEOMETRY.BIN",
        "car/X/BOUNDS.BIN",
        "car/X/GEOMETRY.BIN/more",
        "sound/X/engine",
        "sound/X/horn/1",
        "sound/X/engine/one",
        "sound/X/engine/1/2",
        "plate/X/1",
    ] {
        assert_eq!(parse_key(bad), None, "{bad:?}");
    }
}

#[test]
fn the_recipe_is_written_and_read_back_whole_and_its_cars_are_sorted() {
    let made = recipe();
    assert_eq!(made.car.iter().map(|car| car.id.as_str()).collect::<Vec<_>>(), ["ALF4C12", "POR911CARR12"]);
    let read = Recipe::parse(&made.text()).unwrap();
    assert_eq!(read, made);
    assert_eq!((read.max_texture_size, read.driver_poses), (512, false));
    assert_eq!(read.car("por911carr12").unwrap().liveries, [1085008, 1085009]);
    assert_eq!(read.car("ALF4C12").unwrap().redline_rpm, 6900.5);
    assert!(read.car("NOCAR").is_none());
}

#[test]
fn a_path_with_backslashes_and_quotes_survives_the_recipe() {
    let mut made = recipe();
    made.source = r#"D:\Games\NFS "2012" 'MW'"#.into();
    assert_eq!(Recipe::parse(&made.text()).unwrap().source, made.source);
}

#[test]
fn a_recipe_of_another_format_or_text_that_is_not_one_is_an_error() {
    let text = recipe().text().replace("format = 1", "format = 2");
    assert!(Recipe::parse(&text).is_err());
    assert!(Recipe::parse("not a recipe").is_err());
}

#[test]
fn the_recipe_is_read_from_the_mod_folder_and_a_folder_without_one_has_none() {
    let dir = std::env::temp_dir().join(format!("nfsmw-2012-lazy-recipe-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    assert!(Recipe::read(&dir).is_none());
    recipe().write(&dir).unwrap();
    assert_eq!(Recipe::read(&dir), Some(recipe()));
    std::fs::remove_dir_all(&dir).unwrap();
}

/// A source that gives the key back as the file's bytes.
struct Echo;

impl LazySource for Echo {
    fn read(&self, key: &str) -> io::Result<Vec<u8>> {
        Ok(key.as_bytes().to_vec())
    }
}

#[test]
fn the_host_reads_the_stubs_this_mod_writes() {
    let root = std::env::temp_dir().join(format!("nfsmw-2012-lazy-stub-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(root.join("install/GLOBAL")).unwrap();
    std::fs::write(root.join("install/GLOBAL/X.BIN"), b"x").unwrap();
    std::fs::create_dir_all(root.join("mod/CARS/POR911CARR12")).unwrap();
    let key = car_key("POR911CARR12", Part::Geometry);
    std::fs::write(root.join("mod/CARS/POR911CARR12/GEOMETRY.BIN"), stub(&key)).unwrap();
    let mut dir = GameDir::open(root.join("install")).unwrap();
    dir.add_overlay(MOD_ID, root.join("mod"), 0).unwrap();
    dir.add_lazy_source(MOD_ID, Arc::new(Echo));
    assert_eq!(dir.read("CARS/POR911CARR12/GEOMETRY.BIN").unwrap(), key.as_bytes());
    std::fs::remove_dir_all(&root).unwrap();
}

#[test]
fn a_stub_is_small() {
    let longest = stub(&sound_key("A_CAR_ID_OF_32_CHARACTERS_LONG__", Sound::Backfire, 15));
    assert!(longest.len() < 128, "{} bytes", longest.len());
}

#[test]
fn the_maker_refuses_what_it_cannot_make_without_touching_the_install() {
    let maker = Maker::new(recipe());
    assert_eq!(maker.read("not a key"), Err(Error::Arg));
    assert_eq!(maker.read(&car_key("NOCAR", Part::Geometry)), Err(Error::NotFound), "a car the recipe lacks");
    assert_eq!(maker.read(&sound_key("NOCAR", Sound::Engine, 0)), Err(Error::NotFound));
    // A car the recipe has, over an install that is not there: the install cannot be opened.
    assert_eq!(maker.read(&car_key("ALF4C12", Part::Textures)), Err(Error::NotFound));
    assert_eq!(maker.read(&sound_key("ALF4C12", Sound::Shift, 0)), Err(Error::NotFound));
    assert!(maker.timings().iter().any(|(what, _)| what == "opening the 2012 install"), "the attempt is timed");
}

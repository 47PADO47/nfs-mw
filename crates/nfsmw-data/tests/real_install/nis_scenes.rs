//! The `AnimScene` chunk of every NIS scene bundle, read with `blackbox-nis`: one header per bundle, named after the bundle,
//! and the character records of the arrest scenes (`docs/specs/nis-scenes.md`, sections 1 to 4).

use std::collections::BTreeMap;

use blackbox_hash::bstring_hash;
use blackbox_nis::{Scene, SceneClock, SceneType, find_scene};

use crate::{install, unwrapped};

/// The scene name of a bundle file name: the text between the first and second underscore (`Scene_<name>_BundleB.bun`).
fn scene_name(file: &str) -> Option<&str> {
    let mut parts = file.splitn(3, '_');
    parts.next()?;
    parts.next()
}

#[test]
#[ignore = "needs the game (set NFSMW_GAME_DIR)"]
fn the_nis_scene_chunks_read_their_headers_and_characters() {
    let Some(dir) = install() else { return };
    let mut scenes: Vec<(String, Scene)> = Vec::new();
    for file in dir.files_in("NIS") {
        let lower = file.to_ascii_lowercase();
        if !(lower.starts_with("scene_") && lower.ends_with("_bundleb.bun")) {
            continue;
        }
        let name = scene_name(&file).unwrap_or_else(|| panic!("{file}: no scene name in the file name"));
        let data = unwrapped(&dir, &format!("NIS/{file}"));
        let scene = find_scene(&data)
            .unwrap_or_else(|e| panic!("{file}: {e}"))
            .unwrap_or_else(|| panic!("{file}: no AnimScene chunk"));
        assert_eq!(scene.name_hash, bstring_hash(name), "{file}: the header hash is not the name's hash");
        assert_eq!(scene.name, name, "{file}: the header name");
        scenes.push((name.to_string(), scene));
    }
    assert_eq!(scenes.len(), 87, "NIS scene bundles");

    let mut types: BTreeMap<i32, usize> = BTreeMap::new();
    for (_, scene) in &scenes {
        *types.entry(scene.scene_type as i32).or_default() += 1;
        assert_eq!(scene.start_frame, 0, "{}: StartFrame", scene.name);
    }
    assert_eq!(types, BTreeMap::from([(0, 25), (2, 26), (3, 4), (4, 32)]), "SceneType counts");

    let flybys = scenes.iter().filter(|(_, scene)| scene.overlay_name == "FLYBY").count();
    assert_eq!(flybys, 12, "scenes with the FLYBY overlay");
    let without_cars = scenes.iter().filter(|(_, scene)| !scene.have_car_animation).count();
    assert_eq!(without_cars, 2, "scenes without car animation");

    let characters: usize = scenes.iter().map(|(_, scene)| scene.characters.len()).sum();
    assert_eq!(characters, 64, "character records in the NIS bundles");
    let with_characters: Vec<&(String, Scene)> =
        scenes.iter().filter(|(_, scene)| !scene.characters.is_empty()).collect();
    assert_eq!(with_characters.len(), 26, "scenes with character records");
    for (name, scene) in with_characters {
        assert!(name.starts_with("Arrest"), "{name}: character records outside the arrest scenes");
        assert_eq!(scene.scene_type, SceneType::Type2, "{name}: SceneType of an arrest");
        assert!(scene.have_layout, "{name}: HaveLayout of an arrest");
        for character in &scene.characters {
            assert_eq!(character.type_id, 1, "{name}: entity type of a basic character");
        }
    }

    for wanted in ["IntroNis05", "IntroNisDD", "EndingNis04", "ArrestF02"] {
        let (_, scene) =
            scenes.iter().find(|(name, _)| name == wanted).unwrap_or_else(|| panic!("{wanted} is missing"));
        let clock = SceneClock::from_scene(scene, 30.0).expect("30 fps is a valid rate");
        eprintln!(
            "{wanted}: name hash {:#010x}, type {:?}, layout {}, car animation {}, {} cars, frames {}..{} ({:.2} s), \
             overlay {:?}, {} characters, description {:?}",
            scene.name_hash,
            scene.scene_type,
            scene.have_layout,
            scene.have_car_animation,
            scene.number_of_cars,
            scene.start_frame,
            scene.vanish_frame,
            clock.duration(),
            scene.overlay_name,
            scene.characters.len(),
            scene.description,
        );
        for character in &scene.characters {
            eprintln!(
                "  character {:#010x} at {:?}, skeleton {:#010x}, clip {:#010x}, play speed {}",
                character.instance_name_hash,
                character.local_matrix.w_axis.truncate().to_array(),
                character.skeleton_name_hash,
                character.skeletal_anim_hash,
                character.play_speed,
            );
        }
    }
}

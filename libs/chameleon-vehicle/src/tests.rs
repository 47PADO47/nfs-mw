//! Every car of a real install loads (set `CHAMELEON_GAME_DIR` to the install folder and run with `--ignored`).

use std::path::Path;

use chameleon_bundle::{Bundle, Pool};

use crate::{find_spec, load, vehicle_name};

#[test]
#[ignore = "reads the game install (CHAMELEON_GAME_DIR)"]
fn every_high_detail_car_loads() {
    let Some(dir) = std::env::var_os("CHAMELEON_GAME_DIR") else { return };
    let dir = Path::new(&dir);
    let textures = std::fs::read(dir.join("VEHICLES/VEHICLETEX.BNDL")).unwrap();
    let shaders = std::fs::read(dir.join("SHADERS.BNDL")).unwrap();
    let mut loaded = 0;
    let mut names = 0;
    for entry in std::fs::read_dir(dir.join("VEHICLES")).unwrap().flatten() {
        let path = entry.path();
        let file = path.file_name().unwrap().to_string_lossy().to_uppercase();
        if !(file.starts_with("VEH_") && file.ends_with("_HI.BNDL")) {
            continue;
        }
        let data = std::fs::read(&path).unwrap();
        let bundle = Bundle::parse(&data).unwrap();
        let spec = find_spec(&bundle).unwrap_or_else(|| panic!("{file}: no spec"));
        let mut pool = Pool::new();
        pool.add(bundle.clone());
        pool.add(Bundle::parse(&textures).unwrap());
        pool.add(Bundle::parse(&shaders).unwrap());
        let vehicle = load(&pool, spec).unwrap_or_else(|error| panic!("{file}: {error}"));
        assert_eq!(vehicle.wheels.len(), 4, "{file}");
        assert!(vehicle.body.iter().all(|lod| !lod.is_empty()) && !vehicle.body.is_empty(), "{file}");
        assert!(vehicle.wheels.iter().all(|wheel| wheel.lods.first().is_some_and(|lod| !lod.is_empty())), "{file}");
        names += usize::from(vehicle_name(&bundle).is_some());
        loaded += 1;
    }
    assert!(loaded > 50, "only {loaded} cars found");
    println!("{loaded} cars loaded, {names} with a name");
}

#[test]
#[ignore = "reads the game install (CHAMELEON_GAME_DIR)"]
fn every_high_detail_car_has_paints() {
    let Some(dir) = std::env::var_os("CHAMELEON_GAME_DIR") else { return };
    let mut total = 0;
    for entry in std::fs::read_dir(Path::new(&dir).join("VEHICLES")).unwrap().flatten() {
        let file = entry.file_name().to_string_lossy().to_uppercase();
        if !(file.starts_with("VEH_") && file.ends_with("_HI.BNDL")) {
            continue;
        }
        let data = std::fs::read(entry.path()).unwrap();
        let found = crate::paints(&Bundle::parse(&data).unwrap());
        assert!(!found.is_empty(), "{file}: no paint");
        total += found.len();
    }
    assert_eq!(total, 443, "the 93 cars of the checked install hold 443 paint records");
}

#[test]
#[ignore = "reads the game install (CHAMELEON_GAME_DIR)"]
fn the_veyrons_mclaren_and_venom_have_a_wing_machine() {
    let Some(dir) = std::env::var_os("CHAMELEON_GAME_DIR") else { return };
    let mut found = Vec::new();
    for entry in std::fs::read_dir(Path::new(&dir).join("VEHICLES")).unwrap().flatten() {
        let file = entry.file_name().to_string_lossy().to_uppercase();
        if !(file.starts_with("VEH_") && file.ends_with("_HI.BNDL")) {
            continue;
        }
        let data = std::fs::read(entry.path()).unwrap();
        let bundle = Bundle::parse(&data).unwrap();
        let Some(skeleton) = crate::Skeleton::read(&bundle) else { continue };
        let Some(machine) = crate::wing_machine(&bundle, &skeleton) else { continue };
        let bones: std::collections::BTreeSet<usize> = machine.tracks.iter().map(|t| t.bone).collect();
        println!(
            "{file} {:?} {:?} bones {bones:?} tracks {}",
            vehicle_name(&bundle),
            machine.seconds,
            machine.tracks.len()
        );
        found.push(file);
    }
    assert_eq!(found.len(), 4, "{found:?}");
}

#[test]
#[ignore = "reads the game install (CHAMELEON_GAME_DIR)"]
fn the_cars_with_an_active_spoiler_are_found() {
    let Some(dir) = std::env::var_os("CHAMELEON_GAME_DIR") else { return };
    let mut found = Vec::new();
    for entry in std::fs::read_dir(Path::new(&dir).join("VEHICLES")).unwrap().flatten() {
        let file = entry.file_name().to_string_lossy().to_uppercase();
        if !(file.starts_with("VEH_") && file.ends_with("_HI.BNDL")) {
            continue;
        }
        let data = std::fs::read(entry.path()).unwrap();
        let bundle = Bundle::parse(&data).unwrap();
        let Some(skeleton) = crate::Skeleton::read(&bundle) else { continue };
        let Some(motion) = crate::spoiler_motion(&bundle, &skeleton) else { continue };
        println!("{file} {:?} {motion:?} {:?}", vehicle_name(&bundle), motion.rotation_axis_angle());
        found.push((file, motion.above_mph, motion.below_mph));
    }
    // The single-state speed machines of the install: the speeds come from their conditions.
    assert_eq!(found.len(), 11, "{found:?}");
    let speeds: std::collections::BTreeSet<(i32, i32)> =
        found.iter().map(|(_, above, below)| (*above as i32, *below as i32)).collect();
    let expected = [(70, 0), (70, 40), (70, 50), (74, 64), (75, 75), (80, 50), (100, 80)];
    assert_eq!(speeds, expected.into_iter().collect(), "{found:?}");
}

#[test]
#[ignore = "reads the game install (CHAMELEON_GAME_DIR)"]
fn the_air_vents_have_their_delays() {
    let Some(dir) = std::env::var_os("CHAMELEON_GAME_DIR") else { return };
    let mut found = Vec::new();
    for entry in std::fs::read_dir(Path::new(&dir).join("VEHICLES")).unwrap().flatten() {
        let file = entry.file_name().to_string_lossy().to_uppercase();
        if !(file.starts_with("VEH_") && file.ends_with("_HI.BNDL")) {
            continue;
        }
        let data = std::fs::read(entry.path()).unwrap();
        let bundle = Bundle::parse(&data).unwrap();
        let Some(skeleton) = crate::Skeleton::read(&bundle) else { continue };
        for vent in crate::vent_motions(&bundle, &skeleton) {
            println!("{file} {:?} {:?}", vehicle_name(&bundle), vent);
            found.push((
                vent.motion.above_mph,
                vent.open_seconds,
                vent.motion.below_mph,
                vent.close_seconds,
                1 + vent.attached.len(),
            ));
        }
    }
    found.sort_by(|a, b| a.partial_cmp(b).unwrap());
    // 918 Concept 75/3 s and 75/5 s; Marussia 120/3 s and 100/5 s; LP700-4 and Aventador J 100/3 s and 100/5 s.
    let wanted: Vec<(f32, f32, f32, f32)> =
        vec![(75.0, 3.0, 75.0, 5.0), (100.0, 3.0, 100.0, 5.0), (100.0, 3.0, 100.0, 5.0), (120.0, 3.0, 100.0, 5.0)];
    let numbers: Vec<_> = found.iter().map(|f| (f.0, f.1, f.2, f.3)).collect();
    assert_eq!(numbers, wanted, "{found:?}");
}

#[test]
#[ignore = "reads the game install (CHAMELEON_GAME_DIR)"]
fn the_huayra_flaps_are_guarded_animations() {
    let Some(dir) = std::env::var_os("CHAMELEON_GAME_DIR") else { return };
    let data = std::fs::read(Path::new(&dir).join("VEHICLES/VEH_535435_HI.BNDL")).unwrap();
    let bundle = Bundle::parse(&data).unwrap();
    for (i, e) in bundle.entries().iter().enumerate() {
        if e.kind != 0x501 {
            continue;
        }
        let machine = bundle.resource(i).unwrap();
        for drive in crate::hsm::drives(&machine.blocks[0]) {
            let tests = |g: &crate::hsm::Guard| {
                format!(
                    "{}{:?}",
                    if g.any { "ANY" } else { "ALL" },
                    g.tests.iter().map(|t| format!("{} {} {}", t.input, t.operator, t.value)).collect::<Vec<_>>()
                )
            };
            println!(
                "DRIVE {} anim {} open {} hold {} close {}",
                drive.name,
                drive.animation,
                tests(&drive.open),
                drive.open_hold,
                tests(&drive.close)
            );
        }
    }
}

//! The driver's setup, figure and placement. The real-install test needs `CHAMELEON_GAME_DIR` (or `MW2012_GAME_DIR`) and
//! `--ignored`.

use std::path::{Path, PathBuf};

use chameleon_bundle::{Bundle, Pool};
use chameleon_gfx::Vertex;

use crate::driver::{DriverSetup, SETUP};
use crate::genesys::Object;
use crate::{Appearance, Driver, Lod, PartMesh, SurfaceKind, Vehicle};
use crate::{driver_meshes, driver_setup, is_driver, load, load_driver, load_texture, remove_baked_driver};

fn mesh(technique: &str, points: &[[f32; 3]]) -> PartMesh {
    PartMesh {
        vertices: points.iter().map(|&position| Vertex { position, color: [255; 4], uv: [0.25; 2] }).collect(),
        triangles: vec![[0, 1, 2]],
        appearance: Appearance {
            kind: SurfaceKind::Body,
            technique: technique.to_owned(),
            diffuse_texture: Some(7),
            diffuse_colour: None,
            alpha_test: false,
        },
    }
}

/// A setup object as the files lay it out: the seat at 0x10, the character at 0x24, the belt count at 0x42.
fn object(seat: [f32; 3], character: u32, belts: u16) -> Object {
    let mut main = vec![0u8; 0x130];
    for (n, value) in seat.iter().enumerate() {
        main[0x10 + 4 * n..0x14 + 4 * n].copy_from_slice(&value.to_le_bytes());
    }
    main[0x24..0x28].copy_from_slice(&character.to_le_bytes());
    main[0x42..0x44].copy_from_slice(&belts.to_le_bytes());
    main[0x2C..0x30].copy_from_slice(&0x1779e3u32.to_le_bytes());
    main[0x30..0x34].copy_from_slice(&0x1779e2u32.to_le_bytes());
    main[0x50..0x54].copy_from_slice(&0x1779c7u32.to_le_bytes());
    Object { main, structs: vec![(SETUP.to_owned(), 0)] }
}

#[test]
fn the_setup_is_read_by_its_offsets() {
    let setup = DriverSetup::of(&object([0.29, -0.739, -0.222], 0x17785D, 4)).unwrap();
    let expected = DriverSetup {
        character: 0x17785D,
        seat: Some([0.29, -0.739, -0.222]),
        belts: 4,
        shifter_group: 0x1779e3,
        grip_group: 0x1779e2,
        pose_clip: 0x1779c7,
    };
    assert_eq!(setup, expected);
}

#[test]
fn a_seat_of_19_metres_is_no_seat() {
    let setup = DriverSetup::of(&object([19.0; 3], 0x3D8E, 3)).unwrap();
    assert_eq!(setup.seat, None);
    assert_eq!(setup.character, 0x3D8E);
    assert_eq!(DriverSetup::of(&object([f32::NAN, 0.0, 0.0], 0x3D8E, 3)).unwrap().seat, None);
}

#[test]
fn an_object_without_the_setup_or_cut_short_is_refused() {
    let other = Object { main: vec![0; 0x130], structs: vec![("Genesys.Gen.Other".to_owned(), 0)] };
    assert!(DriverSetup::of(&other).is_err());
    let short = Object { main: vec![0; 0x30], structs: vec![(SETUP.to_owned(), 0)] };
    assert!(DriverSetup::of(&short).is_err());
}

fn figure() -> Driver {
    let near = vec![mesh("Character_Opaque", &[[0.0, 0.8, -0.4], [0.2, 1.8, 0.1], [-0.2, 1.0, 1.0]])];
    let far = vec![mesh("Character_Opaque", &[[0.0, 0.8, -0.4], [0.2, 1.8, 0.1], [-0.2, 1.0, 1.0]])];
    Driver { character: 0x3D8E, lods: vec![near, far] }
}

#[test]
fn the_seat_moves_every_level_and_nothing_else() {
    let figure = figure();
    let seated = figure.seated([0.4, -0.6, 0.1]);
    assert_eq!(seated.len(), 2);
    for lod in &seated {
        let moved = lod[0].vertices[1].position;
        assert!(moved.iter().zip([0.6, 1.2, 0.2]).all(|(a, b)| (a - b).abs() < 1e-6), "{moved:?}");
        assert_eq!(lod[0].vertices[1].uv, [0.25; 2]);
        assert_eq!(lod[0].triangles, [[0, 1, 2]]);
    }
    assert_eq!(figure.lods[0][0].vertices[1].position, [0.2, 1.8, 0.1], "the figure itself is not moved");
}

#[test]
fn the_figures_textures_are_listed_once() {
    let mut figure = figure();
    figure.lods[1][0].appearance.diffuse_texture = Some(9);
    figure.lods[0].push(mesh("Character_Opaque", &[[0.0; 3]; 3]));
    assert_eq!(figure.textures(), [7, 9]);
}

#[test]
fn the_baked_driver_is_told_by_its_technique() {
    assert!(is_driver(&mesh("VehicleNFS13_Body_Driver", &[])));
    assert!(!is_driver(&mesh("VehicleNFS13_Body", &[])));
    let lod = vec![mesh("A_Body", &[]), mesh("A_Body_Driver", &[]), mesh("A_Glass", &[]), mesh("B_Driver", &[])];
    assert_eq!(driver_meshes(&lod), [1, 3]);
}

#[test]
fn the_baked_driver_leaves_the_body_at_every_level() {
    let level = || vec![mesh("A_Body", &[[0.0; 3]; 3]), mesh("A_Body_Driver", &[[0.0; 3]; 3])];
    let mut vehicle = Vehicle { body: vec![level(), level()], wheels: Vec::new() };
    assert_eq!(remove_baked_driver(&mut vehicle), 1);
    assert!(vehicle.body.iter().all(|lod| lod.len() == 1 && !is_driver(&lod[0])));
    assert_eq!(remove_baked_driver(&mut vehicle), 0);
}

fn install() -> Option<PathBuf> {
    let dir = std::env::var_os("CHAMELEON_GAME_DIR").or_else(|| std::env::var_os("MW2012_GAME_DIR"))?;
    Some(PathBuf::from(dir))
}

fn read(dir: &Path, rel: &str) -> Vec<u8> {
    std::fs::read(dir.join(rel)).unwrap_or_else(|error| panic!("{rel}: {error}"))
}

fn extent(lod: &Lod) -> ([f32; 3], [f32; 3]) {
    let (mut low, mut high) = ([f32::MAX; 3], [f32::MIN; 3]);
    for vertex in lod.iter().flat_map(|mesh| mesh.vertices.iter()) {
        for axis in 0..3 {
            low[axis] = low[axis].min(vertex.position[axis]);
            high[axis] = high[axis].max(vertex.position[axis]);
        }
    }
    (low, high)
}

/// Every one of the 93 cars of the install sets a driver up; the two characters read with their textures; seated at the
/// car's seat the figure is inside the car's body (head under the highest point of the body, soles on the floor), and
/// the one figure a body bakes in (the Explorer's) is where the seated character is.
#[test]
#[ignore = "reads the game install (CHAMELEON_GAME_DIR or MW2012_GAME_DIR)"]
fn every_car_seats_a_character_inside_its_body() {
    let Some(dir) = install() else { return };
    let textures = read(&dir, "VEHICLES/VEHICLETEX.BNDL");
    let shaders = read(&dir, "SHADERS.BNDL");
    let characters = read(&dir, "CHARACTERS/ALL_CHARS.BNDL");
    let mut character_pool = Pool::new();
    character_pool.add(Bundle::parse(&characters).unwrap());
    character_pool.add(Bundle::parse(&shaders).unwrap());
    let mut figures = std::collections::HashMap::new();
    let (mut cars, mut without, mut baked) = (0, Vec::new(), Vec::new());
    for entry in std::fs::read_dir(dir.join("VEHICLES")).unwrap().flatten() {
        let file = entry.file_name().to_string_lossy().to_uppercase();
        if !(file.starts_with("VEH_") && file.ends_with("_HI.BNDL")) {
            continue;
        }
        let data = std::fs::read(entry.path()).unwrap();
        let bundle = Bundle::parse(&data).unwrap();
        let setup = driver_setup(&bundle).unwrap_or_else(|| panic!("{file}: no driver setup"));
        cars += 1;
        // The harness (4 belt bones) goes with the second character, the belt (3) with the first.
        assert_eq!(setup.belts == 4, setup.character == 0x17785D, "{file}: {setup:?}");
        let Some(seat) = setup.seat else {
            without.push(file);
            continue;
        };
        let figure = figures.entry(setup.character).or_insert_with(|| {
            let figure = load_driver(&character_pool, setup.character).unwrap();
            assert_eq!(figure.lods.len(), 5, "five levels of detail");
            for id in figure.textures() {
                let texture = load_texture(&character_pool, id).unwrap();
                assert!(texture.width >= 1024, "{id:X}: {}x{}", texture.width, texture.height);
            }
            for mesh in figure.lods.iter().flatten() {
                assert!(mesh.appearance.diffuse_texture.is_some(), "every mesh of the figure has a diffuse texture");
            }
            figure
        });
        let mut pool = Pool::new();
        pool.add(bundle.clone());
        pool.add(Bundle::parse(&textures).unwrap());
        pool.add(Bundle::parse(&shaders).unwrap());
        let mut vehicle = load(&pool, crate::find_spec(&bundle).unwrap()).unwrap();
        let bakes = vehicle.body[0].iter().any(is_driver);
        let baked_at = vehicle.body[0].iter().filter(|mesh| is_driver(mesh)).cloned().collect::<Lod>();
        assert_eq!(remove_baked_driver(&mut vehicle) > 0, bakes, "{file}");
        let (low, high) = extent(&figure.seated(seat)[0]);
        let (body_low, body_high) = extent(&vehicle.body[0]);
        let (head, soles) = (high[1], low[1]);
        assert!(head <= body_high[1] + 0.05, "{file}: the head ({head}) is above the whole body ({})", body_high[1]);
        assert!(soles >= -0.05, "{file}: the soles ({soles}) are under the ground");
        for axis in [0, 2] {
            let outside = low[axis] < body_low[axis] - 0.05 || high[axis] > body_high[axis] + 0.05;
            assert!(!outside, "{file}: the driver pokes out of the body along axis {axis}");
        }
        if bakes {
            let (baked_low, baked_high) = extent(&baked_at);
            for axis in 0..3 {
                assert!((low[axis] - baked_low[axis]).abs() < 0.2, "{file}: axis {axis} low");
                assert!((high[axis] - baked_high[axis]).abs() < 0.2, "{file}: axis {axis} high");
            }
            baked.push(file);
        }
    }
    assert_eq!(cars, 93);
    assert_eq!(figures.len(), 2, "two characters sit in the cars");
    assert_eq!(without.len(), 1, "one car (the BMW 1M, with Lightning McQueen's body) has no driver: {without:?}");
    assert_eq!(baked.len(), 1, "one car bakes its driver into the body: {baked:?}");
    println!("{cars} cars; no driver: {without:?}; baked: {baked:?}");
}

use blackbox_attrib::vlt_hash;
use blackbox_collision::{BoundsSet, FitOptions, Shape, bounds_flags, fit_body, write_bounds_pack};

use super::*;
use crate::car::physics::DAMAGE_ZONES;

/// A box-shaped car, 1.9 m wide, 1.3 m tall, 4.6 m long, with a narrower cabin, in vehicle axes.
fn points() -> Vec<[f32; 3]> {
    let mut points = Vec::new();
    let grid = |lo: f32, hi: f32, n: usize| (0..=n).map(move |i| lo + (hi - lo) * i as f32 / n as f32);
    for x in grid(-0.95, 0.95, 4) {
        for y in grid(0.0, 0.9, 3) {
            for z in grid(-2.3, 2.3, 10) {
                points.push([x, y, z]);
            }
        }
    }
    for x in grid(-0.65, 0.65, 3) {
        for y in grid(0.9, 1.3, 2) {
            for z in grid(-1.4, 0.6, 6) {
                points.push([x, y, z]);
            }
        }
    }
    points
}

fn set(folder: &str) -> BoundsSet {
    fit_body(vlt_hash(&folder.to_ascii_uppercase()), &points(), &FitOptions::default()).unwrap()
}

/// A scratch install whose `CARS/<folder>/BOUNDS.BIN` is `bytes`.
fn install(name: &str, folder: &str, bytes: Option<Vec<u8>>) -> (std::path::PathBuf, GameDir) {
    let root = std::env::temp_dir().join(format!("nfsmw-data-modcar-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(root.join("install/CARS").join(folder)).unwrap();
    if let Some(bytes) = bytes {
        std::fs::write(root.join("install/CARS").join(folder).join("BOUNDS.BIN"), bytes).unwrap();
    }
    let dir = GameDir::open(root.join("install")).unwrap();
    (root, dir)
}

#[test]
fn a_mod_car_has_the_bounds_of_its_own_file() {
    let own = set("TESTCAR");
    let (root, dir) = install("own", "TESTCAR", Some(write_bounds_pack(std::slice::from_ref(&own)).unwrap()));
    let (bounds, read) = mod_car_bounds(&dir, "TESTCAR").unwrap();
    assert_eq!(bounds.half_dimensions.to_array(), own.root().half_dimensions);
    assert_eq!(bounds.pivot.to_array(), own.root().pivot);
    // What the file stores is the set at millimetre and 1/32767 resolution.
    assert_eq!(read, BoundsSet::parse(&own.to_payload().unwrap()).unwrap());
    // Matched whatever the case of the folder, like every install path.
    assert_eq!(mod_car_bounds(&dir, "testcar").unwrap().0, bounds);
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn the_collision_data_is_the_boxes_spheres_and_ground_mesh_of_the_file() {
    let (root, dir) = install("data", "TESTCAR", Some(write_bounds_pack(&[set("TESTCAR")]).unwrap()));
    let car = mod_car_collision(&dir, "TESTCAR").unwrap();
    assert!(car.bounds.half_dimensions.x > 0.9 && car.bounds.half_dimensions.z > 2.2);
    let body = &car.geometry.body;
    assert!(!body.used_fallback);
    assert_eq!(body.primitives.iter().filter(|p| p.shape == Shape::Box).count(), 2);
    assert_eq!(body.primitives.iter().filter(|p| p.shape == Shape::Sphere).count(), 6);
    assert_eq!(body.ground_meshes.len(), 1);
    assert!(body.primitives.iter().all(|p| p.collides(bounds_flags::PRIM_VS_WORLD)));
    assert_eq!(car.events.visual_scale.len(), DAMAGE_ZONES.len());
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn a_car_without_a_bounds_file_is_an_error_that_names_it() {
    let (root, dir) = install("missing", "BMWM3GTR", None);
    let error = format!("{:#}", mod_car_bounds(&dir, "BMWM3GTR").unwrap_err());
    assert!(error.contains("BMWM3GTR has no collision bounds"), "{error}");
    assert!(mod_car_collision(&dir, "BMWM3GTR").is_err());
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn a_file_with_one_set_is_accepted_under_any_key_but_two_foreign_sets_are_not() {
    let other = set("SOMEONEELSE");
    let (root, dir) = install("keys", "TESTCAR", Some(write_bounds_pack(std::slice::from_ref(&other)).unwrap()));
    assert!(mod_car_bounds(&dir, "TESTCAR").is_ok());
    std::fs::remove_dir_all(root).unwrap();
    let bytes = write_bounds_pack(&[set("A"), set("B")]).unwrap();
    let (root, dir) = install("keys2", "TESTCAR", Some(bytes));
    let error = format!("{:#}", mod_car_bounds(&dir, "TESTCAR").unwrap_err());
    assert!(error.contains("none is keyed TESTCAR"), "{error}");
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn a_file_that_is_not_a_vehicle_body_is_refused_with_the_reason() {
    let mut round = set("TESTCAR");
    round.nodes[0].flags = bounds_flags::SPHERE;
    let (root, dir) = install("sphere", "TESTCAR", Some(write_bounds_pack(&[round]).unwrap()));
    let error = format!("{:#}", mod_car_bounds(&dir, "TESTCAR").unwrap_err());
    assert!(error.contains("CARS/TESTCAR/BOUNDS.BIN") && error.contains("not a box"), "{error}");
    std::fs::remove_dir_all(root).unwrap();
    let (root, dir) = install("garbage", "TESTCAR", Some(vec![0xAB; 64]));
    assert!(mod_car_bounds(&dir, "TESTCAR").is_err());
    std::fs::remove_dir_all(root).unwrap();
    let mut huge = set("TESTCAR");
    huge.nodes[0].half_dimensions[2] = 30.0;
    let (root, dir) = install("huge", "TESTCAR", Some(write_bounds_pack(&[huge]).unwrap()));
    assert!(mod_car_bounds(&dir, "TESTCAR").is_err());
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn the_racing_damage_numbers_are_the_shared_table_not_a_cars_own() {
    let spec = CarEventSpec::racing_defaults();
    assert_eq!(spec.strength, 20_000.0);
    assert_eq!(spec.visual_scale, [0.8, 1.5, 3.5, 3.5, 3.5, 3.5, 7.5, 7.5, 3.5, 0.7]);
    assert_eq!((spec.hit_points, spec.hit_point_threshold, spec.suppress_distance), (0.0, 0.0, 0.0));
    assert_eq!(spec.hit_point_scale, [0.0; 10]);
    assert!(spec.sequence.is_none());
    assert_eq!((spec.shock_force, spec.shock_time), (40.0, 3.0));
}

/// The numbers above are the `racers` damage record of the install, which every stock player car links to.
#[test]
#[ignore = "needs the game (set NFSMW_GAME_DIR)"]
fn the_racing_defaults_equal_the_installed_damage_record_of_the_player_cars() {
    let Ok(root) = std::env::var("NFSMW_GAME_DIR") else { return };
    let dir = GameDir::open(root).unwrap();
    let db = crate::shared::attributes(&dir).unwrap();
    let catalog = crate::world::sequences::SequenceCatalog::default();
    let defaults = CarEventSpec::racing_defaults();
    let mut checked = 0;
    for car in ["bmwm3gtr", "corvettec6r", "lamborghini", "porsche911", "mustanggt", "mercedesslr", "supra"] {
        let Some(vehicle) = db.collection("pvehicle", car) else { continue };
        let named = vehicle.follow("damagespecs").and_then(|d| d.name().map(str::to_owned));
        assert_eq!(named.as_deref(), Some("racers"), "{car}");
        let spec = CarEventSpec::read(vehicle, &catalog).unwrap();
        assert_eq!(spec.strength, defaults.strength, "{car}");
        assert_eq!(spec.visual_scale, defaults.visual_scale, "{car}");
        assert_eq!((spec.hit_points, spec.hit_point_threshold, spec.suppress_distance), (0.0, 0.0, 0.0), "{car}");
        assert_eq!(spec.hit_point_scale, defaults.hit_point_scale, "{car}");
        assert_eq!((spec.shock_force, spec.shock_time), (defaults.shock_force, defaults.shock_time), "{car}");
        checked += 1;
    }
    assert!(checked >= 2, "only {checked} player cars found");
}

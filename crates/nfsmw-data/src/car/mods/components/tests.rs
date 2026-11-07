use std::collections::BTreeMap;
use std::sync::Arc;

use blackbox_components::{Catalog, VehicleDef};
use blackbox_solid::{ShadingGroup, Solid, Vertex};
use blackbox_vehicle::bike::BikeSpec;
use glam::Vec3;

use super::parts::{affine, mounted};
use super::select::{category_of, choice_hash, performance_of, picks, resolve};
use super::{Binding, Components, Role, catalog, is_choice, is_choice_of, keys, of, publish, upgraded};
use crate::car::customization::{Category, Selection};
use crate::car::mods::CarStats;
use crate::car::physics::{PerformanceCategory, PerformancePackage};

const CATALOG: &str = r#"
[[component]]
id = "t:exhaust.stock"
effects = [{ kind = "part", show = ["exhaust.stock"] }]
[[component]]
id = "t:exhaust.race"
effects = [{ kind = "part", show = ["exhaust.race"] }]
[[component]]
id = "t:engine.stock"
[[component]]
id = "t:engine.1"
tier = 1
effects = [
  { kind = "param", key = "bike.engine.peak_torque_nm", op = "mul", value = 1.05 },
  { kind = "param", key = "bike.chassis.mass_kg", op = "mul", value = 0.98 },
]
[[component]]
id = "t:engine.2"
tier = 2
effects = [{ kind = "param", key = "bike.engine.peak_torque_nm", op = "mul", value = 1.1 }]
"#;

const VEHICLE: &str = r#"
id = "testbike"
[[slot]]
id = "exhaust"
accepts = ["t:exhaust.stock", "t:exhaust.race"]
default = "t:exhaust.stock"
[[slot]]
id = "perf_engine"
accepts = ["t:engine.stock", "t:engine.1", "t:engine.2"]
default = "t:engine.stock"
"#;

fn binding(solid: &str) -> Binding {
    Binding { solid: solid.to_owned(), pack: None, role: Role::Body, matrix: None, base: false }
}

fn components() -> Components {
    let catalog: Catalog = toml::from_str(CATALOG).unwrap();
    let vehicle: VehicleDef = toml::from_str(VEHICLE).unwrap();
    vehicle.check(&catalog).expect("the fixture is valid");
    let parts = BTreeMap::from([
        ("exhaust.stock".to_owned(), binding("TESTBIKE_KIT00_EXHAUST")),
        ("exhaust.race".to_owned(), binding("MUF_001")),
    ]);
    Components { vehicle, catalog: Arc::new(catalog), parts }
}

#[test]
fn slots_map_onto_the_garages_categories_and_levels() {
    assert_eq!(category_of("exhaust"), Some(Category::Spoiler));
    assert_eq!(category_of("rims"), Some(Category::Wheels));
    assert_eq!(category_of("perf_engine"), None);
    assert_eq!(performance_of("perf_engine"), Some(PerformanceCategory::Engine));
    assert_eq!(performance_of("exhaust"), None);
}

#[test]
fn a_choice_hash_is_the_cars_and_ignores_case() {
    assert_eq!(choice_hash("Testbike", "t:exhaust.race"), choice_hash("TESTBIKE", "T:EXHAUST.RACE"));
    assert_ne!(choice_hash("TESTBIKE", "t:exhaust.race"), choice_hash("OTHERBIKE", "t:exhaust.race"));
}

#[test]
fn the_stock_car_picks_nothing_and_resolves_to_its_stock_parts() {
    let components = components();
    let stock = picks("TESTBIKE", &components, &Selection::default(), PerformancePackage::default());
    assert!(stock.is_empty());
    let resolved = resolve("TESTBIKE", &components, &Selection::default(), PerformancePackage::default()).unwrap();
    assert!(resolved.parts.contains("exhaust.stock") && !resolved.parts.contains("exhaust.race"));
}

#[test]
fn a_garage_choice_installs_its_component_and_swaps_the_part() {
    let components = components();
    let mut selection = Selection::default();
    selection.parts[Category::Spoiler as usize] = Some(choice_hash("TESTBIKE", "t:exhaust.race"));
    let resolved = resolve("TESTBIKE", &components, &selection, PerformancePackage::default()).unwrap();
    assert!(resolved.parts.contains("exhaust.race") && !resolved.parts.contains("exhaust.stock"));
    // A hash the car does not list is ignored: the stock part stays.
    selection.parts[Category::Spoiler as usize] = Some(1234);
    let resolved = resolve("TESTBIKE", &components, &selection, PerformancePackage::default()).unwrap();
    assert!(resolved.parts.contains("exhaust.stock"));
}

#[test]
fn a_performance_level_installs_the_tier_and_a_high_level_the_highest() {
    let components = components();
    let package = |level| {
        let mut package = PerformancePackage::default();
        package.levels[PerformanceCategory::Engine as usize] = level;
        package
    };
    for (level, mass) in [(0, 1.0), (1, 0.98), (2, 1.0), (3, 1.0)] {
        let resolved = resolve("TESTBIKE", &components, &Selection::default(), package(level)).unwrap();
        assert_eq!(resolved.params.apply(keys::MASS, 1.0), mass, "level {level}");
    }
    let resolved = resolve("TESTBIKE", &components, &Selection::default(), package(3)).unwrap();
    assert_eq!(resolved.installed["perf_engine"].component, "t:engine.2");
    assert_eq!(components.max_tier("perf_engine"), 2);
    assert_eq!(components.max_tier("exhaust"), 0);
    assert_eq!(components.max_tier("nothing"), 0);
}

#[test]
fn the_modifiers_change_the_numbers_of_the_bike_and_nothing_else() {
    let components = components();
    let spec = BikeSpec::example();
    let mut package = PerformancePackage::default();
    package.levels[PerformanceCategory::Engine as usize] = 1;
    let resolved = resolve("TESTBIKE", &components, &Selection::default(), package).unwrap();
    let upgraded = keys::apply(&spec, &resolved.params);
    assert!((upgraded.engine.peak_torque - spec.engine.peak_torque * 1.05).abs() < 1e-3);
    assert!((upgraded.chassis.mass - spec.chassis.mass * 0.98).abs() < 1e-3);
    assert_eq!(upgraded.brakes.front_force, spec.brakes.front_force);
    assert_eq!(upgraded.transmission.gears, spec.transmission.gears);
}

#[test]
fn a_stack_cannot_leave_the_range_of_the_base_value() {
    let catalog: Catalog = toml::from_str(
        r#"[[component]]
id = "x"
effects = [{ kind = "param", key = "bike.chassis.mass_kg", op = "mul", value = 9.0 }]"#,
    )
    .unwrap();
    let vehicle: VehicleDef = toml::from_str("id = \"b\"\n[[slot]]\nid = \"s\"\naccepts = [\"x\"]\n").unwrap();
    let mut picks = blackbox_components::Selection::default();
    picks.set("s", blackbox_components::Pick::of("x"));
    let resolved = vehicle.resolve(&catalog, &picks).unwrap();
    let spec = BikeSpec::example();
    assert_eq!(keys::apply(&spec, &resolved.params).chassis.mass, spec.chassis.mass * 2.0);
}

#[test]
fn the_names_the_game_does_not_bind_are_reported() {
    let catalog: Catalog = toml::from_str(CATALOG).unwrap();
    assert!(keys::unbound(&catalog.used()).is_empty());
    let other: Catalog = toml::from_str(
        r#"[[component]]
id = "y"
effects = [
  { kind = "param", key = "stats.mass_kg", op = "add", value = 1.0 },
  { kind = "behaviour", domain = "exhaust.sound", select = "race" },
]"#,
    )
    .unwrap();
    assert_eq!(keys::unbound(&other.used()).len(), 2);
}

#[test]
fn a_published_bike_is_upgraded_by_its_package_and_a_stock_package_changes_nothing() {
    publish("COMPTESTBIKE1", components());
    assert!(of("comptestbike1").is_some() && of("OTHER").is_none());
    let spec = Arc::new(BikeSpec::example());
    let stock = upgraded("COMPTESTBIKE1", Arc::clone(&spec), PerformancePackage::default());
    assert!(Arc::ptr_eq(&stock, &spec));
    let mut package = PerformancePackage::default();
    package.levels[PerformanceCategory::Engine as usize] = 2;
    let fast = upgraded("COMPTESTBIKE1", Arc::clone(&spec), package);
    assert!((fast.engine.peak_torque - spec.engine.peak_torque * 1.1).abs() < 1e-3);
    assert!(Arc::ptr_eq(&upgraded("NOCOMPONENTS", Arc::clone(&spec), package), &spec));
}

#[test]
fn a_car_whose_slots_do_not_fit_its_catalog_still_draws_its_own_stock_parts() {
    let mut parts = components().parts;
    parts
        .insert("exhaust.race".to_owned(), Binding { pack: Some("PARTS/R6/MUF_001".to_owned()), ..binding("MUF_001") });
    let stock = Components::stock_only("StockOnlyBike", parts);
    assert!(stock.vehicle.slots.is_empty() && stock.catalog.get("t:exhaust.stock").is_none());
    let resolved = resolve("STOCKONLYBIKE", &stock, &Selection::default(), PerformancePackage::default()).unwrap();
    // The part of the car's own file is drawn, the aftermarket one of a pack is not.
    assert_eq!(resolved.parts.iter().map(String::as_str).collect::<Vec<_>>(), ["exhaust.stock"]);
}

#[test]
fn the_shop_offers_a_bike_the_levels_of_its_ladders_only() {
    super::super::bike::publish("COMPTESTBIKE2", BikeSpec::example());
    publish("COMPTESTBIKE2", components());
    let offered = catalog::performance("COMPTESTBIKE2", &CarStats::default());
    for category in PerformanceCategory::ALL {
        let expected = if category == PerformanceCategory::Engine { 2 } else { 0 };
        assert_eq!(offered.categories[category as usize].max_level, expected, "{category:?}");
    }
    // A bike with no ladder is offered no upgrade at all, and a car keeps what its stats give.
    super::super::bike::publish("COMPTESTBIKE3", BikeSpec::example());
    assert!(catalog::performance("COMPTESTBIKE3", &CarStats::default()).categories.iter().all(|o| o.max_level == 0));
    assert!(catalog::performance("NOTABIKE", &CarStats::default()).categories.iter().all(|o| o.max_level == 3));
}

fn solid() -> Solid {
    let vertex = |x: f32, y: f32, z: f32| Vertex {
        position: [x, y, z],
        normal: [1.0, 0.0, 0.0],
        color_bgra: [255; 4],
        uv: [0.0; 2],
    };
    Solid {
        name: "TESTBIKE_KIT00_EXHAUST_A".to_owned(),
        name_hash: 1,
        version: 0x16,
        flags: 0,
        bounds_min: [0.0; 3],
        bounds_max: [1.0, 2.0, 3.0],
        transform: [0.0; 16],
        num_polys: 0,
        density: 0.0,
        texture_hashes: vec![5],
        light_material_hashes: vec![6],
        markers: Vec::new(),
        vertex_buffers: Vec::new(),
        vertices: vec![vertex(0.0, 0.0, 0.0), vertex(1.0, 2.0, 3.0)],
        skin: None,
        secondary_uvs: None,
        tangents: Some(vec![[1.0, 0.0, 0.0, -1.0]; 2]),
        indices: vec![0, 1, 0],
        groups: vec![ShadingGroup {
            bounds_min: [0.0; 3],
            bounds_max: [1.0, 2.0, 3.0],
            texture_slots: [0; 5],
            light_material: 0,
            effect_id: 0,
            flags: 0,
            num_vertices: 2,
            num_triangles: 1,
            first_index: 0,
            num_indices: 3,
            base_vertex: 0,
            vertex_buffer: 0,
            bone_table: None,
        }],
    }
}

#[test]
fn rows_make_a_translation_and_a_quarter_turn() {
    let moved = affine([1.0, 0.0, 0.0, 5.0, 0.0, 1.0, 0.0, 6.0, 0.0, 0.0, 1.0, 7.0]);
    assert_eq!(moved.transform_point3(Vec3::ZERO), Vec3::new(5.0, 6.0, 7.0));
    let turned = affine([0.0, -1.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0]);
    assert_eq!(turned.transform_point3(Vec3::X), Vec3::Y);
}

#[test]
fn a_mount_moves_the_vertices_normals_tangents_and_boxes_of_a_copy() {
    let rows = [0.0, -1.0, 0.0, 10.0, 1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0];
    let moved = mounted(&solid(), Some(rows), 42);
    assert_eq!(moved.name_hash, 42);
    assert_eq!(moved.vertices[1].position, [8.0, 1.0, 3.0]);
    // The normal +x turns to +y; the tangent likewise and keeps its handedness.
    assert_eq!(moved.vertices[0].normal, [0.0, 1.0, 0.0]);
    assert_eq!(moved.tangents.as_ref().unwrap()[0], [0.0, 1.0, 0.0, -1.0]);
    assert_eq!(moved.bounds_min, [8.0, 0.0, 0.0]);
    assert_eq!(moved.bounds_max, [10.0, 1.0, 3.0]);
    assert_eq!(moved.groups[0].bounds_max, moved.bounds_max);
    // Without a matrix the copy is the solid under another name.
    let same = mounted(&solid(), None, 7);
    assert_eq!(same.vertices, solid().vertices);
    assert_eq!(same.name_hash, 7);
}

#[test]
fn a_component_in_a_garage_slot_is_a_priced_choice_and_a_ladder_tier_is_not() {
    publish("COMPTESTBIKE4", components());
    assert!(is_choice(choice_hash("COMPTESTBIKE4", "t:exhaust.race")));
    assert!(
        !is_choice(choice_hash("COMPTESTBIKE4", "t:engine.1")),
        "the engine ladder is a performance level, not a part"
    );
    assert!(!is_choice(choice_hash("COMPTESTBIKE5", "t:exhaust.race")));
}

#[test]
fn a_choice_of_a_car_is_told_from_the_model_lists_by_its_folder() {
    publish("COMPTESTBIKE6", components());
    assert!(is_choice_of("comptestbike6", choice_hash("COMPTESTBIKE6", "t:exhaust.race")));
    assert!(!is_choice_of("COMPTESTBIKE6", choice_hash("COMPTESTBIKE6", "t:engine.1")));
    assert!(!is_choice_of("NOCOMPONENTSBIKE", choice_hash("NOCOMPONENTSBIKE", "t:exhaust.race")));
}

#[test]
fn a_base_part_has_no_nearest_level_to_stand_in_for_its_own() {
    use super::parts::levels_to_try;
    assert_eq!(
        levels_to_try(true, 2),
        vec![2],
        "a coarser level of a body with fewer solids draws nothing of this one"
    );
    assert_eq!(
        levels_to_try(false, 2),
        vec![2, 1, 0, 3, 4],
        "an aftermarket part falls back to the nearest level its pack has"
    );
}

use std::collections::HashMap;

use blackbox_collision::{BodyLimits, find_bounds, read_bounds_sets};
use chameleon_gfx::Vertex;
use chameleon_vehicle::{Appearance, PartMesh, PolySoup, SurfaceKind, Vehicle};

use super::*;
use crate::fit::to_car_space;

fn mesh(points: Vec<[f32; 3]>) -> PartMesh {
    PartMesh {
        vertices: points.into_iter().map(|position| Vertex { position, color: [255; 4], uv: [0.0; 2] }).collect(),
        triangles: vec![[0, 1, 2]],
        appearance: Appearance {
            kind: SurfaceKind::Body,
            technique: String::new(),
            diffuse_texture: None,
            diffuse_colour: None,
            alpha_test: false,
        },
    }
}

/// The corners and face centres of a box.
fn boxed(min: [f32; 3], max: [f32; 3]) -> Vec<[f32; 3]> {
    let mut points = Vec::new();
    for x in [min[0], (min[0] + max[0]) / 2.0, max[0]] {
        for y in [min[1], (min[1] + max[1]) / 2.0, max[1]] {
            for z in [min[2], (min[2] + max[2]) / 2.0, max[2]] {
                points.push([x, y, z]);
            }
        }
    }
    points
}

/// A car in the 2012 engine's space (x left, y up, z forward): a body 1.8 m wide, 1.1 m tall and 4.4 m long on four
/// wheels 0.34 m in radius whose models reach 0.2 m outwards and 0.05 m inwards of the hub.
fn vehicle(body_shift_x: f32) -> Vehicle {
    let body = boxed([-0.9 + body_shift_x, 0.2, -2.2], [0.9 + body_shift_x, 1.3, 2.2]);
    let tyre = mesh(boxed([-0.05, -0.34, -0.34], [0.2, 0.34, 0.34]));
    let wheel = |name: &str, x: f32, z: f32| chameleon_vehicle::Wheel {
        name: name.into(),
        position: [x, 0.34, z],
        lods: vec![vec![tyre.clone()]],
    };
    Vehicle {
        body: vec![vec![mesh(body)]],
        wheels: vec![
            wheel("FrontLeft", 0.77, 1.3),
            wheel("FrontRight", -0.77, 1.3),
            wheel("RearLeft", 0.77, -1.3),
            wheel("RearRight", -0.77, -1.3),
        ],
    }
}

fn source(vehicle: Vehicle, collision: Option<PolySoup>) -> SourceCar {
    SourceCar { vehicle, textures: HashMap::new(), paints: Vec::new(), collision }
}

fn soup(min: [f32; 3], max: [f32; 3]) -> PolySoup {
    let vertices = boxed(min, max);
    PolySoup { triangles: vec![[0, 1, 2], [3, 4, 5]], vertices }
}

fn read(collision: &Collision, folder: &str) -> blackbox_collision::BoundsSet {
    let sets = read_bounds_sets(&collision.bytes).unwrap();
    assert_eq!(sets.len(), 1);
    find_bounds(&sets, key(folder)).expect("the set is keyed by the folder").clone()
}

#[test]
fn a_car_without_a_collision_mesh_is_fitted_to_its_visible_body_and_its_tyres() {
    let result = build(&source(vehicle(0.0), None), &to_car_space, "TESTCAR").unwrap();
    assert_eq!(result.basis, Basis::VisibleMesh);
    let set = read(&result, "TESTCAR");
    set.check_vehicle_body(&BodyLimits::default()).unwrap();
    // The tyres stand 0.2 m beyond the hub: 0.77 + 0.2 = 0.97 on each side, wider than the 0.9 m body. Their lowest
    // point is the ground; the roof is at 1.3 m; the body is the longest part.
    assert_eq!(result.half, [0.97, 0.65, 2.2]);
    assert_eq!(set.root().pivot, [0.0, 0.65, 0.0]);
}

#[test]
fn the_collision_mesh_decides_the_body_when_the_bundle_has_one() {
    let car = source(vehicle(0.0), Some(soup([-0.8, 0.25, -2.0], [0.8, 1.2, 2.1])));
    let result = build(&car, &to_car_space, "TESTCAR").unwrap();
    assert_eq!(result.basis, Basis::CollisionMesh);
    // 2.0 m behind and 2.1 m ahead of the origin: half length 2.05, centre 0.05 forward. Tyres still widen it to 0.97.
    assert_eq!(result.half[2], 2.05);
    assert_eq!(result.half[0], 0.97);
    assert_eq!(read(&result, "TESTCAR").root().pivot[2], 0.05);
}

#[test]
fn engine_left_is_the_negative_side_of_the_bounds_and_forward_is_z() {
    // A body shifted 0.1 m to the left (+x in the 2012 files) puts the pivot at -0.1 in vehicle axes (x right).
    let result = build(&source(vehicle(0.1), None), &to_car_space, "TESTCAR").unwrap();
    let pivot = read(&result, "TESTCAR").root().pivot;
    assert!(pivot[0] < 0.0, "{pivot:?}");
    // A soup reaching further forward (+z) moves the pivot forward.
    let car = source(vehicle(0.0), Some(soup([-0.8, 0.25, -1.0], [0.8, 1.2, 2.0])));
    let result = build(&car, &to_car_space, "TESTCAR").unwrap();
    assert!(read(&result, "TESTCAR").root().pivot[2] > 0.0);
}

#[test]
fn the_transform_of_the_geometry_carries_over_to_the_bounds() {
    // A conversion that stretches the car 1.1 times along its length: the bounds follow.
    let stretch = |p: [f32; 3]| {
        let c = to_car_space(p);
        [c[0] * 1.1, c[1], c[2]]
    };
    let plain = build(&source(vehicle(0.0), None), &to_car_space, "TESTCAR").unwrap();
    let stretched = build(&source(vehicle(0.0), None), &stretch, "TESTCAR").unwrap();
    assert!((stretched.half[2] - plain.half[2] * 1.1).abs() < 0.002);
    assert_eq!(stretched.half[0], plain.half[0]);
}

#[test]
fn the_set_is_keyed_by_the_folder_name_whatever_its_case() {
    let result = build(&source(vehicle(0.0), None), &to_car_space, "Por911CarR12").unwrap();
    let set = read(&result, "POR911CARR12");
    assert_eq!(set.name_hash, key("por911carr12"));
    assert_eq!(set.name_hash, blackbox_attrib::vlt_hash("POR911CARR12"));
}

#[test]
fn every_body_node_has_the_car_body_surface() {
    let result = build(&source(vehicle(0.0), None), &to_car_space, "TESTCAR").unwrap();
    let set = read(&result, "TESTCAR");
    assert_eq!(CARBODY_SURFACE, blackbox_attrib::vlt_hash("carbody"));
    assert!(set.nodes[1..].iter().all(|node| node.surface == CARBODY_SURFACE));
}

#[test]
fn a_car_that_is_not_a_vehicle_is_an_error_naming_it() {
    let tiny = Vehicle { body: vec![vec![mesh(boxed([0.0; 3], [0.1; 3]))]], wheels: Vec::new() };
    let error = build(&source(tiny, None), &to_car_space, "TINY").unwrap_err();
    assert!(format!("{error:#}").contains("TINY"), "{error:#}");
}

#[test]
fn a_car_with_no_meshes_is_an_error() {
    let empty = Vehicle { body: Vec::new(), wheels: Vec::new() };
    assert!(build(&source(empty, None), &to_car_space, "EMPTY").is_err());
}

/// Every car of a real 2012 install gets bounds a vehicle can be built from, and they match the visible body.
/// `MW2012_GAME_DIR=<2012 install> cargo test -p nfsmw-2012-content collision -- --ignored --nocapture`
#[test]
#[ignore = "reads the 2012 install (MW2012_GAME_DIR)"]
fn every_car_of_the_install_gets_bounds_that_hold_its_body() {
    let Some(root) = std::env::var_os("MW2012_GAME_DIR") else { return };
    let source = crate::source::Source::open(std::path::Path::new(&root)).unwrap();
    let (mut from_mesh, mut from_body) = (0, 0);
    for file in source.cars() {
        let car = source.load(file).unwrap();
        let name = format!("VEH_{}", file.number);
        let result = build(&car, &to_car_space, &name).unwrap_or_else(|error| panic!("{name}: {error:#}"));
        match result.basis {
            Basis::CollisionMesh => from_mesh += 1,
            Basis::VisibleMesh => from_body += 1,
        }
        let set = read(&result, &name);
        set.check_vehicle_body(&BodyLimits::default()).unwrap_or_else(|error| panic!("{name}: {error:#}"));
        // The root box holds the visible body to within a few centimetres (the collision mesh is a simplified shell).
        let root = set.root();
        let outside = visible_points(&car.vehicle.body)
            .into_iter()
            .map(to_physics_for_test)
            .map(|p| {
                (0..3).map(|a| ((p[a] - root.pivot[a]).abs() - root.half_dimensions[a]).max(0.0)).fold(0.0, f32::max)
            })
            .fold(0.0, f32::max);
        println!(
            "{name:14} {:?} half {:?} nodes {} cloud {} visible body outside the root by {outside:.2} m",
            result.basis,
            result.half,
            set.nodes.len(),
            set.point_clouds[0].len()
        );
        assert!(outside < 0.35, "{name}: the visible body sticks out of the root box by {outside} m");
        // The one car without a collision mesh is the SWAT van: 3.3 m wide with its mirrors, 5.9 m long.
        let (wide, long) = if result.basis == Basis::VisibleMesh { (1.8, 3.2) } else { (1.3, 3.2) };
        assert!(
            result.half[0] > 0.8 && result.half[0] < wide && result.half[2] > 1.4 && result.half[2] < long,
            "{name}"
        );
    }
    println!("{from_mesh} from the collision mesh, {from_body} from the visible body");
    assert!(from_mesh > 50 && from_body <= 2);
}

fn to_physics_for_test(p: [f32; 3]) -> [f32; 3] {
    super::to_physics(to_car_space(p))
}

//! A mod car has no `ecar` record and no `BASE` solid: its lamps are the markers its own body solid carries.

use std::collections::HashMap;

use blackbox_hash::bstring_hash;
use blackbox_solid::{PositionMarker, Solid};
use glam::{Mat4, Vec3};
use nfsmw_data::car::{CarModel, Placement, TextureSwaps};

use super::{CarLights, Kind};

/// A marker at `at` that shines along `shines`, laid out the way the 2012 mod writes it: x axis up, z axis the way the
/// lamp shines.
fn marker(name: &str, at: [f32; 3], shines: [f32; 3]) -> PositionMarker {
    let y = [0.0, -shines[0], 0.0];
    let matrix =
        [0.0, 0.0, 1.0, 0.0, y[0], y[1], y[2], 0.0, shines[0], shines[1], shines[2], 0.0, at[0], at[1], at[2], 1.0];
    PositionMarker { name_hash: bstring_hash(name), int_param: 0, float_params: [0.0; 2], matrix }
}

fn body(markers: Vec<PositionMarker>) -> Solid {
    let name = "MODCAR_KIT00_BODY_A";
    Solid {
        secondary_uvs: None,
        tangents: None,
        name: name.into(),
        name_hash: bstring_hash(name),
        version: 0,
        flags: 0,
        bounds_min: [0.0; 3],
        bounds_max: [0.0; 3],
        transform: [0.0; 16],
        num_polys: 0,
        density: 0.0,
        texture_hashes: Vec::new(),
        light_material_hashes: Vec::new(),
        markers,
        vertex_buffers: Vec::new(),
        vertices: Vec::new(),
        indices: Vec::new(),
        groups: Vec::new(),
        skin: None,
    }
}

fn model(solid: Solid) -> CarModel {
    let placement =
        Placement { solid: solid.name_hash, transform: Mat4::IDENTITY, left_brake: false, slot: 0, corner: None };
    CarModel {
        driving_lods: Default::default(),
        flare_profiles: Default::default(),
        name: "MODCAR".into(),
        car_type: None,
        solids: HashMap::from([(solid.name_hash, solid)]),
        placements: vec![placement],
        textures: HashMap::new(),
        swaps: TextureSwaps::default(),
        paint: None,
        window_tint_material: None,
        paintable: false,
        light_materials: HashMap::new(),
        floor_height: 0.0,
        corners: None,
        wheel_setup: None,
    }
}

#[test]
fn the_markers_of_a_mod_cars_body_are_its_head_brake_and_centre_lamps() {
    let (forward, back) = ([1.0, 0.0, 0.0], [-1.0, 0.0, 0.0]);
    let lights = CarLights::for_model(&model(body(vec![
        marker("LEFT_HEADLIGHT", [2.2, 0.7, 0.65], forward),
        marker("RIGHT_HEADLIGHT", [2.2, -0.7, 0.65], forward),
        marker("LEFT_BRAKELIGHT", [-2.0, 0.6, 0.75], back),
        marker("RIGHT_BRAKELIGHT", [-2.0, -0.6, 0.75], back),
        marker("BRAKELIGHT_CENTRE", [-1.8, 0.0, 0.95], back),
        marker("LEFT_EXHAUST", [-2.0, 0.3, 0.3], back),
    ])));
    let of = |kind| lights.lamps().iter().filter(|lamp| lamp.kind == kind).collect::<Vec<_>>();
    assert_eq!(of(Kind::Head).len(), 2);
    assert_eq!(of(Kind::Tail).len(), 2);
    assert_eq!(of(Kind::CentreBrake).len(), 1);
    assert!(of(Kind::Reverse).is_empty(), "a car with no reverse marker has no reverse flare");
    assert!(of(Kind::Head).iter().all(|lamp| lamp.direction.dot(Vec3::X) > 0.99), "head lamps shine forward");
    assert!(of(Kind::Tail).iter().all(|lamp| lamp.direction.dot(Vec3::NEG_X) > 0.99), "brake lamps shine back");
    // Each lamp keeps the damage identity of its marker name, so a broken left lamp leaves the right one lit.
    let native = lights.lamps().iter().filter(|lamp| !matches!(lamp.kind, Kind::Signal(_)));
    let mut flags: Vec<u32> = native.map(|lamp| lamp.flags).collect();
    flags.sort_unstable();
    assert_eq!(flags, vec![1, 2, 8, 16, 32]);
}

#[test]
fn a_body_with_no_lamp_markers_has_no_lamps() {
    let lights = CarLights::for_model(&model(body(vec![marker("LEFT_EXHAUST", [-2.0, 0.3, 0.3], [-1.0, 0.0, 0.0])])));
    assert!(lights.lamps().is_empty());
}

#[test]
fn a_mod_cars_reverse_lamps_are_left_right_and_centre_markers() {
    let back = [-1.0, 0.0, 0.0];
    let lights = CarLights::for_model(&model(body(vec![
        marker("LEFT_REVERSE", [-2.0, 0.5, 0.35], back),
        marker("RIGHT_REVERSE", [-2.0, -0.5, 0.35], back),
        marker("CENTRE_REVERSE", [-2.3, 0.0, 0.31], back),
    ])));
    let reverse: Vec<_> = lights.lamps().iter().filter(|lamp| lamp.kind == Kind::Reverse).collect();
    assert_eq!(reverse.len(), 3);
    assert!(reverse.iter().all(|lamp| lamp.direction.dot(Vec3::NEG_X) > 0.99), "reverse lamps shine back");
    let mut flags: Vec<u32> = reverse.iter().map(|lamp| lamp.flags).collect();
    flags.sort_unstable();
    assert_eq!(flags, vec![0, 64, 128], "the centre lamp has no damage identity of its own");
}

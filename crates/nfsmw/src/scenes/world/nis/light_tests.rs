//! The character light: the camera rig, the per-vertex rule and the light-material ramps (`nis-player-2.md` 9.6).

use glam::{Mat4, Vec3, Vec4};

use super::*;
use crate::viewer::camera::view;

fn near(a: Vec3, b: Vec3, tolerance: f32) -> bool {
    (a - b).length() < tolerance
}

/// A camera at `from` looking at `at`, the worlds being Z-up.
fn looking(from: Vec3, at: Vec3) -> Eye {
    Eye::new(view(from, at), from)
}

fn light(toward: Vec3, grey: f32) -> Light {
    Light { toward, colour: Vec3::splat(grey) }
}

fn material(min_scale: f32, max_scale: f32, alpha: (f32, f32)) -> LightMaterial {
    LightMaterial {
        name: "CHAR_TEST".to_owned(),
        name_hash: 7,
        version: 3,
        diffuse_min_scale: min_scale,
        diffuse_min: [1.0, 0.5, 0.25],
        diffuse_max_scale: max_scale,
        diffuse_max: [1.0, 1.0, 1.0],
        diffuse_min_alpha: alpha.0,
        diffuse_max_alpha: alpha.1,
        specular_power: 0.0,
        specular_min_scale: 0.0,
        specular_min: [0.0; 3],
        specular_max_scale: 0.0,
        specular_max: [0.0; 3],
        envmap_power: 0.0,
        envmap_min_scale: 0.0,
        envmap_min: [0.0; 3],
        envmap_max_scale: 0.0,
        envmap_max: [0.0; 3],
        metallic_scale: 0.0,
        specular_hot_spot: 0.0,
    }
}

#[test]
fn the_eye_takes_its_axes_from_the_view() {
    let looking_north = looking(Vec3::new(1.0, 2.0, 3.0), Vec3::new(1.0, 7.0, 3.0));
    assert!(near(looking_north.right, Vec3::X, 1e-6), "{looking_north:?}");
    assert!(near(looking_north.down, Vec3::NEG_Z, 1e-6));
    assert!(near(looking_north.forward, Vec3::Y, 1e-6));
    assert_eq!(looking_north.position, Vec3::new(1.0, 2.0, 3.0));
}

#[test]
fn the_rig_is_a_fill_from_the_left_and_a_key_from_the_right_of_the_camera() {
    let [fill, key] = rig(&looking(Vec3::ZERO, Vec3::Y));
    // Camera space (right, down, forward): fill (-0.815, 0.242, -0.527), key (0.469, 0, -0.883).
    assert!(near(fill.toward, Vec3::new(-0.8147, -0.5270, -0.2419), 1e-3), "{fill:?}");
    assert!(near(key.toward, Vec3::new(0.4695, -0.8830, 0.0), 1e-3), "{key:?}");
    assert!(near(fill.colour, Vec3::splat(0.5 * 0.53), 1e-6));
    assert!(near(key.colour, Vec3::splat(0.59), 1e-6));
}

#[test]
fn the_rig_turns_with_the_camera() {
    let north = looking(Vec3::ZERO, Vec3::Y);
    let east = looking(Vec3::new(4.0, -2.0, 1.0), Vec3::new(9.0, -2.0, 1.0));
    let axes = |e: &Eye, l: &Light| Vec3::new(l.toward.dot(e.right), l.toward.dot(e.down), l.toward.dot(e.forward));
    for (a, b) in rig(&north).into_iter().zip(rig(&east)) {
        assert!(near(axes(&north, &a), axes(&east, &b), 1e-5), "the same light in camera space");
        assert_eq!(a.colour, b.colour);
    }
}

#[test]
fn a_face_on_vertex_gets_the_ramp_maximum_times_the_wrapped_lights() {
    let lights = [light(Vec3::Z, 0.5), light(Vec3::X, 0.25)];
    let ramp = Ramp { min: Vec4::new(1.19, 1.19, 1.19, 1.0), range: Vec4::new(0.06, 0.06, 0.06, 0.0475) };
    let lit = vertex_light(Vec3::Z, Vec3::ZERO, Vec3::new(0.0, 0.0, 10.0), &lights, &ramp);
    // Ramp 1.25; the head-on light gives 0.5, the grazing one 0.25 * 0.3.
    assert!((lit - Vec4::new(0.71875, 0.71875, 0.71875, 1.0)).abs().max_element() < 1e-5, "{lit}");
}

#[test]
fn the_wrap_lights_past_the_terminator_and_the_facing_term_has_a_floor() {
    let behind = light(Vec3::new(0.0, 0.96_f32.sqrt(), -0.2), 1.0);
    let lights = [behind, light(Vec3::X, 0.0)];
    let ramp = Ramp { min: Vec4::new(1.0, 1.0, 1.0, 0.5), range: Vec4::new(1.0, 1.0, 1.0, 0.5) };
    let lit = vertex_light(Vec3::Z, Vec3::ZERO, Vec3::new(0.0, 0.0, -10.0), &lights, &ramp);
    // Cosine -0.2 plus the wrap is 0.1; the eye behind the surface leaves the facing term at 0.01.
    assert!((lit - Vec4::new(0.101, 0.101, 0.101, 0.505)).abs().max_element() < 1e-5, "{lit}");
}

#[test]
fn the_vertex_light_saturates() {
    let lights = [light(Vec3::Z, 1.0), light(Vec3::Z, 1.0)];
    let ramp = Ramp { min: Vec4::splat(3.0), range: Vec4::ZERO };
    assert_eq!(vertex_light(Vec3::Z, Vec3::ZERO, Vec3::Z, &lights, &ramp), Vec4::ONE);
    let dark = Ramp { min: Vec4::splat(-1.0), range: Vec4::ZERO };
    assert_eq!(vertex_light(Vec3::Z, Vec3::ZERO, Vec3::Z, &lights, &dark), Vec4::ZERO);
}

#[test]
fn a_light_material_gives_its_diffuse_ramp() {
    let ramp = Ramp::of(&material(1.652, 1.354, (1.019, 1.379)));
    assert!((ramp.min - Vec4::new(1.652, 0.826, 0.413, 1.019)).abs().max_element() < 1e-5, "{ramp:?}");
    let range = Vec4::new(1.354 - 1.652, 1.354 - 0.826, 1.354 - 0.413, 0.25 * (1.379 - 1.019));
    assert!((ramp.range - range).abs().max_element() < 1e-5, "{ramp:?}");
    assert_eq!(ramps(&[material(1.0, 1.0, (1.0, 1.0))]).get(&7), Some(&Ramp::of(&material(1.0, 1.0, (1.0, 1.0)))));
}

#[test]
fn the_actor_frame_lights_the_bind_pose_as_render_space_would() {
    let frame = Mat4::from_translation(Vec3::new(5.0, 0.0, 0.0)) * Mat4::from_rotation_z(std::f32::consts::FRAC_PI_2);
    let camera = looking(Vec3::new(4.0, 8.0, 1.5), Vec3::new(5.0, 0.0, 1.0));
    let ramp = Ramp { min: Vec4::new(1.0, 0.9, 0.8, 1.0), range: Vec4::new(0.3, 0.2, 0.1, 0.0) };
    let vertex = Vertex { position: [0.2, 0.0, 1.0], normal: [1.0, 0.0, 0.0], color_bgra: [255; 4], uv: [0.0; 2] };
    let local = Lighting::new(&camera, frame).colour(&vertex, &ramp);
    let normal = frame.transform_vector3(Vec3::X);
    let position = frame.transform_point3(Vec3::new(0.2, 0.0, 1.0));
    let world = vertex_light(normal, position, camera.position, &rig(&camera), &ramp);
    let bytes = [world.z, world.y, world.x, world.w].map(|x| (x * 255.0).round() as i32);
    for (got, want) in local.iter().zip(bytes) {
        assert!((i32::from(*got) - want).abs() <= 1, "{local:?} against {bytes:?}");
    }
    assert!(local[2] > local[0], "the red channel comes third (D3DCOLOR order) and is the brightest here");
}

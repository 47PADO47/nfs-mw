use super::super::car_light_flares::FlareView;
use super::*;
use blackbox_gfx::{BlendMode, TextureHandle};
use glam::{Mat4, Vec3};

#[test]
fn blink_channels_use_strict_boundaries_discard_overshoot_and_ignore_invalid_steps() {
    let mut clock = BlinkClock::default();
    assert_eq!(clock.on, [false; 3]);
    clock.advance(0.4);
    assert_eq!(clock.on, [false; 3]);
    clock.advance(0.05);
    assert_eq!(clock.on, [false, true, true]);
    clock.advance(0.05);
    assert_eq!(clock.on, [false, true, true], "red is exactly at its threshold");
    clock.advance(0.001);
    assert_eq!(clock.on, [true; 3]);
    for invalid in [0.0, -1.0, f32::NAN, f32::INFINITY] {
        clock.advance(invalid);
    }
    assert_eq!(clock.on, [true; 3]);
    clock.advance(3.0);
    assert_eq!(clock.on, [false; 3], "one toggle; no accumulated overshoot or catch-up loop");
    assert_eq!(clock.elapsed, [0.0; 3]);
}

#[test]
fn amber_phase_uses_truncated_world_x_including_negative_positions() {
    let clock = BlinkClock { on: [false, true, false], ..Default::default() };
    for x in [1.9, -1.9, 4.1, -4.1] {
        assert!(clock.lit(Kind::AmberBlink, x));
    }
    for x in [0.9, -0.9, 2.9, -2.9, 3.1] {
        assert!(!clock.lit(Kind::AmberBlink, x));
    }
    assert!(clock.lit(Kind::Orange, 0.0));
    assert!(!clock.lit(Kind::RedBlink, 1.0));
    assert!(!clock.lit(Kind::GreenBlink, 1.0));
}

#[test]
fn blink_direction_comes_from_transformed_horizontal_displacement() {
    let marker = Marker { position: Vec3::new(2.0, 0.0, 3.0), kind: Kind::AmberBlink };
    let world = Mat4::from_translation(Vec3::new(10.0, 20.0, 0.0)) * Mat4::from_rotation_z(std::f32::consts::FRAC_PI_2);
    let (at, facing) = draw::sample(marker, world, Vec3::new(10.0, 30.0, 3.0), Vec3::NEG_Y).unwrap();
    assert!(at.distance(Vec3::new(10.0, 22.0, 3.0)) < 1e-5);
    assert!((facing - 1.0).abs() < 1e-5);
    assert!(draw::sample(marker, world, Vec3::new(10.0, 10.0, 3.0), Vec3::Y).unwrap().1 < 0.0);
}

#[test]
fn reflector_cone_uses_camera_aim_and_ignores_marker_direction() {
    let marker = Marker { position: Vec3::new(10.0, 0.0, 0.0), kind: Kind::Orange };
    assert!((draw::sample(marker, Mat4::IDENTITY, Vec3::ZERO, Vec3::X).unwrap().1 - 1.0).abs() < 1e-5);
    assert!(draw::sample(marker, Mat4::IDENTITY, Vec3::ZERO, Vec3::Y).is_none());
    let forward = Vec3::new(0.98, (1.0_f32 - 0.98_f32.powi(2)).sqrt(), 0.0);
    assert!((draw::sample(marker, Mat4::IDENTITY, Vec3::ZERO, forward).unwrap().1 - 0.5).abs() < 1e-5);
}

pub(super) fn fixture() -> (EnvironmentLights, HashMap<u32, (TextureHandle, BlendMode)>, FlareView) {
    let mesh = MeshHandle::from_raw(1);
    let profile = FlareProfile {
        size: [30.0; 2],
        scale: [0.001, 0.1],
        power: 1.0,
        z_bias: 0.3,
        color: [1.0, 0.5, 0.1, 1.0],
        texture: 1,
    };
    let lights = EnvironmentLights {
        blink: BlinkClock { on: [true; 3], ..Default::default() },
        profiles: vec![(Kind::AmberBlink, profile)],
        sources: HashMap::from([(mesh, vec![Marker { position: Vec3::X, kind: Kind::AmberBlink }].into())]),
        revision: None,
        detail: Default::default(),
        visible: vec![Instance::new(mesh, Mat4::IDENTITY)],
    };
    let textures = HashMap::from([(1, (TextureHandle::from_raw(1), BlendMode::Additive))]);
    let view = FlareView { camera: Vec3::new(10.0, 0.0, 0.0), forward: Vec3::NEG_X, fov_degrees: 70.0, height: 720 };
    (lights, textures, view)
}

#[test]
fn draws_authored_markers_without_a_car_and_batches_multiple_lights() {
    let (mut lights, textures, view) = fixture();
    let mesh = lights.visible[0].mesh;
    lights.sources.insert(
        mesh,
        vec![
            Marker { position: Vec3::X, kind: Kind::AmberBlink },
            Marker { position: Vec3::new(1.0, 1.0, 0.0), kind: Kind::AmberBlink },
        ]
        .into(),
    );
    let mut out = Vec::new();
    lights.draw(&textures, view, &mut out);
    assert_eq!(out.len(), 1);
    assert_eq!(out[0].vertices.len(), 12);
    assert_eq!(out[0].blend, BlendMode::Additive);
    lights.blink.on = [false; 3];
    out.clear();
    lights.draw(&textures, view, &mut out);
    assert!(out.is_empty());
}

#[test]
fn missing_or_retired_meshes_and_hidden_or_unselected_instances_do_not_leave_lights() {
    let (mut lights, textures, view) = fixture();
    let mut out = Vec::new();
    lights.visible[0].mesh = MeshHandle::from_raw(2);
    lights.draw(&textures, view, &mut out);
    assert!(out.is_empty(), "a different LOD has no inherited guessed lamp");
    lights.visible.clear();
    lights.draw(&textures, view, &mut out);
    assert!(out.is_empty(), "hidden geometry has no ghost lamp");
}

#[test]
fn respects_native_pool_bound_and_current_mesh_transform() {
    let (mut lights, textures, view) = fixture();
    lights.visible = vec![lights.visible[0]; 60];
    let mut out = Vec::new();
    lights.draw(&textures, view, &mut out);
    assert_eq!(out[0].vertices.len(), 50 * 6);
    lights.visible.truncate(1);
    lights.visible[0].transform = Mat4::from_translation(Vec3::new(300.0, 0.0, 0.0));
    out.clear();
    lights.draw(&textures, view, &mut out);
    assert!(out.is_empty(), "the lamp follows the prop beyond the camera/range");
}

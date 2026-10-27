use super::*;

fn state(nitrous: bool) -> crate::scenes::world::exhaust::CarState {
    crate::scenes::world::exhaust::CarState {
        to_world: Mat4::from_translation(Vec3::new(20.0, 30.0, 1.0)),
        velocity: Vec3::X * 80.0,
        gear: 3,
        throttle: 1.0,
        nitrous,
    }
}

fn effect() -> Enhanced {
    Enhanced { enabled: true, texture: Some(TextureHandle::from_raw(1)), ..Default::default() }
}

#[test]
fn flame_roots_follow_the_installed_pipe_axis_without_world_space_gaps() {
    let mut effect = effect();
    effect.step(1.0 / 60.0, &state(true), None);
    let pipe = Mat4::from_rotation_translation(
        glam::Quat::from_rotation_y(-std::f32::consts::FRAC_PI_2),
        Vec3::new(-2.0, 0.5, 0.3),
    );
    let mut volumes = Vec::new();
    assert!(effect.geometry(&[pipe], &mut volumes));
    let volume = &volumes[0];
    let root = volume.to_world.transform_point3(Vec3::NEG_Y * 0.5);
    let tip = volume.to_world.transform_point3(Vec3::Y * 0.5);
    let marker = state(true).to_world * pipe;
    assert!(root.abs_diff_eq(marker.transform_point3(Vec3::ZERO), 1e-5));
    assert!((tip - root).normalize().dot(marker.transform_vector3(Vec3::Z).normalize()) > 0.999);
    assert!(volume.blue);
}

#[test]
fn nitrous_response_is_time_based_and_reset_does_not_leave_a_flame() {
    let mut a = effect();
    let mut b = effect();
    for _ in 0..60 {
        a.step(1.0 / 60.0, &state(true), None);
    }
    for _ in 0..144 {
        b.step(1.0 / 144.0, &state(true), None);
    }
    assert!((a.nitrous - b.nitrous).abs() < 1e-5);
    for _ in 0..30 {
        a.step(1.0 / 60.0, &state(false), None);
    }
    let mut out = Vec::new();
    a.geometry(&[Mat4::IDENTITY], &mut out);
    assert!(out.is_empty());
    b.disconnect();
    b.geometry(&[Mat4::IDENTITY], &mut out);
    assert!(out.is_empty());
}

#[test]
fn selecting_original_falls_back_and_backfire_stays_orange() {
    let mut effect = effect();
    effect.step(0.02, &state(false), Some(crate::garage::BackfireEvent::Pop));
    let mut volumes = Vec::new();
    effect.geometry(&[Mat4::IDENTITY], &mut volumes);
    assert_eq!(volumes.len(), 1);
    assert!(!volumes[0].blue);
    effect.enabled = false;
    assert!(!effect.geometry(&[Mat4::IDENTITY], &mut Vec::new()));
}

#[test]
fn each_ignition_changes_the_flow_seed_without_reseeding_a_held_burst() {
    let mut effect = effect();
    effect.step(0.02, &state(true), None);
    let first = effect.seed;
    for _ in 0..120 {
        effect.step(1.0 / 60.0, &state(true), None);
    }
    assert_eq!(effect.seed, first);
    effect.step(0.02, &state(false), None);
    effect.step(0.02, &state(true), None);
    assert_ne!(effect.seed, first);
    effect.disconnect();
    effect.step(0.02, &state(true), None);
    assert_ne!(effect.seed, first);
}

#[test]
fn nitrous_flow_accelerates_with_vehicle_speed_without_changing_activation() {
    let mut slow = effect();
    let mut fast = effect();
    let mut car = state(true);
    car.velocity = Vec3::ZERO;
    slow.step(0.1, &car, None);
    car.velocity = Vec3::Z * 80.0;
    fast.step(0.1, &car, None);
    assert!(fast.time > slow.time);
    assert_eq!(fast.nitrous, slow.nitrous);
    assert_eq!(fast.seed, slow.seed);
}

#[test]
fn enhanced_appearance_is_independent_and_round_trips_to_saved_settings() {
    use crate::settings::{Partial, Settings, WorldTuning, file, write};
    assert!(!Settings::from(Partial::default()).enhanced_exhaust);
    let layer = file::parse("enhanced_exhaust = true\nexhaust_flames = true", "test");
    let settings = Settings::from(layer);
    assert!(WorldTuning::from(&settings).enhanced_exhaust);
    let text = write::merge("", &settings.graphics_layer()).unwrap();
    assert_eq!(file::parse(&text, "test").enhanced_exhaust, Some(true));
}

#[test]
fn upward_curl_follows_world_up_and_customization_keeps_the_root_attached() {
    let mut effect = effect();
    let mut car = state(true);
    car.velocity = Vec3::ZERO;
    car.to_world = Mat4::from_rotation_x(0.4);
    effect.step(0.1, &car, None);
    let pipe = Mat4::from_rotation_y(-std::f32::consts::FRAC_PI_2);
    let mut volumes = Vec::new();
    effect.geometry(&[pipe], &mut volumes);
    let first = &volumes[0];
    assert_eq!(first.appearance.root_color[3], 0.0, "neutral settings preserve reference shading");
    let up = first.to_world.transform_vector3(first.appearance.bend);
    assert!(up.z > 0.01 && up.x.abs() < 1e-5 && up.y.abs() < 1e-5);
    effect.appearance.curl = 0;
    effect.appearance.response = 0;
    effect.appearance.length = 140;
    effect.appearance.root = [0, 255, 0];
    volumes.clear();
    effect.geometry(&[pipe], &mut volumes);
    assert_eq!(volumes[0].appearance.bend, Vec3::ZERO);
    assert_eq!(volumes[0].appearance.root_tint.x, 0.0);
    assert_eq!(volumes[0].appearance.root_color, [0.0, 1.0, 0.0, 1.0]);
    assert!(volumes[0].to_world.transform_point3(Vec3::NEG_Y * 0.5).abs_diff_eq(Vec3::ZERO, 1e-5));
    effect.appearance.root = [255; 3];
    volumes.clear();
    effect.geometry(&[pipe], &mut volumes);
    assert_eq!(volumes[0].appearance.root_color, [1.0; 4], "white must not inherit the blue reference hue");
}

#[test]
fn garage_preview_emits_without_nitrous_and_stops_when_disabled() {
    let mut effect = effect();
    effect.step(0.1, &state(false), None);
    effect.set_preview(true);
    let mut volumes = Vec::new();
    effect.geometry(&[Mat4::IDENTITY], &mut volumes);
    assert_eq!(effect.nitrous, 0.0);
    assert_eq!(volumes.len(), 1);
    assert!(volumes[0].blue);
    effect.set_preview(false);
    volumes.clear();
    effect.geometry(&[Mat4::IDENTITY], &mut volumes);
    assert!(volumes.is_empty());
}

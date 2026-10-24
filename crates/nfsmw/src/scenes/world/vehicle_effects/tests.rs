use super::*;
use crate::settings::XenonEffects;
use glam::Quat;
use nfsmw_data::vehicle_effects::EmitterStyle;

const STEP: f32 = 1.0 / 60.0;
const ACTIVATION_SPEED: f32 = 44.0;

fn style() -> EmitterStyle {
    EmitterStyle {
        color: [0.8, 1.0, 0.8, 0.15],
        life: 0.25,
        life_variance: 0.25,
        count: 15.0,
        extent: [1.5, 2.0, 0.75],
        velocity: [0.0, 0.0, -1.0],
        inherit: [-0.3, -0.3, -0.1],
        gravity: -2.0,
        length: 200.0,
        length_delta: 50.0,
        height: 255.0,
        ..EmitterStyle::default()
    }
}

fn pose() -> CarPose {
    CarPose {
        position: Vec3::new(5.0, 7.0, 2.0),
        rotation: Quat::IDENTITY,
        wheels: [nfsmw_data::car::WheelPose::default(); 4],
    }
}

fn emit(trails: &mut Trails, speed: f32, dt: f32) {
    trails.emit(Some(style()), pose(), Vec3::X * speed, false, &XenonEffects::default(), dt);
}

#[test]
fn wind_trails_gate_total_speed_and_stop_below_threshold_without_debt() {
    let mut trails = Trails::default();
    for speed in [0.0, ACTIVATION_SPEED - 0.01] {
        emit(&mut trails, speed, 1.0);
    }
    assert_eq!(trails.len(), 0);
    for _ in 0..60 {
        emit(&mut trails, ACTIVATION_SPEED, STEP);
    }
    assert_eq!(trails.emitted, 900);
    let count = trails.emitted;
    emit(&mut trails, 0.0, 1.0);
    assert_eq!(trails.emitted, count);
    trails.age(0.6);
    assert_eq!(trails.len(), 0);
    emit(&mut trails, 44.0, STEP);
    assert_eq!(trails.len(), 15, "one dispatch with no accumulated spawn debt");
    let mut reverse = Trails::default();
    emit(&mut reverse, -100.0, STEP);
    assert_eq!(reverse.emitted, 15, "the restoration gates magnitude, including reverse");
}

#[test]
fn trail_brightness_rises_with_speed_without_changing_density_and_remains_bounded() {
    let mut slow = Trails::default();
    let mut fast = Trails::default();
    for _ in 0..60 {
        emit(&mut slow, 44.0, STEP);
        emit(&mut fast, 88.0, STEP);
    }
    assert_eq!(fast.emitted, slow.emitted);
    let mut dim = Vec::new();
    let mut bright = Vec::new();
    slow.geometry(Vec3::ZERO, Vec3::X, &mut dim);
    fast.geometry(Vec3::ZERO, Vec3::X, &mut bright);
    assert!(bright[0].color[3] > dim[0].color[3]);
    for _ in 0..1000 {
        emit(&mut fast, 200.0, 1000.0);
    }
    assert_eq!(fast.len(), XenonEffects::default().max_trails);
    let mut vertices = Vec::new();
    fast.geometry(Vec3::new(-10.0, 0.0, 8.0), Vec3::X, &mut vertices);
    assert!(vertices.len() <= XenonEffects::default().max_trails * 6);
    assert!(vertices.iter().all(|v| Vec3::from(v.position).is_finite()));
}

#[test]
fn lifetime_without_physics_disable_reset_and_free_camera_disconnect_trails() {
    let data = VisualEffectsData { trail: Some(style()), ..VisualEffectsData::default() };
    let mut effects = VehicleEffects::new(data);
    effects.set_enabled(true, true);
    effects.step(&[], pose(), Vec3::X * 88.0, false, 0.1);
    assert!(effects.trails.len() > 0);
    let mut chase = Vec::new();
    effects.geometry(Vec3::ZERO, Vec3::X, true, &mut chase);
    assert!(!chase.is_empty());
    let mut free = Vec::new();
    effects.geometry(Vec3::ZERO, Vec3::X, false, &mut free);
    assert!(free.is_empty());
    effects.disconnect();
    assert_eq!(effects.trails.len(), 0);
    effects.step(&[], pose(), Vec3::X * 88.0, false, 0.1);
    effects.age(0.6);
    assert_eq!(effects.trails.len(), 0);
    effects.step(&[], pose(), Vec3::X * 88.0, false, 0.1);
    effects.set_enabled(true, false);
    effects.step(&[], pose(), Vec3::X * 88.0, false, 0.1);
    assert_eq!(effects.trails.len(), 0);
    effects.set_enabled(false, true);
    effects.step(&[], pose(), Vec3::X * 88.0, false, 0.1);
    effects.clear();
    assert_eq!(effects.trails.len(), 0);
}

#[test]
fn invalid_time_and_velocity_never_create_or_age_particles() {
    let mut effects = VehicleEffects::new(VisualEffectsData { trail: Some(style()), ..VisualEffectsData::default() });
    effects.set_enabled(true, true);
    for dt in [f32::NAN, f32::INFINITY, -1.0, 0.0] {
        effects.step(&[], pose(), Vec3::X * 88.0, false, dt);
    }
    effects.step(&[], pose(), Vec3::NAN, false, STEP);
    effects.step(&[], CarPose { position: Vec3::NAN, ..pose() }, Vec3::X * 88.0, false, STEP);
    assert_eq!(effects.trails.len(), 0);
}

#[test]
fn repeated_runs_have_identical_trails_after_motion_and_camera_changes() {
    let run = || {
        let mut trails = Trails::default();
        for _ in 0..30 {
            trails.age(STEP);
            emit(&mut trails, 88.0, STEP);
        }
        let mut vertices = Vec::new();
        trails.geometry(Vec3::new(-8.0, -7.0, 9.0), Vec3::X, &mut vertices);
        vertices
    };
    assert_eq!(run(), run());
}

#[test]
fn speed_streaks_are_gone_below_the_trail_speed_and_grow_with_it() {
    use super::streak_strength;
    for speed in [0.0, -10.0, ACTIVATION_SPEED - 0.01] {
        assert_eq!(streak_strength(speed, false, &XenonEffects::default()), 0.0, "no streaks at {speed} m/s");
    }
    let at_start = streak_strength(ACTIVATION_SPEED, false, &XenonEffects::default());
    assert!((at_start - 0.1).abs() < 1e-6, "the trail's first intensity: {at_start}");
    assert!(streak_strength(60.0, false, &XenonEffects::default()) > at_start, "stronger as the car goes faster");
    assert_eq!(
        streak_strength(-120.0, false, &XenonEffects::default()),
        streak_strength(120.0, false, &XenonEffects::default()),
        "reverse counts as speed"
    );
    assert_eq!(streak_strength(200.0, false, &XenonEffects::default()), 0.75, "capped like the trail");
}

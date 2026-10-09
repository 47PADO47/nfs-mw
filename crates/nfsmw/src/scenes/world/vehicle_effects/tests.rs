use super::*;
use glam::Quat;
use nfsmw_data::vehicle_effects::EmitterStyle;

const STEP: f32 = 1.0 / 60.0;

fn style() -> EmitterStyle {
    EmitterStyle { color: [0.8, 1.0, 0.8, 0.15], life: 0.25, life_variance: 0.25 }
}

fn pose() -> CarPose {
    CarPose {
        position: Vec3::new(5.0, 7.0, 2.0),
        rotation: Quat::IDENTITY,
        wheels: [nfsmw_data::car::WheelPose::default(); 4],
    }
}

fn emit(trails: &mut Trails, speed: f32, dt: f32) {
    trails.emit(Some(style()), pose(), Vec3::X * speed, -Vec3::X * 2.0, Vec3::ONE, dt);
}

#[test]
fn wind_trails_gate_forward_speed_and_stop_below_threshold_without_debt() {
    let mut trails = Trails::default();
    for speed in [0.0, -100.0, trails::ACTIVATION_SPEED - 0.01] {
        emit(&mut trails, speed, 1.0);
    }
    assert_eq!(trails.len(), 0);
    for _ in 0..60 {
        emit(&mut trails, trails::ACTIVATION_SPEED, STEP);
    }
    assert!((23..=24).contains(&trails.len()));
    let count = trails.emitted;
    emit(&mut trails, 0.0, 1.0);
    assert_eq!(trails.emitted, count);
    trails.age(0.6);
    assert_eq!(trails.len(), 0);
    emit(&mut trails, 44.0, STEP);
    assert_eq!(trails.len(), 0, "no accumulated spawn debt");
}

#[test]
fn trail_density_rises_with_speed_and_remains_bounded() {
    let mut slow = Trails::default();
    let mut fast = Trails::default();
    for _ in 0..60 {
        emit(&mut slow, 44.0, STEP);
        emit(&mut fast, 88.0, STEP);
    }
    assert!(fast.emitted > slow.emitted * 6);
    for _ in 0..1000 {
        emit(&mut fast, 200.0, 1000.0);
    }
    assert_eq!(fast.len(), MAX_TRAILS);
    let mut vertices = Vec::new();
    fast.geometry(Vec3::new(-10.0, 0.0, 8.0), Vec3::X, &mut vertices);
    assert!(vertices.len() <= MAX_TRAILS * 6);
    assert!(vertices.iter().all(|v| Vec3::from(v.position).is_finite()));
}

#[test]
fn lifetime_without_physics_disable_reset_and_free_camera_disconnect_trails() {
    let data = VisualEffectsData { trail: Some(style()), ..VisualEffectsData::default() };
    let mut effects = VehicleEffects::new(data, -Vec3::X * 2.0, Vec3::ONE);
    effects.set_enabled(true, true);
    effects.step(&[], pose(), Vec3::X * 88.0, 0.1);
    assert!(effects.trails.len() > 0);
    let mut chase = Vec::new();
    effects.geometry(Vec3::ZERO, Vec3::X, true, &mut chase);
    assert!(!chase.is_empty());
    let mut free = Vec::new();
    effects.geometry(Vec3::ZERO, Vec3::X, false, &mut free);
    assert!(free.is_empty());
    effects.disconnect();
    assert_eq!(effects.trails.len(), 0);
    effects.step(&[], pose(), Vec3::X * 88.0, 0.1);
    effects.age(0.6);
    assert_eq!(effects.trails.len(), 0);
    effects.step(&[], pose(), Vec3::X * 88.0, 0.1);
    effects.set_enabled(true, false);
    effects.step(&[], pose(), Vec3::X * 88.0, 0.1);
    assert_eq!(effects.trails.len(), 0);
    effects.set_enabled(false, true);
    effects.step(&[], pose(), Vec3::X * 88.0, 0.1);
    effects.clear();
    assert_eq!(effects.trails.len(), 0);
}

#[test]
fn invalid_time_and_velocity_never_create_or_age_particles() {
    let mut effects = VehicleEffects::new(
        VisualEffectsData { trail: Some(style()), ..VisualEffectsData::default() },
        Vec3::ZERO,
        Vec3::ONE,
    );
    effects.set_enabled(true, true);
    for dt in [f32::NAN, f32::INFINITY, -1.0, 0.0] {
        effects.step(&[], pose(), Vec3::X * 88.0, dt);
    }
    effects.step(&[], pose(), Vec3::NAN, STEP);
    effects.step(&[], CarPose { position: Vec3::NAN, ..pose() }, Vec3::X * 88.0, STEP);
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

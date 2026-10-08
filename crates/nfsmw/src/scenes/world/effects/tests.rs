use blackbox_render::EffectVertex;
use blackbox_vehicle::{FIXED_STEP, FlatGround, InputState, Vehicle, VehicleSpec};
use glam::Vec3;

use super::*;

fn contact(x: f32) -> Contact {
    Contact {
        point: Vec3::new(x, 0.0, 5.0),
        normal: Vec3::Z,
        forward: Vec3::X,
        section: 7,
        skid: 1.0,
        smoke: 1.0,
        width: 0.24,
    }
}

fn vertices(effects: &mut TireEffects) -> (Vec<EffectVertex>, Vec<EffectVertex>) {
    let layer = effects.build(Vec3::new(-8.0, 0.0, 8.0), Vec3::X);
    (layer.surfaces.clone(), layer.particles.clone())
}

#[test]
fn rolling_and_lost_contact_emit_nothing() {
    let mut effects = TireEffects::default();
    let quiet = Contact { skid: 0.0, smoke: 0.0, ..contact(0.0) };
    for _ in 0..120 {
        effects.step([Some(quiet); 4], Vec3::X, FIXED_STEP);
    }
    effects.step([None; 4], Vec3::ZERO, FIXED_STEP);
    assert_eq!(effects.smoke.len(), 0);
    assert_eq!(effects.marks.len(), 0);
}

#[test]
fn stationary_burnout_reuses_one_stamp_per_wheel_and_smoke_expires() {
    let mut effects = TireEffects::default();
    for _ in 0..600 {
        effects.step([Some(contact(0.0)); 4], Vec3::ZERO, FIXED_STEP);
    }
    assert_eq!(effects.marks.len(), 4, "stationary burnouts must not pile up quads");
    assert!(effects.smoke.len() > 100 && effects.smoke.len() <= MAX_PARTICLES);
    assert!(effects.smoke.oldest() <= 2.0);
    let (marks, particles) = vertices(&mut effects);
    assert_eq!(marks.len(), 24);
    assert!(particles.iter().all(|v| v.position.iter().all(|x| x.is_finite())));
    effects.disconnect();
    effects.age(2.1);
    assert_eq!(effects.smoke.len(), 0, "parked effects still age without physics steps");
    assert_eq!(effects.marks.len(), 4);
    effects.age(45.1);
    assert_eq!(effects.marks.len(), 0);
}

#[test]
fn moving_skids_follow_each_contact_normal_and_fade() {
    let mut effects = TireEffects::default();
    let normal = Vec3::new(-0.2, 0.0, 1.0).normalize();
    let forward = Vec3::new(1.0, 0.0, 0.2).normalize();
    for n in 0..30 {
        let x = n as f32 * 0.2;
        let c = Contact { point: Vec3::new(x, 0.0, 5.0 + x * 0.2), normal, forward, smoke: 0.0, ..contact(x) };
        effects.step([Some(c), None, None, None], Vec3::X, FIXED_STEP);
    }
    let initial = vertices(&mut effects).0;
    assert!(effects.marks.len() > 20);
    assert!(
        initial.iter().all(|v| {
            let p = Vec3::from_array(v.position);
            (p.dot(normal) - Vec3::new(0.0, 0.0, 5.0).dot(normal) - 0.012).abs() < 1e-4
        }),
        "every end follows the contact plane with the depth offset"
    );
    effects.age(42.0);
    let faded = vertices(&mut effects).0;
    assert!(faded.iter().zip(initial).all(|(after, before)| after.color[3] < before.color[3]));
    effects.age(4.0);
    assert_eq!(effects.marks.len(), 0);
}

#[test]
fn stationary_stamp_follows_small_movements_steering_and_tread_width() {
    let mut effects = TireEffects::default();
    effects.step([Some(contact(0.0)), None, None, None], Vec3::ZERO, FIXED_STEP);
    let c = Contact { point: Vec3::new(0.1, 0.05, 5.0), forward: Vec3::Y, width: 0.3, ..contact(0.0) };
    effects.step([Some(c), None, None, None], Vec3::ZERO, FIXED_STEP);
    let geometry = vertices(&mut effects).0;
    assert_eq!(geometry.len(), 6);
    let p = |i: usize| Vec3::from_array(geometry[i].position);
    assert!(((p(0) + p(2)) * 0.5 - (c.point + Vec3::Z * 0.012)).length() < 1e-5);
    assert!((p(0).distance(p(1)) - c.width).abs() < 1e-5);
    assert!((p(1).distance(p(2)) - 0.18).abs() < 1e-5);
    // Sampling remains relative to the initial point, so small steps eventually form a strip.
    effects.step([Some(Contact { point: Vec3::new(0.2, 0.05, 5.0), ..c }), None, None, None], Vec3::X, FIXED_STEP);
    let strip = vertices(&mut effects).0[6..].to_vec();
    assert_eq!(strip.len(), 6);
    effects.step([Some(Contact { point: Vec3::new(0.25, 0.05, 5.0), ..c }), None, None, None], Vec3::X, FIXED_STEP);
    assert_eq!(&vertices(&mut effects).0[6..], strip, "finished strips must not move with the tire");
}

#[test]
fn lift_teleport_respawn_section_and_sharp_normal_changes_break_tracks() {
    let mut effects = TireEffects::default();
    let sample = |effects: &mut TireEffects, c| effects.step([c, None, None, None], Vec3::ZERO, FIXED_STEP);
    sample(&mut effects, Some(contact(0.0)));
    sample(&mut effects, Some(contact(0.3)));
    sample(&mut effects, None);
    sample(&mut effects, Some(contact(0.9)));
    sample(&mut effects, Some(contact(1000.0)));
    effects.disconnect();
    sample(&mut effects, Some(contact(1000.3)));
    sample(&mut effects, Some(Contact { section: 9, ..contact(1000.6) }));
    sample(&mut effects, Some(Contact { section: 9, normal: Vec3::Y, ..contact(1000.9) }));
    assert_eq!(effects.marks.len(), 7);
    let geometry = vertices(&mut effects).0;
    for quad in geometry.as_chunks::<6>().0 {
        let points: Vec<_> = quad.iter().map(|v| Vec3::from_array(v.position)).collect();
        assert!(points.iter().all(|a| points.iter().all(|b| a.distance(*b) < 0.6)), "track bridged a gap");
    }
    effects.retain_sections(|s| s != 7);
    assert_eq!(effects.marks.len(), 2);
    effects.retain_sections(|_| false);
    assert_eq!(effects.marks.len(), 0);
}

#[test]
fn marks_and_particles_have_hard_budgets_and_reuse_storage() {
    let mut effects = TireEffects::default();
    for n in 0..(MAX_MARKS + 100) {
        let x = n as f32 * 0.25;
        effects.step([Some(contact(x)); 4], Vec3::X, FIXED_STEP);
    }
    assert_eq!(effects.marks.len(), MAX_MARKS);
    assert!(effects.smoke.len() <= MAX_PARTICLES);
    // Stress emission without aging: even a saturated burst replaces the oldest rather than growing.
    for _ in 0..100 {
        effects.smoke.emit(0, Some(contact(0.0)), Vec3::ZERO, 1.0);
    }
    assert_eq!(effects.smoke.len(), MAX_PARTICLES);
    let geometry = effects.build(Vec3::ZERO, Vec3::X);
    assert_eq!(geometry.surfaces.len(), MAX_MARKS * 6);
    assert_eq!(geometry.particles.len(), MAX_PARTICLES * 6);
    let capacity = [geometry.surfaces.capacity(), geometry.particles.capacity()];
    effects.clear();
    let geometry = effects.build(Vec3::ZERO, Vec3::X);
    assert!(geometry.surfaces.is_empty() && geometry.particles.is_empty());
    assert_eq!([geometry.surfaces.capacity(), geometry.particles.capacity()], capacity);
}

#[test]
fn independent_toggles_clear_resources_and_invalid_time_cannot_emit() {
    let mut effects = TireEffects::default();
    for _ in 0..60 {
        effects.step([Some(contact(0.0)); 4], Vec3::ZERO, FIXED_STEP);
    }
    effects.set_enabled(false, true);
    assert_eq!(effects.smoke.len(), 0);
    assert_eq!(effects.marks.len(), 4);
    effects.set_enabled(true, false);
    assert_eq!(effects.marks.len(), 0);
    for dt in [0.0, -1.0, f32::INFINITY, f32::NAN] {
        effects.step([Some(contact(0.0)); 4], Vec3::ZERO, dt);
    }
    assert_eq!(effects.smoke.len(), 0);
    effects.command(&["smoke", "off"]).unwrap();
    effects.command(&["clear"]).unwrap();
    assert!(effects.command(&["smoke", "maybe"]).is_err());
}

#[test]
fn fixed_step_runs_produce_identical_geometry() {
    let run = || {
        let mut effects = TireEffects::default();
        for n in 0..180 {
            effects.step([Some(contact(n as f32 * 0.1)); 4], Vec3::X * 6.0, FIXED_STEP);
        }
        vertices(&mut effects)
    };
    assert_eq!(run(), run());
}

#[test]
fn physics_outputs_drive_burnout_emission_and_quiet_rolling() {
    let ground = FlatGround::new(0.0);
    let mut car = Vehicle::new(VehicleSpec::example());
    assert!(car.place_on_ground(&ground, 0.0, 0.0, 5.0, 0.0));
    for _ in 0..180 {
        car.step(FIXED_STEP, &InputState::default(), &ground);
    }
    let mut effects = TireEffects::default();
    for _ in 0..300 {
        car.step(FIXED_STEP, &InputState { throttle: 1.0, ..InputState::default() }, &ground);
        let contacts = std::array::from_fn(|i| {
            let wheel = car.wheel(i);
            wheel.on_ground.then_some(Contact { skid: wheel.skid, smoke: wheel.smoke, ..contact(i as f32) })
        });
        effects.step(contacts, Vec3::ZERO, FIXED_STEP);
    }
    assert!(effects.smoke.emitted > 0 || effects.marks.created > 0, "existing physics produced no tire effects");
    assert_eq!(car.wheel(0).smoke, 0.0);
    assert_eq!(car.wheel(1).smoke, 0.0);
    effects.clear();
    car.place_moving(Vec3::new(0.0, 0.74, 0.0), glam::Quat::IDENTITY, 25.0);
    for _ in 0..30 {
        car.step(FIXED_STEP, &InputState { throttle: 0.3, ..InputState::default() }, &ground);
    }
    for _ in 0..180 {
        car.step(FIXED_STEP, &InputState { throttle: 0.4, ..InputState::default() }, &ground);
        let contacts = std::array::from_fn(|i| {
            let w = car.wheel(i);
            Some(Contact { skid: w.skid, smoke: w.smoke, ..contact(i as f32) })
        });
        effects.step(contacts, Vec3::ZERO, FIXED_STEP);
    }
    assert_eq!(effects.smoke.len(), 0);
    assert_eq!(effects.marks.len(), 0);
}

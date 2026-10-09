use glam::{Mat4, Vec3};

use crate::{Curve, Emitter, EmitterSpec, Frame, MAX_PARTICLES};

const STEP: f32 = 1.0 / 60.0;

fn spec() -> EmitterSpec {
    EmitterSpec { rate: 60.0, life: 0.5, speed: 4.0, ..Default::default() }
}

fn run(emitter: &mut Emitter, steps: usize, frame: &Frame) {
    for _ in 0..steps {
        emitter.step(STEP, frame);
    }
}

#[test]
fn the_curve_passes_through_its_keys() {
    let curve = Curve::new([0.0, 0.6, 0.8, 1.0], [0.35, 0.45, 0.1, 0.0]);
    for (k, v) in [(0.0, 0.35), (0.6, 0.45), (0.8, 0.1), (1.0, 0.0)] {
        assert!((curve.value(k) - v).abs() < 1e-5, "key {k}");
    }
}

#[test]
fn equal_keys_fall_back_to_straight_lines() {
    let curve = Curve::new([0.0, 0.5, 0.5, 1.0], [0.0, 1.0, 1.0, 0.0]);
    assert!((curve.value(0.25) - 0.5).abs() < 1e-5);
    assert!((curve.value(0.75) - 0.5).abs() < 1e-5);
    assert_eq!(curve.value(-1.0), 0.0);
    assert_eq!(curve.value(2.0), 0.0);
}

#[test]
fn a_rate_below_one_per_step_waits_for_a_whole_particle() {
    // 50 per second at 60 Hz owes 0.83 per step: the second step is the first to reach 1.
    let mut emitter = Emitter::new(EmitterSpec { rate: 50.0, ..spec() }, 1);
    emitter.step(STEP, &Frame::default());
    assert_eq!(emitter.len(), 0);
    emitter.step(STEP, &Frame::default());
    assert_eq!(emitter.len(), 1);
}

#[test]
fn a_whole_number_per_step_drops_the_fraction() {
    // 500 per second at 60 Hz is 8.33 per step: 8 spawn and the 0.33 is not carried over.
    let mut emitter = Emitter::new(EmitterSpec { rate: 500.0, life: 10.0, ..spec() }, 1);
    run(&mut emitter, 3, &Frame::default());
    assert_eq!(emitter.len(), 24);
}

#[test]
fn zero_intensity_and_a_disabled_emitter_spawn_nothing() {
    let mut emitter = Emitter::new(spec(), 1);
    run(&mut emitter, 10, &Frame { intensity: 0.0, ..Frame::default() });
    assert!(emitter.is_empty());
    emitter.set_enabled(false);
    run(&mut emitter, 10, &Frame::default());
    assert!(emitter.is_empty());
}

#[test]
fn particles_fly_along_the_local_z_axis() {
    let mut emitter = Emitter::new(EmitterSpec { speed_variance: 0.5, life: 5.0, ..spec() }, 7);
    let backwards = Mat4::from_cols(
        Vec3::Y.extend(0.0),
        Vec3::Z.extend(0.0),
        (-Vec3::X).extend(0.0),
        Vec3::new(10.0, 0.0, 1.0).extend(1.0),
    );
    run(&mut emitter, 30, &Frame { to_world: backwards, ..Frame::default() });
    assert!(!emitter.is_empty());
    for sprite in emitter.sprites() {
        // Born at x = 10 and moving towards -x only.
        assert!(sprite.position.x < 10.0 && (sprite.position.y).abs() < 1e-4 && (sprite.position.z - 1.0).abs() < 1e-4);
    }
}

#[test]
fn a_cone_spreads_the_direction() {
    let mut emitter = Emitter::new(EmitterSpec { spread: 40.0, rate: 600.0, life: 5.0, ..spec() }, 3);
    run(&mut emitter, 10, &Frame::default());
    let sideways = emitter.sprites().filter(|s| s.position.x.abs() > 1e-3 || s.position.y.abs() > 1e-3).count();
    assert!(sideways > emitter.len() / 2);
}

#[test]
fn live_motion_keeps_the_inherited_velocity_out() {
    let car = Frame { inherit_velocity: Vec3::new(30.0, 0.0, 0.0), ..Frame::default() };
    let spec = EmitterSpec { inherit: 1.0, speed: 0.0, velocity_start: Vec3::ZERO, life: 5.0, ..spec() };
    let mut inheriting = Emitter::new(spec.clone(), 1);
    let mut staying = Emitter::new(EmitterSpec { live_motion: true, ..spec }, 1);
    run(&mut inheriting, 6, &car);
    run(&mut staying, 6, &car);
    let x = |e: &Emitter| e.sprites().map(|s| s.position.x).fold(f32::MIN, f32::max);
    assert!(x(&inheriting) > 1.0);
    assert!(x(&staying).abs() < 1e-4);
}

#[test]
fn particles_die_at_the_end_of_their_life() {
    let mut emitter = Emitter::new(EmitterSpec { life: 0.2, rate: 60.0, ..spec() }, 1);
    run(&mut emitter, 60, &Frame::default());
    assert!(emitter.len() <= 12);
    emitter.set_enabled(false);
    run(&mut emitter, 13, &Frame::default());
    assert!(emitter.is_empty());
}

#[test]
fn the_curve_position_counts_down_the_nominal_life() {
    // A particle born with a quarter of the nominal life is already three quarters along the curve after
    // its first step; size runs 0 -> 4 over the nominal life, so the full width reads about 3.
    let spec = EmitterSpec {
        rate: 60.0,
        life: 1.0,
        life_variance: 0.75,
        speed: 0.0,
        size: [0.0, 4.0 / 3.0, 8.0 / 3.0, 4.0],
        ..Default::default()
    };
    let mut emitter = Emitter::new(spec, 5);
    for _ in 0..40 {
        emitter.step(STEP, &Frame::default());
    }
    let widest = emitter.sprites().map(|s| s.half_size * 2.0).fold(0.0, f32::max);
    assert!(widest > 2.0 && widest <= 4.0 + 1e-3, "{widest}");
}

#[test]
fn a_negative_gravity_lifts_and_drag_slows() {
    let mut up = Emitter::new(EmitterSpec { gravity: -2.0, speed: 0.0, life: 5.0, ..spec() }, 1);
    run(&mut up, 30, &Frame::default());
    assert!(up.sprites().all(|s| s.position.z > 0.0));

    let mut draggy = Emitter::new(EmitterSpec { drag: 1.0, life: 5.0, rate: 60.0, ..spec() }, 1);
    let mut free = Emitter::new(EmitterSpec { life: 5.0, rate: 60.0, ..spec() }, 1);
    run(&mut draggy, 30, &Frame::default());
    run(&mut free, 30, &Frame::default());
    let reach = |e: &Emitter| e.sprites().map(|s| s.position.z).fold(0.0, f32::max);
    assert!(reach(&draggy) < reach(&free));
}

#[test]
fn a_particle_whose_alpha_reaches_the_floor_dies_early() {
    let colors = [[255, 255, 255, 255], [255, 255, 255, 160], [255, 255, 255, 60], [255, 255, 255, 0]];
    let fading = EmitterSpec { colors, life: 1.0, speed: 0.0, ..spec() };
    let mut dying = Emitter::new(fading.clone(), 1);
    let mut lasting = Emitter::new(EmitterSpec { kill_alpha: None, ..fading }, 1);
    for _ in 0..120 {
        dying.step(STEP, &Frame::default());
        lasting.step(STEP, &Frame::default());
    }
    assert!(dying.len() < lasting.len());
}

#[test]
fn the_particle_count_is_capped() {
    let mut emitter = Emitter::new(EmitterSpec { rate: 100_000.0, life: 30.0, ..spec() }, 1);
    run(&mut emitter, 5, &Frame::default());
    assert_eq!(emitter.len(), MAX_PARTICLES);
}

#[test]
fn the_same_seed_gives_the_same_particles() {
    let spec = EmitterSpec { spread: 20.0, speed_variance: 0.3, volume_extent: Vec3::splat(0.2), ..spec() };
    let (mut a, mut b) = (Emitter::new(spec.clone(), 9), Emitter::new(spec, 9));
    run(&mut a, 20, &Frame::default());
    run(&mut b, 20, &Frame::default());
    assert!(a.sprites().zip(b.sprites()).all(|(x, y)| x == y));
    assert_eq!(a.len(), b.len());
}

#[test]
fn a_sprite_turns_about_the_view_direction() {
    let sprite =
        crate::Sprite { position: Vec3::ZERO, half_size: 1.0, angle: std::f32::consts::FRAC_PI_2, color: [255; 4] };
    // Looking along +x with right = -y... any right/up pair turns by a quarter turn.
    let corners = sprite.corners(Vec3::Y, Vec3::Z, Vec3::X);
    for c in corners {
        assert!(c.x.abs() < 1e-5 && (c.length() - 2f32.sqrt()).abs() < 1e-5);
    }
    let straight = crate::Sprite { angle: 0.0, ..sprite }.corners(Vec3::Y, Vec3::Z, Vec3::X);
    assert!((straight[0] - Vec3::new(0.0, -1.0, -1.0)).length() < 1e-5);
    assert!((corners[0] - straight[0]).length() > 1.0);
}

#[test]
fn a_lowered_limit_caps_the_particles_and_stepping_does_not_allocate() {
    let mut emitter = Emitter::new(EmitterSpec { rate: 100_000.0, life: 30.0, ..spec() }, 1);
    emitter.set_limit(40);
    assert_eq!(emitter.limit(), 40);
    let reserved = emitter.capacity();
    assert!(reserved >= 40);
    run(&mut emitter, 30, &Frame::default());
    assert_eq!(emitter.len(), 40);
    assert_eq!(emitter.capacity(), reserved, "the storage reserved by the limit is enough");
}

#[test]
fn the_limit_stays_between_one_and_the_hard_cap() {
    let mut emitter = Emitter::new(spec(), 1);
    emitter.set_limit(0);
    assert_eq!(emitter.limit(), 1);
    emitter.set_limit(usize::MAX);
    assert_eq!(emitter.limit(), MAX_PARTICLES);
}

#[test]
fn particles_beyond_a_lowered_limit_are_left_to_die() {
    let mut emitter = Emitter::new(EmitterSpec { rate: 6_000.0, life: 0.2, ..spec() }, 1);
    run(&mut emitter, 10, &Frame::default());
    let before = emitter.len();
    emitter.set_limit(5);
    assert_eq!(emitter.len(), before, "none are removed at once");
    emitter.set_enabled(false);
    run(&mut emitter, 30, &Frame::default());
    assert!(emitter.is_empty());
}

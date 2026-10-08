use blackbox_particles::EmitterSpec;
use blackbox_render::EffectLayer;
use glam::{Mat4, Vec3, Vec4};
use nfsmw_data::car::exhaust::{ExhaustFx, GroupEmitter};

use super::{CarState, ExhaustFlames, SpriteTexture};

const STEP: f32 = 1.0 / 60.0;

/// A pipe at the rear whose local z axis points backwards (the car's -x).
fn pipe(y: f32) -> Mat4 {
    Mat4::from_cols(
        Vec4::new(0.0, 1.0, 0.0, 0.0),
        Vec4::new(0.0, 0.0, 1.0, 0.0),
        Vec4::new(-1.0, 0.0, 0.0, 0.0),
        Vec4::new(-2.0, y, 0.3, 1.0),
    )
}

fn emitter(texture: u32) -> GroupEmitter {
    GroupEmitter {
        spec: EmitterSpec { rate: 300.0, life: 0.1, speed: 6.0, size: [0.4; 4], ..Default::default() },
        texture,
    }
}

fn flames(engine_upgrades: i32) -> ExhaustFlames {
    let fx = ExhaustFx {
        pipes: vec![pipe(0.3), pipe(-0.3)],
        shift_speed: 8.0,
        shift_angle: 2.25,
        engine_upgrades,
        nitrous: vec![emitter(1), emitter(2)],
    };
    let textures =
        vec![SpriteTexture { handle: None, additive: true }, SpriteTexture { handle: None, additive: false }];
    ExhaustFlames::new(fx, &[0, 1], textures)
}

fn car(gear: i32, nitrous: bool) -> CarState {
    CarState {
        to_world: Mat4::from_translation(Vec3::new(100.0, 0.0, 0.0)),
        velocity: Vec3::new(40.0, 0.0, 0.0),
        gear,
        nitrous,
    }
}

fn run(flames: &mut ExhaustFlames, steps: usize, car: &CarState) {
    for _ in 0..steps {
        flames.step(STEP, car);
    }
}

fn particles(flames: &ExhaustFlames) -> usize {
    flames.live()
}

#[test]
fn nothing_flames_by_itself() {
    let mut fx = flames(0);
    run(&mut fx, 60, &car(3, false));
    assert_eq!(particles(&fx), 0);
}

#[test]
fn the_nitrous_flames_while_it_burns_and_the_particles_die_after() {
    let mut fx = flames(3);
    run(&mut fx, 30, &car(3, true));
    assert!(particles(&fx) > 0);
    run(&mut fx, 30, &car(3, false));
    assert_eq!(particles(&fx), 0, "the flame dies with its short particles");
}

#[test]
fn particles_are_born_at_the_pipes_and_shot_backwards() {
    let mut fx = flames(0);
    run(&mut fx, 3, &car(3, true));
    let sprites: Vec<_> = fx.lanes[0].emitters.iter().flat_map(|e| e.sprites()).collect();
    assert!(!sprites.is_empty());
    // The car is at x = 100 and the pipes 2 m behind its origin; live motion is off in this spec, so the
    // particles also take the car's 40 m/s, but they were born behind the origin.
    assert!(sprites.iter().all(|s| s.position.x < 100.0 && s.position.x > 90.0));
    assert!(sprites.iter().any(|s| (s.position.y - 0.3).abs() < 0.1));
    assert!(sprites.iter().any(|s| (s.position.y + 0.3).abs() < 0.1));
}

#[test]
fn a_gear_change_flames_only_where_the_engine_allows_it() {
    // A car that cannot be upgraded flames at a shift from the start.
    let mut open = flames(0);
    run(&mut open, 5, &car(3, false));
    run(&mut open, 3, &car(4, false));
    assert!(particles(&open) > 0);

    // A stock engine of an upgradable car does not.
    let mut stock = flames(3);
    run(&mut stock, 5, &car(3, false));
    run(&mut stock, 3, &car(4, false));
    assert_eq!(particles(&stock), 0);

    // After an engine upgrade it does.
    stock.command(&["engine", "1"]).unwrap();
    run(&mut stock, 5, &car(4, false));
    run(&mut stock, 3, &car(5, false));
    assert!(particles(&stock) > 0);
}

#[test]
fn the_shift_flame_stops_after_the_pitch_time() {
    let mut fx = flames(0);
    run(&mut fx, 5, &car(3, false));
    run(&mut fx, 1, &car(4, false));
    let mut flaming = 0;
    for _ in 0..60 {
        fx.step(STEP, &car(4, false));
        flaming += usize::from(fx.flaming);
    }
    // 0.28 s at 60 Hz: 16 steps with the event running, the first of them the step of the change.
    assert_eq!(flaming + 1, 16);
}

#[test]
fn a_slow_shift_and_the_off_switch_do_not_flame() {
    let slow = CarState { velocity: Vec3::new(5.0, 0.0, 0.0), ..car(2, false) };
    let mut fx = flames(0);
    run(&mut fx, 5, &car(1, false));
    run(&mut fx, 3, &slow);
    assert_eq!(particles(&fx), 0);

    fx.command(&["off"]).unwrap();
    run(&mut fx, 20, &car(3, true));
    assert_eq!(particles(&fx), 0);
    fx.command(&["on"]).unwrap();
    run(&mut fx, 20, &car(3, true));
    assert!(particles(&fx) > 0);
}

#[test]
fn geometry_has_one_batch_per_texture_with_its_blend() {
    let mut fx = flames(0);
    run(&mut fx, 10, &car(3, true));
    let mut layer = EffectLayer::default();
    fx.geometry(Vec3::new(90.0, 0.0, 1.0), Vec3::X, &mut layer);
    assert_eq!(layer.sprites.len(), 2);
    assert!(layer.sprites[0].additive && !layer.sprites[1].additive);
    assert!(layer.sprites.iter().all(|b| !b.vertices.is_empty() && b.vertices.len() % 6 == 0));
    // The pass that draws them needs far-to-near order: the first quad is not nearer than the last.
    let depth = |v: &blackbox_render::EffectVertex| v.position[0];
    let batch = &layer.sprites[0].vertices;
    assert!(depth(&batch[0]) >= depth(&batch[batch.len() - 1]) - 1.0);
}

#[test]
fn aging_lets_the_flames_die_without_new_ones_and_clear_empties_them() {
    let mut fx = flames(0);
    run(&mut fx, 30, &car(3, true));
    assert!(particles(&fx) > 0);
    for _ in 0..20 {
        fx.age(STEP);
    }
    assert_eq!(particles(&fx), 0);
    run(&mut fx, 10, &car(3, true));
    fx.clear();
    assert_eq!(particles(&fx), 0);
}

#[test]
fn the_command_reports_and_validates() {
    let mut fx = flames(3);
    let status = fx.command(&["status"]).unwrap();
    assert!(
        status.contains("2 pipes") && status.contains("engine level 0 of 3") && status.contains("blow-off off"),
        "{status}"
    );
    assert!(fx.command(&["engine", "2"]).unwrap().contains("blow-off on"));
    assert!(fx.command(&["engine", "-1"]).is_err());
    assert!(fx.command(&["engine"]).is_err());
    assert!(fx.command(&["sparkle"]).is_err());
}

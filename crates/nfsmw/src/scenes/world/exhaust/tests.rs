use blackbox_particles::EmitterSpec;
use blackbox_render::{BlendMode, EffectLayer};
use glam::{Mat4, Vec3, Vec4};

use super::load::lane;
use super::trigger::{BACKFIRE_SECONDS, ShiftTiming};
use super::{Active, CarState, EMITTER_LIMIT, ExhaustFlames, SpriteTexture, State};

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

fn spec() -> EmitterSpec {
    EmitterSpec { rate: 300.0, life: 0.1, speed: 6.0, size: [0.4; 4], ..Default::default() }
}

/// Two pipes, two group emitters (one additive, one alpha-blended texture), the setting on.
fn flames(engine_upgrades: i32) -> ExhaustFlames {
    let textures = vec![
        SpriteTexture { handle: None, blend: BlendMode::Additive },
        SpriteTexture { handle: None, blend: BlendMode::AlphaBlend },
    ];
    let lanes = vec![lane(&spec(), 0, 2, 0), lane(&spec(), 1, 2, 1)];
    let timing = ShiftTiming { speed: 8.0, angle: 2.25 };
    let active = Active::new(vec![pipe(0.3), pipe(-0.3)], lanes, textures, timing, engine_upgrades);
    ExhaustFlames { enabled: true, state: State::Ready(Box::new(active)), ..ExhaustFlames::default() }
}

fn active(fx: &ExhaustFlames) -> &Active {
    fx.active().expect("loaded")
}

fn car(gear: i32, nitrous: bool) -> CarState {
    CarState {
        to_world: Mat4::from_translation(Vec3::new(100.0, 0.0, 0.0)),
        velocity: Vec3::new(40.0, 0.0, 0.0),
        gear,
        nitrous,
        throttle: 1.0,
    }
}

/// Coasting in `gear`: off the throttle.
fn lifting(gear: i32) -> CarState {
    CarState { throttle: 0.0, ..car(gear, false) }
}

fn run(flames: &mut ExhaustFlames, steps: usize, car: &CarState) {
    for _ in 0..steps {
        flames.step(STEP, car);
    }
}

#[test]
fn nothing_flames_by_itself() {
    let mut fx = flames(0);
    run(&mut fx, 60, &car(3, false));
    assert_eq!(fx.live(), 0);
}

#[test]
fn the_nitrous_flames_while_it_burns_and_the_particles_die_after() {
    let mut fx = flames(3);
    run(&mut fx, 30, &car(3, true));
    assert!(fx.live() > 0);
    run(&mut fx, 30, &car(3, false));
    assert_eq!(fx.live(), 0, "the flame dies with its short particles");
}

#[test]
fn particles_are_born_at_the_pipes_and_shot_backwards() {
    let mut fx = flames(0);
    run(&mut fx, 3, &car(3, true));
    let sprites: Vec<_> = active(&fx).lanes[0].emitters.iter().flat_map(|e| e.sprites()).collect();
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
    assert!(open.live() > 0);

    // A stock engine of an upgradable car does not.
    let mut stock = flames(3);
    run(&mut stock, 5, &car(3, false));
    run(&mut stock, 3, &car(4, false));
    assert_eq!(stock.live(), 0);

    // After an engine upgrade it does.
    stock.command(&["engine", "1"]).unwrap();
    run(&mut stock, 5, &car(4, false));
    run(&mut stock, 3, &car(5, false));
    assert!(stock.live() > 0);
}

#[test]
fn the_shift_flame_stops_after_the_pitch_time() {
    let mut fx = flames(0);
    run(&mut fx, 5, &car(3, false));
    run(&mut fx, 1, &car(4, false));
    let mut flaming = 0;
    for _ in 0..60 {
        fx.step(STEP, &car(4, false));
        flaming += usize::from(active(&fx).intensity > 0.0);
    }
    // 0.28 s at 60 Hz: 16 steps with the event running, the first of them the step of the change.
    assert_eq!(flaming + 1, 16);
}

#[test]
fn a_slow_shift_does_not_flame() {
    let slow = CarState { velocity: Vec3::new(5.0, 0.0, 0.0), ..car(2, false) };
    let mut fx = flames(0);
    run(&mut fx, 5, &car(1, false));
    run(&mut fx, 3, &slow);
    assert_eq!(fx.live(), 0);
}

#[test]
fn a_sputter_pop_off_the_throttle_makes_a_short_weaker_flame() {
    let mut fx = flames(3);
    run(&mut fx, 5, &lifting(3));
    fx.note_pops(1);
    fx.step(STEP, &lifting(3));
    assert!(fx.live() > 0, "the pop lights the pipes at once");
    let backfire = active(&fx).intensity;
    assert!(backfire > 0.0 && backfire < 1.0);
    let mut flaming = 1;
    for _ in 0..30 {
        fx.step(STEP, &lifting(3));
        flaming += usize::from(active(&fx).intensity > 0.0);
    }
    assert_eq!(flaming as f32, (BACKFIRE_SECONDS / STEP).ceil());
    assert_eq!(fx.live(), 0, "and it dies with its short particles");
}

#[test]
fn a_pop_with_the_foot_down_or_without_the_setting_makes_no_flame() {
    let mut fx = flames(3);
    fx.note_pops(3);
    run(&mut fx, 3, &car(3, false));
    assert_eq!(fx.live(), 0, "under power the shift rule alone decides");

    let mut off = ExhaustFlames::default();
    off.note_pops(3);
    off.step(STEP, &lifting(3));
    assert_eq!((off.pops, off.live()), (0, 0));
}

#[test]
fn the_pops_of_one_step_are_used_once() {
    let mut fx = flames(3);
    fx.note_pops(1);
    fx.step(STEP, &car(3, false));
    assert_eq!(fx.pops, 0);
    fx.step(STEP, &lifting(3));
    assert_eq!(fx.live(), 0, "the pop under power is gone; it does not wait for the lift-off");
}

#[test]
fn the_console_pop_backfires_whatever_the_throttle_does() {
    let mut fx = flames(3);
    fx.command(&["pop"]).unwrap();
    fx.step(STEP, &car(3, false));
    assert!(fx.live() > 0);
    assert!(ExhaustFlames::default().command(&["pop"]).is_err(), "off: nothing to pop");
}

#[test]
fn a_quiet_car_does_no_emitter_work() {
    let mut fx = flames(3);
    run(&mut fx, 10, &car(3, false));
    assert!(active(&fx).lanes.iter().flat_map(|l| &l.emitters).all(|e| e.enabled()));
    // No emitter was stepped or switched: they are still in their initial state with no storage touched.
    assert_eq!(active(&fx).intensity, 0.0);
    assert_eq!(fx.live(), 0);
}

#[test]
fn each_emitter_is_capped_and_its_storage_reserved_up_front() {
    let mut fx = flames(0);
    let heavy = EmitterSpec { rate: 1.0e6, life: 30.0, ..spec() };
    let State::Ready(active) = &mut fx.state else { unreachable!() };
    active.lanes = vec![lane(&heavy, 0, 2, 0)];
    let reserved: Vec<usize> = active.lanes[0].emitters.iter().map(|e| e.capacity()).collect();
    assert!(reserved.iter().all(|&c| c >= EMITTER_LIMIT));
    run(&mut fx, 20, &car(3, true));
    assert_eq!(fx.live(), 2 * EMITTER_LIMIT);
    let after: Vec<usize> = active_lanes(&fx).iter().map(|e| e.capacity()).collect();
    assert_eq!(after, reserved, "stepping never grew the storage");
}

fn active_lanes(fx: &ExhaustFlames) -> Vec<&blackbox_particles::Emitter> {
    active(fx).lanes.iter().flat_map(|l| &l.emitters).collect()
}

#[test]
fn quads_are_built_per_lane_far_to_near_and_reuse_their_buffer() {
    let mut fx = flames(0);
    run(&mut fx, 10, &car(3, true));
    let State::Ready(a) = &mut fx.state else { unreachable!() };
    for lane in 0..2 {
        assert!(a.quads(lane, Vec3::new(90.0, 0.0, 1.0), Vec3::X));
        let vertices = &a.lanes[lane].vertices;
        assert!(!vertices.is_empty() && vertices.len() % 6 == 0);
        // The pass that draws them needs far-to-near order: the first quad is not nearer than the last.
        let depth = |v: &blackbox_render::EffectVertex| v.position[0];
        assert!(depth(&vertices[0]) >= depth(&vertices[vertices.len() - 1]) - 1.0);
    }
    let capacity = a.lanes[0].vertices.capacity();
    assert!(a.quads(0, Vec3::new(90.0, 0.0, 1.0), Vec3::X));
    assert_eq!(a.lanes[0].vertices.capacity(), capacity);
    // Without a GPU texture a lane is built but not handed to the layer.
    let mut layer = EffectLayer::default();
    fx.geometry(Vec3::new(90.0, 0.0, 1.0), Vec3::X, &mut layer);
    assert!(layer.textured.is_empty());
}

#[test]
fn aging_lets_the_flames_die_without_new_ones_and_clear_empties_them() {
    let mut fx = flames(0);
    run(&mut fx, 30, &car(3, true));
    assert!(fx.live() > 0);
    for _ in 0..20 {
        fx.age(STEP);
    }
    assert_eq!(fx.live(), 0);
    run(&mut fx, 10, &car(3, true));
    fx.clear();
    assert_eq!(fx.live(), 0);
    assert_eq!(active(&fx).intensity, 0.0);
}

#[test]
fn turning_the_setting_off_unloads_and_queues_the_textures_for_release() {
    let mut fx = flames(0);
    assert!(!fx.wants_load(), "already loaded");
    fx.set_enabled(false);
    assert!(matches!(fx.state, State::Unloaded));
    assert!(!fx.wants_load(), "off: nothing to load");
    fx.set_enabled(true);
    assert!(fx.wants_load(), "on again: the next frame loads");
    assert_eq!(fx.live(), 0);
}

#[test]
fn a_default_instance_is_off_and_holds_nothing() {
    let fx = ExhaustFlames::default();
    assert!(!fx.enabled() && !fx.wants_load());
    assert!(fx.stale.is_empty() && fx.live() == 0);
    assert!(fx.status().contains("exhaust flames off"));
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
    assert!(fx.command(&["on"]).is_err(), "the setting switches the flames, not the command");
    assert!(fx.command(&["sparkle"]).is_err());
}

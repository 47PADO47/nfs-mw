use blackbox_modding::content::{FlameProfile, parse_nitrous_profile};

use super::*;

fn effect(on: EffectOn) -> EffectProfile {
    EffectProfile {
        on,
        texture: "NITROUS/X/A.PNG".into(),
        additive: true,
        grid: 0,
        fps: 24,
        random_start: false,
        rate: 200.0,
        rate_per_speed: 0.0,
        life: 0.5,
        life_variance: 0.0,
        speed: 1.0,
        speed_variance: 0.0,
        spread: 0.0,
        lift: 0.0,
        drag: 0.0,
        damping: 0.0,
        inherit: 0.0,
        inherit_variance: 0.0,
        angle_range: 0.0,
        keys: vec![0.0, 1.0 / 3.0, 2.0 / 3.0, 1.0],
        size: vec![0.1; 4],
        colors: vec![[255; 4]; 4],
        spin: 0.0,
        duration: 0.3,
    }
}

fn car(nitrous: bool, throttle: f32) -> CarState {
    CarState { to_world: Mat4::IDENTITY, velocity: Vec3::ZERO, gear: 3, throttle, nitrous }
}

/// A variant with the given effects loaded without a renderer: one emitter per pipe, no texture handle.
fn loaded(effects: Vec<EffectProfile>, flame: Option<FlameProfile>) -> Variant {
    let profile = Arc::new(NitrousProfile { flame, effects, ..NitrousProfile::default() });
    let lanes = profile
        .effects
        .iter()
        .enumerate()
        .map(|(index, e)| VariantLane {
            on: e.on,
            duration: e.duration,
            rate: e.rate,
            rate_per_speed: e.rate_per_speed,
            carried: carried_share(e),
            lane: lane(e, index, 2, 0),
        })
        .collect();
    let mut variant = Variant::default();
    variant.set_wanted(Some(profile.clone()));
    variant.loaded = Some(Loaded {
        profile,
        textures: vec![SpriteTexture { handle: None, blend: BlendMode::Additive }],
        lanes,
        start_t: f32::INFINITY,
        pop_t: f32::INFINITY,
        handed: Vec::new(),
    });
    variant
}

/// Run for `seconds`; a blow-off is off the throttle and anything else has the pedal down.
fn run(variant: &mut Variant, seconds: f32, nitrous: bool, pop: f32) {
    run_with(variant, seconds, &car(nitrous, if pop > 0.0 { 0.0 } else { 1.0 }), pop);
}

fn run_with(variant: &mut Variant, seconds: f32, car: &CarState, pop: f32) {
    let pipes = [Mat4::IDENTITY, Mat4::from_translation(Vec3::Y)];
    let steps = (seconds / 0.01).round() as usize;
    for _ in 0..steps {
        variant.step(0.01, car, &pipes, pop);
    }
}

fn alive(variant: &Variant, on: EffectOn) -> usize {
    let loaded = variant.loaded.as_ref().unwrap();
    loaded.lanes.iter().filter(|l| l.on == on).flat_map(|l| l.lane.emitters.iter()).map(Emitter::len).sum()
}

#[test]
fn the_profiles_numbers_become_the_games_emitter_parameters() {
    let mut e = effect(EffectOn::Burn);
    e.grid = 4;
    e.lift = 2.0;
    e.life_variance = 0.1;
    e.size = vec![0.1, 0.2, 0.3, 0.4];
    let spec = emitter_spec(&e);
    assert_eq!((spec.animation.grid, spec.animation.fps), (4, 24));
    assert_eq!((spec.rate, spec.life, spec.life_variance, spec.speed), (200.0, 0.5, 0.1, 1.0));
    assert_eq!(spec.gravity, -2.0, "a lift is the opposite of the emitter's gravity");
    let look: Vec<(f32, f32)> = spec.track.iter().map(|key| (key.t, key.size)).collect();
    assert_eq!(look, [(0.0, 0.1), (1.0 / 3.0, 0.2), (2.0 / 3.0, 0.3), (1.0, 0.4)]);
}

#[test]
fn a_variant_loads_when_the_profile_in_effect_changes_and_not_without_effects() {
    let mut variant = Variant::default();
    assert!(!variant.needs_load());
    variant.set_wanted(Some(Arc::new(NitrousProfile::default())));
    assert!(!variant.needs_load(), "a profile with no effect is no variant");
    let with_effect = loaded(vec![effect(EffectOn::Burn)], None);
    assert!(!with_effect.needs_load(), "what is loaded is what is wanted");
    let mut dropped = with_effect;
    dropped.set_wanted(None);
    assert!(dropped.needs_load(), "the variant was unchosen: drop it");
    let mut changed = loaded(vec![effect(EffectOn::Burn)], None);
    changed.set_wanted(Some(Arc::new(NitrousProfile { effects: vec![effect(EffectOn::Pop)], ..Default::default() })));
    assert!(changed.needs_load());
    let parsed = parse_nitrous_profile("format = 1\n[[effect]]\non = \"pop\"\ntexture = \"A.PNG\"").unwrap();
    changed.set_wanted(Some(Arc::new(parsed)));
    assert!(changed.needs_load());
}

#[test]
fn a_burn_effect_plays_while_the_nitrous_burns_and_dies_out_after() {
    let mut variant = loaded(vec![effect(EffectOn::Burn)], None);
    run(&mut variant, 0.2, false, 0.0);
    assert_eq!(alive(&variant, EffectOn::Burn), 0);
    run(&mut variant, 0.2, true, 0.0);
    assert!(alive(&variant, EffectOn::Burn) > 0);
    run(&mut variant, 1.0, false, 0.0);
    assert_eq!(alive(&variant, EffectOn::Burn), 0, "the particles live out their life");
    assert!(!variant.live());
}

#[test]
fn a_start_effect_plays_for_its_duration_after_the_nitrous_lights() {
    let mut variant = loaded(vec![effect(EffectOn::Start)], None);
    run(&mut variant, 0.1, true, 0.0);
    let early = alive(&variant, EffectOn::Start);
    assert!(early > 0);
    // Well past the 0.3 s duration, with the nitrous still burning: no new particles are born.
    run(&mut variant, 1.0, true, 0.0);
    assert_eq!(alive(&variant, EffectOn::Start), 0);
    // It lights again after a pause: the burst plays again.
    run(&mut variant, 0.2, false, 0.0);
    run(&mut variant, 0.1, true, 0.0);
    assert!(alive(&variant, EffectOn::Start) > 0);
}

#[test]
fn a_pop_effect_follows_the_games_blow_off_and_not_the_nitrous() {
    let mut variant = loaded(vec![effect(EffectOn::Pop)], None);
    run(&mut variant, 0.1, false, 0.0);
    assert_eq!(alive(&variant, EffectOn::Pop), 0);
    run(&mut variant, 0.1, false, 1.0);
    assert!(alive(&variant, EffectOn::Pop) > 0);
    run(&mut variant, 1.0, true, 1.0);
    assert_eq!(alive(&variant, EffectOn::Pop), 0, "the burning nitrous takes priority, as with the game's own flames");
}

#[test]
fn the_games_flames_are_replaced_only_when_the_profile_asks_and_has_such_an_effect() {
    let both = FlameProfile { tint: [1.0; 3], gain: 1.0, replace_burn: true, replace_pop: true };
    let variant = loaded(vec![effect(EffectOn::Burn)], Some(both));
    assert!(variant.replaces_burn());
    assert!(!variant.replaces_pop(), "no pop effect: the game's blow-off stays");
    let none = loaded(vec![effect(EffectOn::Burn), effect(EffectOn::Pop)], None);
    assert!(!none.replaces_burn() && !none.replaces_pop(), "without the flags the effects play on top");
    assert!(!Variant::default().replaces_burn());
}

#[test]
fn nothing_is_drawn_without_a_variant_and_dropping_one_frees_its_textures() {
    let mut empty = Variant::default();
    let mut layer = EffectLayer::default();
    empty.geometry(Vec3::ZERO, Vec3::X, &mut layer);
    assert!(layer.textured.is_empty());
    let mut variant = loaded(vec![effect(EffectOn::Burn)], None);
    variant.unload();
    assert!(variant.loaded.is_none());
    assert!(!variant.live());
}

#[test]
fn the_spawn_rate_grows_with_the_cars_speed_through_the_emitters_intensity() {
    assert_eq!(spawn_intensity(5.0, 1.0, 0.0), 1.0, "at rest: the base rate");
    assert_eq!(spawn_intensity(5.0, 1.0, 20.0), 5.0, "5 + 1 * 20 = 25 a second is five times the base rate of 5");
    assert_eq!(spawn_intensity(0.0, 2.0, 30.0), 30.0, "no rate at rest: the emitter's rate is per metre a second");
    let mut e = effect(EffectOn::Burn);
    e.rate = 0.0;
    e.rate_per_speed = 2.0;
    assert_eq!(emitter_spec(&e).rate, 2.0);
}

#[test]
fn a_pop_effect_with_a_duration_ends_before_the_pop_does_and_one_without_follows_it() {
    let mut short = effect(EffectOn::Pop);
    short.duration = 0.1;
    let mut variant = loaded(vec![short], None);
    run(&mut variant, 0.05, false, 1.0);
    assert!(alive(&variant, EffectOn::Pop) > 0);
    run(&mut variant, 1.0, false, 1.0);
    assert_eq!(alive(&variant, EffectOn::Pop), 0, "the burst is over though the pop goes on");
    let mut open = effect(EffectOn::Pop);
    open.duration = 0.0;
    let mut variant = loaded(vec![open], None);
    run(&mut variant, 0.6, false, 1.0);
    assert!(alive(&variant, EffectOn::Pop) > 0, "no duration: for as long as the pop lasts");
}

#[test]
fn the_profiles_keys_angle_and_variances_reach_the_emitter() {
    let mut e = effect(EffectOn::Burn);
    e.keys = vec![0.0, 0.05, 0.3, 1.0];
    e.angle_range = 360.0;
    e.inherit = 0.9;
    e.inherit_variance = 0.1;
    e.life_variance = 0.5;
    let spec = emitter_spec(&e);
    assert_eq!(spec.track.iter().map(|key| key.t).collect::<Vec<_>>(), [0.0, 0.05, 0.3, 1.0]);
    assert_eq!(
        (spec.initial_angle_range, spec.inherit, spec.inherit_variance, spec.life_variance),
        (360.0, 0.9, 0.1, 0.5)
    );
}

#[test]
fn a_pop_effect_does_not_play_with_the_pedal_down() {
    let mut variant = loaded(vec![effect(EffectOn::Pop)], None);
    run_with(&mut variant, 0.3, &car(false, 1.0), 1.0);
    assert_eq!(alive(&variant, EffectOn::Pop), 0, "the 2012 pop bit needs the pedal bit clear");
    run_with(&mut variant, 0.1, &car(false, 0.0), 1.0);
    assert!(alive(&variant, EffectOn::Pop) > 0, "off the throttle it plays");
}

#[test]
fn the_damping_of_the_profile_reaches_the_emitter() {
    let mut e = effect(EffectOn::Burn);
    e.damping = 0.75;
    assert_eq!(emitter_spec(&e).damping, 0.75);
}

/// The nitrous of the car as the game's own model runs it, one step of 10 ms: the state the effects are gated by.
struct Tank {
    nos: blackbox_vehicle::nos::Nos,
    spec: blackbox_vehicle::nos::NosSpec,
}

impl Tank {
    fn new(seconds_of_burn: f32) -> Self {
        let spec = blackbox_vehicle::nos::NosSpec {
            nos_capacity: seconds_of_burn,
            torque_boost: 0.3,
            nos_disengage: 0.5,
            recharge_min: 20.0,
            recharge_max: 10.0,
            recharge_min_speed: 10.0,
            recharge_max_speed: 60.0,
        };
        Self { nos: blackbox_vehicle::nos::Nos::new(&spec), spec }
    }

    /// Hold or release the button for `seconds`, with the car at speed in gear, and feed the effects what the car does.
    fn hold(&mut self, variant: &mut Variant, held: bool, seconds: f32, recharge: bool) {
        let pipes = [Mat4::IDENTITY, Mat4::from_translation(Vec3::Y)];
        for _ in 0..(seconds / 0.01).round() as usize {
            self.nos.update(
                &self.spec,
                &blackbox_vehicle::nos::NosInput {
                    dt: 0.01,
                    held,
                    gear: 3,
                    throttle: 1.0,
                    speed_mph: 60.0,
                    blown: false,
                    tuning: 0.0,
                    can_recharge: recharge,
                },
            );
            variant.step(0.01, &car(self.nos.boost > 1.0, 1.0), &pipes, 0.0);
        }
    }
}

#[test]
fn the_effects_follow_the_nitrous_through_burning_running_dry_and_refilling() {
    let mut variant = loaded(vec![effect(EffectOn::Start), effect(EffectOn::Burn)], None);
    let mut tank = Tank::new(1.0);
    // Idle: nothing.
    tank.hold(&mut variant, false, 0.3, false);
    assert!(!variant.live());
    // Activate: the start burst and the burn begin together.
    tank.hold(&mut variant, true, 0.1, false);
    assert!(alive(&variant, EffectOn::Start) > 0 && alive(&variant, EffectOn::Burn) > 0);
    // Burn on: the burst is over after its duration, the burn goes on.
    tank.hold(&mut variant, true, 0.75, false);
    assert_eq!(alive(&variant, EffectOn::Start), 0, "only the sustained effect is left");
    assert!(alive(&variant, EffectOn::Burn) > 0);
    // Run out with the button held: the burn stops and nothing else starts, however long the button stays down.
    tank.hold(&mut variant, true, 1.5, false);
    assert!(tank.nos.capacity == 0.0 && tank.nos.boost == 1.0, "the tank is dry");
    tank.hold(&mut variant, true, 2.0, false);
    assert!(!variant.live(), "no long flames with an empty tank and the button held");
    // Release, and press again with the tank still empty: still nothing, and no new burst.
    tank.hold(&mut variant, false, 0.5, false);
    tank.hold(&mut variant, true, 2.0, false);
    assert!(!variant.live());
    // Refill (the game recharges it while the car drives): the next press lights it again, with its burst.
    tank.hold(&mut variant, false, 1.0, true);
    tank.nos.capacity = 1.0;
    tank.hold(&mut variant, true, 0.1, false);
    assert!(alive(&variant, EffectOn::Start) > 0 && alive(&variant, EffectOn::Burn) > 0);
    // Release: the particles live out their short life.
    tank.hold(&mut variant, false, 1.5, false);
    assert!(!variant.live());
}

/// Drive a car forward at 40 m/s with one pipe at its rear (the frame's z axis points backwards, as the tail pipe markers do)
/// and return, in the car's frame, where the effect's particles are after `seconds`.
fn behind_the_car(effect: EffectProfile, seconds: f32) -> Vec<f32> {
    let on = effect.on;
    let mut variant = loaded(vec![effect], None);
    let rear = Mat4::from_cols(
        Vec3::Y.extend(0.0),
        Vec3::Z.extend(0.0),
        (-Vec3::X).extend(0.0),
        Vec3::new(-2.0, 0.0, 0.4).extend(1.0),
    );
    let mut x = 0.0;
    for _ in 0..(seconds / 0.01).round() as usize {
        x += 0.4;
        let state = CarState {
            to_world: Mat4::from_translation(Vec3::new(x, 0.0, 0.0)),
            velocity: Vec3::new(40.0, 0.0, 0.0),
            gear: 3,
            throttle: 1.0,
            nitrous: true,
        };
        variant.step(0.01, &state, &[rear], 0.0);
    }
    let lane = variant.loaded.as_ref().unwrap().lanes.iter().find(|lane| lane.on == on).unwrap();
    // The pipe's own place now is x - 2 in the world.
    lane.lane.emitters[0].sprites().map(|sprite| sprite.position.x - (x - 2.0)).collect()
}

/// The numbers decoded from the 2012 glow (inherit 0.9, a damping of 0.75, a lift of 0.5): the particles are born at the pipe
/// with the car's speed and stop at once, so the car drives away and leaves them behind, a trail out of the rear; undamped they
/// would ride along on the pipe.
#[test]
fn a_glow_that_is_damped_is_left_behind_the_moving_car_and_an_undamped_one_stays_on_it() {
    let mut glow = effect(EffectOn::Burn);
    (glow.rate, glow.life, glow.speed, glow.inherit, glow.lift, glow.duration) = (100.0, 0.4, 0.0, 0.9, 0.5, 0.0);
    glow.damping = 0.75;
    let trail = behind_the_car(glow.clone(), 0.6);
    assert!(trail.len() > 10);
    let oldest = trail.iter().copied().fold(f32::MAX, f32::min);
    assert!(oldest < -10.0, "the first of them is metres behind the pipe: {oldest}");
    assert!(trail.iter().all(|x| *x < 0.5), "none is ahead of the pipe, none is inside the car");
    glow.damping = 0.0;
    let carried = behind_the_car(glow, 0.6);
    let farthest = carried.iter().copied().fold(f32::MAX, f32::min);
    assert!(farthest > -3.0, "without the damping they drift along with the car: {farthest}");
}

/// A flame whose particles follow the emitter (all of the car's velocity, no damping) stays at the pipe.
#[test]
fn a_flame_that_follows_its_emitter_stays_at_the_pipe() {
    let mut flame = effect(EffectOn::Burn);
    (flame.rate, flame.life, flame.speed, flame.inherit, flame.duration) = (100.0, 0.2, 0.0, 1.0, 0.0);
    let found = behind_the_car(flame, 0.5);
    assert!(found.len() > 5);
    assert!(found.iter().all(|x| x.abs() < 0.2), "{found:?}");
}

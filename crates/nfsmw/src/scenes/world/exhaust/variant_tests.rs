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
        life: 0.5,
        life_variance: 0.0,
        speed: 1.0,
        speed_variance: 0.0,
        spread: 0.0,
        lift: 0.0,
        drag: 0.0,
        inherit: 0.0,
        size: [0.1; 4],
        colors: [[255; 4]; 4],
        duration: 0.3,
    }
}

fn car(nitrous: bool) -> CarState {
    CarState { to_world: Mat4::IDENTITY, velocity: Vec3::ZERO, gear: 3, throttle: 1.0, nitrous }
}

/// A variant with the given effects loaded without a renderer: one emitter per pipe, no texture handle.
fn loaded(effects: Vec<EffectProfile>, flame: Option<FlameProfile>) -> Variant {
    let profile = Arc::new(NitrousProfile { flame, effects, ..NitrousProfile::default() });
    let lanes = profile
        .effects
        .iter()
        .enumerate()
        .map(|(index, e)| VariantLane { on: e.on, duration: e.duration, lane: lane(e, index, 2, 0) })
        .collect();
    let mut variant = Variant::default();
    variant.set_wanted(Some(profile.clone()));
    variant.loaded = Some(Loaded {
        profile,
        textures: vec![SpriteTexture { handle: None, blend: BlendMode::Additive }],
        lanes,
        start_left: 0.0,
        burning: false,
        handed: Vec::new(),
    });
    variant
}

fn run(variant: &mut Variant, seconds: f32, nitrous: bool, pop: f32) {
    let pipes = [Mat4::IDENTITY, Mat4::from_translation(Vec3::Y)];
    let steps = (seconds / 0.01).round() as usize;
    for _ in 0..steps {
        variant.step(0.01, &car(nitrous), &pipes, pop);
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
    e.size = [0.1, 0.2, 0.3, 0.4];
    let spec = emitter_spec(&e);
    assert_eq!((spec.animation.grid, spec.animation.fps), (4, 24));
    assert_eq!((spec.rate, spec.life, spec.life_variance, spec.speed), (200.0, 0.5, 0.1, 1.0));
    assert_eq!(spec.gravity, -2.0, "a lift is the opposite of the emitter's gravity");
    assert_eq!(spec.size, [0.1, 0.2, 0.3, 0.4]);
    assert_eq!(spec.keys, EmitterSpec::default().keys);
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

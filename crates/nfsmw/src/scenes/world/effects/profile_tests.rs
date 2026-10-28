use blackbox_modding::content::parse_tire_smoke_profile;
use blackbox_render::EffectLayer;

use super::*;
use crate::settings::{SmokeColor, SmokeColorMode};

const PROFILE: &str = r#"
format = 1
[[layer]]
texture = "TIRESMOKE/X/PUFF.PNG"
blend = "alpha"
grid = 2
random_start = true
fps = 0
rate = 60.0
rate_per_speed = 0.0
life = 0.8
speed = 0.5
spread = 60.0
size = [0.5, 1.5, 2.5]
keys = [0.0, 0.3, 1.0]
colors = [[200, 205, 220, 0], [200, 205, 220, 200], [150, 155, 170, 0]]
[[layer]]
texture = "TIRESMOKE/X/PUFF.PNG"
blend = "alpha"
rate = 20.0
life = 1.5
"#;

fn contact(x: f32, smoke: f32) -> Contact {
    Contact {
        point: Vec3::new(x, 0.0, 5.0),
        normal: Vec3::Z,
        forward: Vec3::X,
        section: 1,
        skid: 1.0,
        smoke,
        width: 0.24,
    }
}

/// A variant with the profile loaded without a renderer: both layers share the one picture, whose handle is made up.
fn loaded() -> SmokeVariant {
    let profile = Arc::new(parse_tire_smoke_profile(PROFILE).unwrap());
    let mut variant = SmokeVariant::default();
    variant.set_wanted(Some(profile.clone()));
    let layers = make_layers(&profile, &[0, 0]);
    let textures = vec![Some((TextureHandle::from_raw(3), BlendMode::AlphaBlend))];
    variant.loaded = Some(Loaded { profile, textures, layers, handed: Vec::new() });
    variant
}

const OFF: SmokeTint = SmokeTint { mode: SmokeColorMode::Off, color: SmokeColor([255, 0, 0]) };
const STEP: f32 = 1.0 / 120.0;

fn run(variant: &mut SmokeVariant, contacts: [Option<Contact>; WHEELS], seconds: f32) {
    for _ in 0..(seconds / STEP) as usize {
        variant.step(&contacts, true, Vec3::new(20.0, 0.0, 0.0), OFF, STEP);
    }
}

#[test]
fn nothing_is_drawn_until_a_profile_is_loaded_and_the_game_smoke_is_used_meanwhile() {
    let mut variant = SmokeVariant::default();
    assert!(!variant.active() && !variant.needs_load());
    variant.set_wanted(Some(Arc::new(parse_tire_smoke_profile(PROFILE).unwrap())));
    assert!(variant.needs_load(), "a profile is wanted and not loaded");
    assert!(!variant.active(), "so the game's own smoke is still drawn");
    let mut layer = EffectLayer::default();
    variant.geometry(Vec3::ZERO, Vec3::X, OFF, &mut layer);
    assert!(layer.textured.is_empty());
    let loaded = loaded();
    assert!(loaded.active() && !loaded.needs_load());
}

#[test]
fn a_new_read_of_the_same_profile_is_not_a_reload_but_another_profile_is() {
    let mut variant = loaded();
    variant.set_wanted(Some(Arc::new(parse_tire_smoke_profile(PROFILE).unwrap())));
    assert!(!variant.needs_load(), "the same content");
    variant
        .set_wanted(Some(Arc::new(parse_tire_smoke_profile(&PROFILE.replace("rate = 60.0", "rate = 61.0")).unwrap())));
    assert!(variant.needs_load());
    variant.set_wanted(None);
    assert!(variant.needs_load(), "back to the game's smoke: unload");
    variant.unload();
    assert!(!variant.needs_load() && !variant.active());
}

#[test]
fn a_wheel_that_smokes_spawns_from_every_layer_and_a_wheel_in_the_air_does_not() {
    let mut variant = loaded();
    run(&mut variant, [Some(contact(0.0, 1.0)), Some(contact(2.0, 0.0)), None, None], 0.5);
    let live = variant.len();
    assert!(live > 20, "60 and 20 a second for half a second: {live}");
    let loaded = variant.loaded.as_ref().unwrap();
    assert!(loaded.layers.iter().all(|layer| !layer.emitters[0].is_empty()), "the wheel that smokes");
    assert!(loaded.layers.iter().all(|layer| layer.emitters[1].is_empty() && layer.emitters[3].is_empty()));
}

#[test]
fn the_wheels_smoke_scales_the_rate() {
    let (mut heavy, mut light) = (loaded(), loaded());
    run(&mut heavy, [Some(contact(0.0, 1.0)), None, None, None], 0.4);
    run(&mut light, [Some(contact(0.0, 0.25)), None, None, None], 0.4);
    assert!(heavy.len() > 2 * light.len(), "{} against {}", heavy.len(), light.len());
}

#[test]
fn smoke_stops_when_the_effect_is_off_and_dies_out() {
    let mut variant = loaded();
    run(&mut variant, [Some(contact(0.0, 1.0)), None, None, None], 0.5);
    assert!(variant.len() > 0);
    for _ in 0..(3.0 / STEP) as usize {
        variant.step(&[Some(contact(0.0, 1.0)), None, None, None], false, Vec3::ZERO, OFF, STEP);
    }
    assert_eq!(variant.len(), 0, "no smoke is spawned when it is off, and what there was is gone after its life");
}

#[test]
fn parked_smoke_ages_without_spawning_and_clear_removes_it() {
    let mut variant = loaded();
    run(&mut variant, [Some(contact(0.0, 1.0)), None, None, None], 0.3);
    let before = variant.len();
    variant.age(0.05);
    assert!(variant.len() <= before, "ageing never adds");
    variant.clear();
    assert_eq!(variant.len(), 0);
}

#[test]
fn the_layers_become_textured_batches_that_come_back_for_the_next_frame() {
    let mut variant = loaded();
    run(&mut variant, [Some(contact(0.0, 1.0)), None, None, None], 0.4);
    let mut layer = EffectLayer::default();
    variant.geometry(Vec3::new(-8.0, 0.0, 8.0), Vec3::X, OFF, &mut layer);
    assert_eq!(layer.textured.len(), 2, "one batch for each layer with particles");
    assert!(
        layer.textured.iter().all(|batch| batch.texture == TextureHandle::from_raw(3) && !batch.vertices.is_empty())
    );
    assert!(layer.textured.iter().all(|batch| batch.vertices.len() % 6 == 0), "six vertices a quad");
    variant.reclaim(&mut layer);
    let loaded = variant.loaded.as_ref().unwrap();
    assert!(loaded.layers.iter().all(|layer| layer.vertices.capacity() > 0), "the buffers are kept for the next frame");
    assert!(loaded.handed.is_empty());
}

#[test]
fn the_colour_mode_recolours_the_sprites_and_keeps_their_alpha() {
    let mut variant = loaded();
    run(&mut variant, [Some(contact(0.0, 1.0)), None, None, None], 0.4);
    let tinted = SmokeTint { mode: SmokeColorMode::Solid, color: SmokeColor([10, 200, 30]) };
    let (mut plain, mut coloured) = (EffectLayer::default(), EffectLayer::default());
    variant.geometry(Vec3::new(-8.0, 0.0, 8.0), Vec3::X, OFF, &mut plain);
    let a = plain.textured[0].vertices.clone();
    variant.reclaim(&mut plain);
    variant.geometry(Vec3::new(-8.0, 0.0, 8.0), Vec3::X, tinted, &mut coloured);
    let b = &coloured.textured[0].vertices;
    assert_eq!(a.len(), b.len());
    assert!(b.iter().all(|vertex| vertex.color[..3] == [10, 200, 30]), "every sprite is the chosen colour");
    assert!(a.iter().zip(b.iter()).all(|(x, y)| x.color[3] == y.color[3]), "the alpha curve is the profile's");
    assert!(
        a.iter().any(|vertex| vertex.color[..3] != [10, 200, 30]),
        "and without the mode it is the profile's colour"
    );
}

#[test]
fn the_emitter_frame_has_the_road_normal_as_its_axis_and_sits_just_above_the_road() {
    let frame = road_frame(&Contact { normal: Vec3::Z, forward: Vec3::Y, ..contact(3.0, 1.0) });
    assert_eq!(frame.z_axis.truncate(), Vec3::Z);
    assert_eq!(frame.x_axis.truncate(), Vec3::Y);
    assert!((frame.y_axis.truncate() - Vec3::Z.cross(Vec3::Y)).length() < 1e-6, "a right-handed frame");
    assert!((frame.w_axis.truncate() - Vec3::new(3.0, 0.0, 5.0 + LIFT)).length() < 1e-6);
}

use blackbox_modding::content::{EffectOn, parse_nitrous_profile};

use super::*;

fn picture(width: u32) -> Picture {
    Picture { width, height: width, rgba: vec![0; (width * width * 4) as usize] }
}

fn emitter(width: u32, flipbook: bool, colour: Option<[f32; 3]>) -> Emitter {
    Emitter { picture: picture(width), flipbook, colour }
}

fn profile(set: &EffectSet) -> blackbox_modding::content::NitrousProfile {
    let text =
        format!("format = 1\n[flame]\nreplace_burn = {}\nreplace_pop = {}\n{}", set.has_burn, set.has_pop, set.toml);
    parse_nitrous_profile(&text).unwrap()
}

/// The three effects as the real install has them: smoke puffs, two flames and dust; a glow; the same for the pop.
fn real_shaped() -> EffectSet {
    let smoke = emitter(128, false, None);
    let start =
        [smoke.clone(), smoke.clone(), emitter(512, true, None), emitter(128, true, None), emitter(512, true, None)];
    let burn = [emitter(256, false, Some([0.0307, 0.521, 0.863]))];
    let pop = [smoke, emitter(512, true, None), emitter(128, true, None), emitter(512, true, None)];
    build(&start, &burn, &pop).unwrap()
}

#[test]
fn the_effects_of_the_three_moments_parse_with_the_games_own_reader() {
    let set = real_shaped();
    let profile = profile(&set);
    let on = |moment: EffectOn| profile.effects.iter().filter(|e| e.on == moment).count();
    assert_eq!(
        (on(EffectOn::Start), on(EffectOn::Burn), on(EffectOn::Pop)),
        (3, 2, 3),
        "two flames and dust, two glows"
    );
    assert!(set.has_burn && set.has_pop);
    let flame = profile.effects.iter().find(|e| e.on == EffectOn::Start).unwrap();
    assert_eq!((flame.grid, flame.texture.as_str()), (5, "NITROUS/MW2012/FLAME.PNG"));
    assert!(flame.additive && flame.duration == 0.8);
    assert!(flame.fps as f32 * flame.life >= 24.0, "the sheet's 25 frames play over a particle's life");
}

#[test]
fn the_glow_takes_the_colour_read_from_the_effect_and_stays_on_the_pipe() {
    let profile = profile(&real_shaped());
    let halo = profile.effects.iter().find(|e| e.on == EffectOn::Burn).unwrap();
    assert_eq!(halo.colors[1], [8, 133, 220, 230]);
    assert_eq!(halo.colors[0][3], 0, "it fades in from nothing");
    assert_eq!((halo.speed, halo.inherit), (0.0, 1.0));
    let core = profile.effects.iter().filter(|e| e.on == EffectOn::Burn).nth(1).unwrap();
    assert!(core.colors[1][0] > halo.colors[1][0], "the core is lighter");
    assert!(core.size[1] < halo.size[1], "and smaller");
}

#[test]
fn each_picture_is_written_once_and_the_smoke_is_left_out() {
    let set = real_shaped();
    let names: Vec<&str> = set.pictures.iter().map(|(name, _)| *name).collect();
    assert_eq!(names, ["FLAME.PNG", "DUST.PNG", "GLOW.PNG"]);
    assert!(!set.toml.contains("SMOKE"));
}

#[test]
fn an_effect_with_nothing_to_play_gives_nothing() {
    assert!(build(&[emitter(128, false, None)], &[], &[emitter(128, false, None)]).is_none());
    let burn_only = build(&[], &[emitter(256, false, None)], &[]).unwrap();
    assert!(burn_only.has_burn && !burn_only.has_pop, "a car with no pop effect keeps the game's blow-off");
    assert!(profile(&burn_only).effects.iter().all(|e| e.on == EffectOn::Burn));
}

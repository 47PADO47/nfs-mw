use blackbox_modding::content::parse_skid_marks_profile;
use blackbox_vehicle::FIXED_STEP;

use super::*;
use crate::scenes::world::effects::{Contact, TireEffects};

const PROFILE: &str = r#"
format = 1
[[set]]
surface = "paved"
slip = "SKIDMARKS/X/SLIP.PNG"
spin = "SKIDMARKS/X/SPIN.PNG"
slip_tile = 2.0
spin_tile = 1.0
width = 2.0
life = 20.0
[[set]]
surface = "paved"
wet = true
spin = "SKIDMARKS/X/WET.PNG"
"#;

fn contact(x: f32, slide: f32, loose: bool) -> Contact {
    Contact {
        point: Vec3::new(x, 0.0, 5.0),
        normal: Vec3::Z,
        forward: Vec3::X,
        section: 7,
        skid: 1.0,
        smoke: 0.0,
        width: 0.25,
        loose,
        slide,
    }
}

/// Effects with the profile loaded without a renderer: the pictures' handles are made up (slip, spin, wet).
fn with_variant() -> TireEffects {
    let profile = Arc::new(parse_skid_marks_profile(PROFILE).unwrap());
    let mut effects = TireEffects::default();
    effects.set_marks_variant(Some(profile.clone()));
    let textures = (1..=3).map(|n| Some(TextureHandle::from_raw(n))).collect();
    effects.mark_variant.loaded = Some(Loaded {
        profile,
        textures,
        sets: vec![Pictures { slip: Some(0), spin: Some(1) }, Pictures { slip: Some(2), spin: Some(2) }],
        batches: vec![Vec::new(); 3],
        handed: Vec::new(),
    });
    effects
}

fn drive(effects: &mut TireEffects, slide: f32, loose: bool, wet: bool) {
    effects.set_wet(wet);
    for n in 0..60 {
        effects.step([Some(contact(n as f32 * 0.1, slide, loose)), None, None, None], Vec3::X, FIXED_STEP);
    }
}

#[test]
fn a_set_picks_the_picture_of_the_kind_of_slide_and_the_weather() {
    let mut effects = with_variant();
    drive(&mut effects, 1.0, false, false);
    let mark = effects.marks.iter().last().unwrap();
    assert!(mark.slip && effects.mark_variant.covers(mark));
    assert_eq!(
        effects.mark_variant.pictures_of(mark).unwrap().0,
        0,
        "a sideways slide on dry paved road: the slip picture"
    );
    let mut spin = with_variant();
    drive(&mut spin, 0.0, false, false);
    assert_eq!(spin.mark_variant.pictures_of(spin.marks.iter().last().unwrap()).unwrap().0, 1);
    let mut wet = with_variant();
    drive(&mut wet, 0.0, false, true);
    let mark = wet.marks.iter().last().unwrap();
    assert!(mark.wet);
    assert_eq!(wet.mark_variant.pictures_of(mark).unwrap().0, 2, "the wet set's picture on a wet road");
}

#[test]
fn a_road_the_profile_has_no_set_for_keeps_the_games_own_strip() {
    let mut effects = with_variant();
    drive(&mut effects, 1.0, true, false);
    let mark = effects.marks.iter().last().unwrap();
    assert!(!effects.mark_variant.covers(mark), "no loose set");
    let layer = effects.build(Vec3::new(-8.0, 0.0, 8.0), Vec3::X);
    assert!(layer.textured.is_empty() && !layer.surfaces.is_empty());
}

#[test]
fn textured_marks_are_cut_at_every_repeat_and_lie_just_above_the_road() {
    let mut effects = with_variant();
    drive(&mut effects, 1.0, false, false);
    let layer = effects.build(Vec3::new(-8.0, 0.0, 8.0), Vec3::X);
    assert!(layer.surfaces.is_empty(), "the pictures replace the dark strips");
    assert_eq!(layer.textured.len(), 1);
    let vertices = &layer.textured[0].vertices;
    assert!(!vertices.is_empty() && vertices.len() % 6 == 0);
    assert!(vertices.iter().all(|v| (0.0..=1.0).contains(&v.uv[0]) && (0.0..=1.0).contains(&v.uv[1])));
    assert!(vertices.iter().all(|v| (v.position[2] - 5.042).abs() < 0.05), "above the road by the offset and the lift");
    // The track is 5.9 m long and a picture covers 2 m of it: at least three repeats.
    assert!(vertices.iter().filter(|v| v.uv[0] == 0.0).count() >= 3 * 2);
    effects.reclaim_built();
}

#[test]
fn the_sets_width_and_life_apply_to_the_marks_made_under_it() {
    let mut effects = with_variant();
    drive(&mut effects, 1.0, false, false);
    effects.age(19.0);
    assert!(effects.marks.len() > 0, "life 20");
    effects.age(1.5);
    assert_eq!(effects.marks.len(), 0, "gone after the set's life, not the game's 45 s");
    let mut wide = with_variant();
    drive(&mut wide, 1.0, false, false);
    let first = wide.marks.iter().next().unwrap().lifted(0.0);
    let width = first[0].distance(first[1]);
    assert!((width - 0.5).abs() < 1e-4, "twice the tread's 0.25: {width}");
}

#[test]
fn with_no_variant_the_marks_are_the_games_own() {
    let mut effects = TireEffects::default();
    drive(&mut effects, 1.0, false, false);
    let layer = effects.build(Vec3::new(-8.0, 0.0, 8.0), Vec3::X);
    assert!(layer.textured.is_empty() && layer.surfaces.len() == effects.marks.len() * 6);
    let first = effects.marks.iter().next().unwrap().lifted(0.0);
    assert!((first[0].distance(first[1]) - 0.25).abs() < 1e-4);
}

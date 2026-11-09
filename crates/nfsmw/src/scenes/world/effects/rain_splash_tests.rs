use blackbox_modding::content::parse_rain_splash_profile;

use super::*;

const PROFILE: &str = r#"
format = 1
radius = 8.0
[[layer]]
texture = "RAINSPLASH/X/RING.PNG"
rate = 100.0
life = 0.4
life_variance = 0.0
size = [0.2, 1.0]
opacity = 0.5
[[layer]]
texture = "RAINSPLASH/X/BURST.PNG"
kind = "burst"
rate = 50.0
grid = 2
life = 0.3
"#;

/// A splash with the profile loaded without a renderer: the pictures' handles are made up.
fn loaded() -> RainSplash {
    let profile = Arc::new(parse_rain_splash_profile(PROFILE).unwrap());
    let mut splash = RainSplash::default();
    splash.set_wanted(Some(profile.clone()));
    let textures = vec![
        Some((TextureHandle::from_raw(1), BlendMode::AlphaBlend)),
        Some((TextureHandle::from_raw(2), BlendMode::AlphaBlend)),
    ];
    splash.credit = vec![0.0; 2];
    splash.loaded = Some(Loaded { profile, textures, batches: vec![Vec::new(); 2], handed: Vec::new() });
    splash
}

fn flat(_x: f32, _y: f32, _from: f32, _to: f32) -> Option<f32> {
    Some(5.0)
}

fn run(splash: &mut RainSplash, rain: f32, seconds: f32) {
    for _ in 0..(seconds * 60.0) as usize {
        splash.step(1.0 / 60.0, rain, Vec3::new(10.0, 20.0, 5.0), flat);
    }
}

#[test]
fn nothing_is_made_without_rain_or_without_a_variant() {
    let mut splash = loaded();
    run(&mut splash, 0.0, 2.0);
    assert_eq!(splash.len(), 0, "dry weather and cover make none");
    let mut none = RainSplash::default();
    run(&mut none, 1.0, 2.0);
    assert_eq!(none.len(), 0, "the game's own rain makes no splash");
}

#[test]
fn splashes_follow_the_rate_the_rain_and_the_life() {
    let mut splash = loaded();
    run(&mut splash, 1.0, 1.0);
    // 150 a second, each living 0.3 to 0.4 s: about 50 alive after the first second.
    assert!((35..=65).contains(&splash.len()), "{} live", splash.len());
    let mut light = loaded();
    run(&mut light, 0.25, 1.0);
    assert!(light.len() * 3 < splash.len() * 2, "half as many for a quarter of the rain");
    for _ in 0..90 {
        splash.step(1.0 / 60.0, 0.0, Vec3::ZERO, flat);
    }
    assert_eq!(splash.len(), 0, "they die out when the rain stops");
}

#[test]
fn splashes_are_on_the_road_around_the_player_and_none_where_there_is_no_road() {
    let mut splash = loaded();
    run(&mut splash, 1.0, 1.0);
    assert!(
        splash.splashes.iter().all(|s| s.at.z == 5.0 && s.at.truncate().distance(glam::Vec2::new(10.0, 20.0)) <= 8.01)
    );
    let mut over_a_hole = loaded();
    for _ in 0..60 {
        over_a_hole.step(1.0 / 60.0, 1.0, Vec3::ZERO, |_, _, _, _| None);
    }
    assert_eq!(over_a_hole.len(), 0);
}

#[test]
fn the_pool_is_bounded() {
    let mut splash = loaded();
    for _ in 0..3000 {
        splash.step(0.1, 1.0, Vec3::ZERO, flat);
        splash.splashes.iter_mut().for_each(|s| s.age = 0.0);
    }
    assert!(splash.len() <= POOL);
}

#[test]
fn a_ring_lies_on_the_road_and_a_burst_stands_up() {
    let mut splash = loaded();
    splash.splashes.push(Splash { layer: 0, at: Vec3::new(1.0, 2.0, 5.0), age: 0.2, life: 0.4, cell: 0, turn: 0.3 });
    splash.splashes.push(Splash { layer: 1, at: Vec3::new(3.0, 4.0, 5.0), age: 0.1, life: 0.3, cell: 3, turn: 0.0 });
    let mut layer = EffectLayer::default();
    splash.geometry(Vec3::X, &mut layer);
    assert_eq!(layer.textured.len(), 2, "one batch for each picture with splashes");
    let ring = &layer.textured[0].vertices;
    assert_eq!(ring.len(), 6);
    assert!(ring.iter().all(|v| (v.position[2] - 5.03).abs() < 1e-5), "flat, just above the road");
    let width = ring.iter().map(|v| v.position[0]).fold(f32::MIN, f32::max)
        - ring.iter().map(|v| v.position[0]).fold(f32::MAX, f32::min);
    assert!(width > 0.3 && width < 1.2, "a ring half grown is {width} wide");
    let burst = &layer.textured[1].vertices;
    assert!(burst.iter().any(|v| v.position[2] > 5.1), "it stands up from the road");
    assert!(burst.iter().all(|v| v.uv[0] >= 0.5 && v.uv[1] >= 0.5), "cell 3 of a 2 x 2 sheet is the lower right one");
    splash.reclaim(&mut layer);
    assert!(layer.textured.iter().all(|batch| batch.vertices.is_empty()), "the buffers go back to the splash");
}

#[test]
fn a_changed_profile_is_noticed() {
    let mut splash = loaded();
    assert!(!splash.needs_load());
    splash.set_wanted(Some(Arc::new(parse_rain_splash_profile("format = 1\n[[layer]]\ntexture = \"A.PNG\"").unwrap())));
    assert!(splash.needs_load());
    splash.set_wanted(None);
    assert!(splash.needs_load(), "a variant that went is dropped");
}

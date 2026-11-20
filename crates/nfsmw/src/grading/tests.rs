use std::collections::HashMap;

use blackbox_modding::content::{encode_lut, parse_grading_profile};

use super::*;

const PROFILE: &str = r#"
format = 1
[[cube]]
name = "plain"
file = "GRADING/T/PLAIN.LUT"
[[cube]]
name = "flip"
file = "GRADING/T/FLIP.LUT"
[[cube]]
name = "dark"
file = "GRADING/T/DARK.LUT"

[[set]]
weather = "clear"
[[set.key]]
hour = 6.0
cube = "plain"
[[set.key]]
hour = 18.0
cube = "flip"

[[set]]
weather = "rain"
[[set.key]]
hour = 0.0
cube = "dark"
"#;

fn cube_bytes(map: impl Fn(u8) -> u8) -> Vec<u8> {
    let identity = ColourCube::identity(8).unwrap();
    encode_lut(8, &identity.rgb().iter().map(|b| map(*b)).collect::<Vec<_>>())
}

fn files() -> HashMap<&'static str, Vec<u8>> {
    HashMap::from([
        ("GRADING/T/PLAIN.LUT", cube_bytes(|b| b)),
        ("GRADING/T/FLIP.LUT", cube_bytes(|b| 255 - b)),
        ("GRADING/T/DARK.LUT", cube_bytes(|b| b / 4)),
    ])
}

fn look_of(text: &str) -> Arc<GradingLook> {
    let files = files();
    let read = |path: &str| files.get(path).cloned().ok_or_else(|| std::io::Error::other("missing"));
    Arc::new(GradingLook::load(parse_grading_profile(text).unwrap(), read).unwrap())
}

#[test]
fn a_look_reads_every_cube_it_names_and_refuses_one_that_fails() {
    let look = look_of(PROFILE);
    assert_eq!(look.cubes.len(), 3);
    assert!(look.cubes[0].is_identity(0.0));
    let profile = parse_grading_profile(PROFILE).unwrap();
    let none = |_: &str| Err(std::io::Error::other("gone"));
    assert!(GradingLook::load(profile.clone(), none).unwrap_err().contains("PLAIN.LUT"));
    let junk = |_: &str| Ok(b"not a cube".to_vec());
    assert!(GradingLook::load(profile, junk).unwrap_err().contains("PLAIN.LUT"));
}

#[test]
fn the_wet_factor_is_the_rain_or_the_wet_the_roads_keep() {
    assert_eq!(wet_factor(0.0, 0.0), 0.0);
    assert_eq!(wet_factor(1.0, 0.0), 1.0);
    assert_eq!(wet_factor(0.0, 1.0), 1.0, "the road stays wet after the rain stops");
    assert!((wet_factor(0.3, 0.1) - 0.3).abs() < 1e-6);
    assert_eq!(wet_factor(f32::NAN, f32::NAN), 0.0);
    assert_eq!(wet_factor(7.0, -1.0), 1.0);
}

#[test]
fn a_clear_hour_grades_with_the_cubes_around_it() {
    let look = look_of(PROFILE);
    let mut selector = Selector::default();
    // Noon is half way from the plain cube at 6 to the flip at 18.
    let grade = selector.grade(&look, 12.0, 0.0, 1.0).unwrap();
    assert_eq!(grade.from.id(), look.cubes[0].id());
    assert_eq!(grade.to.as_ref().unwrap().id(), look.cubes[1].id());
    assert!((grade.mix - 0.5).abs() < 1e-6);
    assert_eq!(grade.strength, 1.0);
    // Overnight: the flip at 18 towards the plain at 6 + 24.
    let night = selector.grade(&look, 0.0, 0.0, 0.6).unwrap();
    assert_eq!((night.from.id(), night.to.unwrap().id()), (look.cubes[1].id(), look.cubes[0].id()));
    assert_eq!(night.strength, 0.6);
}

#[test]
fn no_strength_no_grade() {
    let look = look_of(PROFILE);
    let mut selector = Selector::default();
    for strength in [0.0, -1.0, f32::NAN] {
        assert!(selector.grade(&look, 12.0, 0.0, strength).is_none());
    }
    assert_eq!(selector.grade(&look, 12.0, 0.0, 5.0).unwrap().strength, 1.0);
}

#[test]
fn full_wet_uses_the_rain_set_and_in_between_is_a_baked_blend() {
    let look = look_of(PROFILE);
    let mut selector = Selector::default();
    let over = selector.grade(&look, 12.0, 1.0, 1.0).unwrap();
    assert_eq!(over.from.id(), look.cubes[2].id(), "the rain set is one cube all day");
    // Half way: one baked cube that is the mean of what the two weathers would draw.
    let rgb = [0.8, 0.4, 0.2];
    let clear = selector.grade(&look, 12.0, 0.0, 1.0).unwrap().apply(rgb);
    let dark = over.apply(rgb);
    let half = selector.grade(&look, 12.0, 0.5, 1.0).unwrap();
    assert!(half.to.is_none(), "a bake is one cube");
    let got = half.apply(rgb);
    for channel in 0..3 {
        let want = 0.5 * (clear[channel] + dark[channel]);
        assert!((got[channel] - want).abs() < 0.02, "channel {channel}: {got:?} vs {want}");
    }
}

#[test]
fn a_bake_is_kept_until_the_blend_moves_a_step() {
    let look = look_of(PROFILE);
    let mut selector = Selector::default();
    let first = selector.grade(&look, 12.0, 0.5, 1.0).unwrap();
    let same = selector.grade(&look, 12.001, 0.501, 0.7).unwrap();
    assert!(Arc::ptr_eq(&first.from, &same.from), "a hair's difference bakes nothing new");
    assert_eq!(same.strength, 0.7, "the strength does not need a bake");
    let moved = selector.grade(&look, 12.0, 0.7, 1.0).unwrap();
    assert!(!Arc::ptr_eq(&first.from, &moved.from));
    // Another look drops the bake.
    let other = look_of(PROFILE);
    let again = selector.grade(&other, 12.0, 0.7, 1.0).unwrap();
    assert!(!Arc::ptr_eq(&moved.from, &again.from));
}

#[test]
fn a_profile_with_one_weather_looks_the_same_in_all_weather() {
    let only_clear = PROFILE.split("[[set]]\nweather = \"rain\"").next().unwrap();
    let look = look_of(only_clear);
    let mut selector = Selector::default();
    let a = selector.grade(&look, 12.0, 0.0, 1.0).unwrap();
    let b = selector.grade(&look, 12.0, 1.0, 1.0).unwrap();
    assert_eq!((a.from.id(), a.mix), (b.from.id(), b.mix));
    let only_over = format!(
        "{}\n",
        PROFILE
            .replace("weather = \"clear\"", "weather = \"rain\"")
            .split("[[set]]\nweather = \"rain\"\n[[set.key]]\nhour = 0.0")
            .next()
            .unwrap()
    );
    let look = look_of(&only_over);
    assert!(selector.grade(&look, 3.0, 0.0, 1.0).is_some(), "a rain-only profile is the fallback too");
}

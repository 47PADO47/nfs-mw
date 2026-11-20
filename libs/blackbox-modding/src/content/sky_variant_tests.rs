use super::*;

const PROFILE: &str = r#"
format = 1
exposure = 0.5
wind = 0.02

[moon]
file = "SKY/MW2012/MOON.DDS"
size = 0.02

[[dome]]
name = "night"
file = "SKY/MW2012/NIGHT.DDS"

[[dome]]
name = "day"
file = "SKY/MW2012/DAY.DDS"
glow = 0.5

[[weather]]
max_cover = 0.4
[[weather.key]]
hour = 0.0
dome = "night"
tint = [0.0, 0.0, 0.0]
[[weather.key]]
hour = 6.0
dome = "day"
tint = [1.0, 1.0, 1.0]
[[weather.key]]
hour = 18.0
dome = "night"
tint = [0.2, 0.2, 0.2]

[[weather]]
max_cover = 1.0
gain = 2.0
[[weather.key]]
hour = 0.0
dome = "day"
tint = [0.5, 0.5, 0.5]
"#;

#[test]
fn a_profile_parses_with_its_defaults() {
    let profile = parse(PROFILE).unwrap();
    assert_eq!((profile.exposure, profile.wind), (0.5, 0.02));
    assert_eq!(profile.moon.as_ref().map(|m| (m.size, m.brightness)), Some((0.02, 1.0)));
    assert_eq!(profile.domes.len(), 2);
    assert_eq!(profile.domes[1].glow, Some(0.5));
    assert_eq!(profile.weathers[0].gain, 1.0, "the default gain");
    let minimal = "format = 1\n[[dome]]\nname = \"a\"\nfile = \"A.PNG\"\n[[weather]]\nmax_cover = 1.0\n[[weather.key]]\nhour = 0.0\ndome = \"a\"\ntint = [1.0, 1.0, 1.0]\n";
    let minimal = parse(minimal).unwrap();
    assert_eq!((minimal.exposure, minimal.wind, minimal.moon), (1.0, 0.01, None));
}

#[test]
fn the_weather_follows_the_cloud_cover() {
    let profile = parse(PROFILE).unwrap();
    assert_eq!(profile.weather(0.1).unwrap().max_cover, 0.4);
    assert_eq!(profile.weather(0.4).unwrap().max_cover, 0.4);
    assert_eq!(profile.weather(0.9).unwrap().max_cover, 1.0);
    assert_eq!(profile.weather(f32::NAN).unwrap().max_cover, 0.4);
    assert_eq!(profile.weather(7.0).unwrap().max_cover, 1.0);
}

#[test]
fn the_pose_blends_the_keys_around_the_hour_and_wraps_the_day() {
    let profile = parse(PROFILE).unwrap();
    let pose = profile.pose(12.0, 0.0).unwrap();
    assert_eq!(pose.dome, [1, 0]);
    assert!((pose.blend - 0.5).abs() < 1e-6, "{pose:?}");
    assert_eq!(pose.tint, [[1.0; 3], [0.2; 3]]);
    let night = profile.pose(21.0, 0.0).unwrap();
    assert_eq!(night.dome, [0, 0], "from the last key (18) to the first (0) wraps past midnight");
    assert!((night.blend - 0.5).abs() < 1e-6);
    assert_eq!(profile.pose(30.0, 0.0).unwrap(), profile.pose(6.0, 0.0).unwrap(), "the hour is taken modulo 24");
    let overcast = profile.pose(3.0, 0.9).unwrap();
    assert_eq!((overcast.dome, overcast.blend, overcast.gain), ([1, 1], 0.0, 2.0), "one key holds all day");
}

#[test]
fn the_hour_exactly_on_a_key_is_that_key() {
    let profile = parse(PROFILE).unwrap();
    let pose = profile.pose(6.0, 0.0).unwrap();
    assert_eq!((pose.dome, pose.blend), ([1, 0], 0.0));
}

#[test]
fn numbers_out_of_range_and_unknown_things_are_refused() {
    let edit = |from: &str, to: &str| parse(&PROFILE.replace(from, to));
    assert!(edit("format = 1", "format = 2").is_err());
    assert!(edit("exposure = 0.5", "exposure = 9.0").is_err());
    assert!(edit("wind = 0.02", "wind = 2.0").is_err());
    assert!(edit("size = 0.02", "size = 0.5").is_err());
    assert!(edit("glow = 0.5", "glow = 1.5").is_err());
    assert!(edit("dome = \"night\"\ntint = [0.0", "dome = \"nope\"\ntint = [0.0").is_err());
    assert!(edit("tint = [1.0, 1.0, 1.0]", "tint = [1.0, 9.0, 1.0]").is_err());
    assert!(edit("hour = 6.0", "hour = 0.0").is_err(), "hours rise");
    assert!(edit("hour = 18.0", "hour = 24.0").is_err());
    assert!(edit("max_cover = 0.4", "max_cover = 1.5").is_err());
    assert!(edit("gain = 2.0", "gain = 0.0").is_err());
    assert!(edit("format = 1", "format = 1\nsurprise = 1").is_err());
}

#[test]
fn paths_and_names_are_plain() {
    let edit = |from: &str, to: &str| parse(&PROFILE.replace(from, to));
    assert!(edit("SKY/MW2012/DAY.DDS", "../DAY.DDS").is_err());
    assert!(edit("SKY/MW2012/DAY.DDS", "SKY/DAY.TXT").is_err());
    assert!(edit("SKY/MW2012/MOON.DDS", "C:/MOON.DDS").is_err());
    assert!(edit("name = \"day\"", "name = \"Day\"").is_err());
    assert!(edit("name = \"night\"", "name = \"day\"").is_err(), "a name is used once");
}

#[test]
fn a_profile_needs_a_dome_and_a_weather() {
    assert!(parse("format = 1").is_err());
    assert!(parse("format = 1\n[[dome]]\nname = \"a\"\nfile = \"A.PNG\"").is_err());
}

#[test]
fn the_option_and_folder_are_the_sky_kind() {
    assert_eq!(sky_profile_path("mw2012"), "SKY/MW2012/profile.toml");
    assert_eq!(SKY.option, "sky_variant");
}

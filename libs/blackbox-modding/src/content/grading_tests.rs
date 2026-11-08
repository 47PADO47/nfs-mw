use super::*;

const GOOD: &str = r#"
format = 1

[[cube]]
name = "night"
file = "GRADING/MW2012/C003E93.LUT"

[[cube]]
name = "noon"
file = "GRADING/MW2012/C04E220.LUT"

[[cube]]
name = "dusk"
file = "GRADING/MW2012/dusk.cube"

[[set]]
weather = "clear"
[[set.key]]
hour = 4.0
cube = "night"
[[set.key]]
hour = 12.0
cube = "noon"
[[set.key]]
hour = 18.0
cube = "dusk"

[[set]]
weather = "overcast"
[[set.key]]
hour = 0.0
cube = "noon"
"#;

fn refused(text: &str) -> String {
    parse(text).expect_err("refused").to_string()
}

#[test]
fn a_profile_parses_with_its_cubes_and_sets() {
    let profile = parse(GOOD).unwrap();
    assert_eq!(profile.cubes.len(), 3);
    assert_eq!(profile.cubes[2], GradingCube { name: "dusk".into(), file: "GRADING/MW2012/dusk.cube".into() });
    let clear = profile.set(GradingWeather::Clear).unwrap();
    assert_eq!(
        clear.keys,
        [GradingKey { hour: 4.0, cube: 0 }, GradingKey { hour: 12.0, cube: 1 }, GradingKey { hour: 18.0, cube: 2 }]
    );
    assert_eq!(profile.set(GradingWeather::Overcast).unwrap().keys.len(), 1);
    assert_eq!(grading_profile_path("mw2012"), "GRADING/MW2012/profile.toml");
}

#[test]
fn the_hour_picks_the_keys_around_it_and_the_blend() {
    let profile = parse(GOOD).unwrap();
    let clear = profile.set(GradingWeather::Clear).unwrap();
    let at = |hour| clear.at(hour).unwrap();
    // On a key: that cube, the next one not yet in.
    assert_eq!(at(12.0), GradingPair { from: 1, to: 2, mix: 0.0 });
    // Half way from noon (12) to dusk (18).
    assert_eq!(at(15.0), GradingPair { from: 1, to: 2, mix: 0.5 });
    // After the last key the day wraps to the first: dusk (18) to night (4 + 24), 11 hours from 22 is 4 of 10.
    let late = at(22.0);
    assert_eq!((late.from, late.to), (2, 0));
    assert!((late.mix - 0.4).abs() < 1e-6, "{late:?}");
    // Before the first key: from the last key of the day before.
    let early = at(1.0);
    assert_eq!((early.from, early.to), (2, 0));
    assert!((early.mix - 0.7).abs() < 1e-6, "{early:?}");
    // The hour is wrapped, and anything that is not a number is midnight.
    assert_eq!(at(36.0), at(12.0));
    assert_eq!(at(-12.0), at(12.0));
    assert_eq!(at(f32::NAN), at(0.0));
    // One key is one cube all day.
    let all_day = profile.set(GradingWeather::Overcast).unwrap();
    for hour in [0.0, 6.5, 23.9] {
        assert_eq!(all_day.at(hour).unwrap(), GradingPair { from: 1, to: 1, mix: 0.0 });
    }
}

#[test]
fn a_blend_runs_continuously_over_midnight() {
    let profile = parse(GOOD).unwrap();
    let clear = profile.set(GradingWeather::Clear).unwrap();
    let mix_at = |hour: f32| clear.at(hour).unwrap().mix;
    // From 18 (dusk) rising to 28 (4 next morning): 23.999 and 0.0 are the same moment.
    assert!((mix_at(23.999) - mix_at(0.0)).abs() < 1e-3);
    assert!(mix_at(0.0) < mix_at(3.9) && mix_at(3.9) < 1.0);
}

#[test]
fn a_profile_is_checked() {
    assert!(refused("format = 2\n").contains("format"));
    assert!(refused("format = 1\n").contains("cube"), "no cubes");
    let one = "format = 1\n[[cube]]\nname = \"a\"\nfile = \"GRADING/X/A.LUT\"\n";
    assert!(refused(one).contains("set"), "no sets");
    let keyed = |key: &str| format!("{one}[[set]]\nweather = \"clear\"\n[[set.key]]\n{key}\n");
    assert!(parse(&keyed("hour = 3.0\ncube = \"a\"")).is_ok());
    assert!(refused(&keyed("hour = 24.0\ncube = \"a\"")).contains("hour"));
    assert!(refused(&keyed("hour = -1.0\ncube = \"a\"")).contains("hour"));
    assert!(refused(&keyed("hour = 3.0\ncube = \"b\"")).contains("cube"));
    assert!(refused(&keyed("hour = 3.0\ncube = \"a\"\nspeed = 2")).contains("speed"), "unknown key");
    let rising = format!("{}[[set.key]]\nhour = 3.0\ncube = \"a\"\n", keyed("hour = 3.0\ncube = \"a\""));
    assert!(refused(&rising).contains("rise"));
    let twice = format!(
        "{}[[set]]\nweather = \"clear\"\n[[set.key]]\nhour = 1.0\ncube = \"a\"\n",
        keyed("hour = 3.0\ncube = \"a\"")
    );
    assert!(refused(&twice).contains("one set"));
    assert!(refused("format = 1\n[[set]]\nweather = \"clear\"\n").contains("cube"));
    let empty_set = format!("{one}[[set]]\nweather = \"clear\"\n");
    assert!(refused(&empty_set).contains("keys"));
    let weather = format!("{one}[[set]]\nweather = \"snow\"\n[[set.key]]\nhour = 1.0\ncube = \"a\"\n");
    assert!(parse(&weather).is_err());
}

#[test]
fn cube_names_and_files_are_checked() {
    let with = |name: &str, file: &str| {
        format!(
            "format = 1\n[[cube]]\nname = \"{name}\"\nfile = \"{file}\"\n[[set]]\nweather = \"clear\"\n[[set.key]]\nhour = 1.0\ncube = \"{name}\"\n"
        )
    };
    assert!(parse(&with("a", "GRADING/X/A.LUT")).is_ok());
    assert!(parse(&with("a", "GRADING/X/a.cube")).is_ok());
    assert!(parse(&with("A", "GRADING/X/A.LUT")).is_err(), "names are lower case");
    assert!(parse(&with("", "GRADING/X/A.LUT")).is_err());
    assert!(parse(&with(&"a".repeat(33), "GRADING/X/A.LUT")).is_err());
    for file in ["GRADING/X/A.PNG", "../A.LUT", "/A.LUT", "GRADING//A.LUT", "C:/A.LUT", "a b.lut"] {
        assert!(parse(&with("a", file)).is_err(), "{file}");
    }
    let twice = "format = 1\n[[cube]]\nname = \"a\"\nfile = \"A.LUT\"\n[[cube]]\nname = \"a\"\nfile = \"B.LUT\"\n[[set]]\nweather = \"clear\"\n[[set.key]]\nhour = 1.0\ncube = \"a\"\n";
    assert!(refused(twice).contains("once"));
}

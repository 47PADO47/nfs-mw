use blackbox_modding::content::{GradingWeather, decode_cube, parse_grading_profile};
use chameleon_postfx::environment::{Timeline, TimelineKey};

use super::*;

fn timeline(id: u64, keys: &[(f32, u64, Option<u64>)]) -> Timeline {
    let full = |cube: u64| 0x0100_0000_0000_0000 | cube;
    Timeline {
        id,
        keys: keys
            .iter()
            .map(|(hour, cube, wet)| TimelineKey { hour: *hour, cube: full(*cube), wet_cube: wet.map(full) })
            .collect(),
    }
}

/// The shape of the install's: a long timeline whose cubes change through the day, a shorter one, and a flat one.
fn all() -> Vec<Timeline> {
    vec![
        timeline(
            0x0100_0000_0009_EFCA,
            &[(5.0, 0x43FB, None), (11.0, 0x43FB, None), (15.0, 0x43FB, None), (20.0, 0x43FB, None)],
        ),
        timeline(0x0100_0000_000F_083C, &[(0.0, 0x3E93, Some(0x3E9E)), (6.0, 0xF864, Some(0x9C45))]),
        timeline(
            0x0100_0000_0006_9CBB,
            &[
                (0.0, 0x3E93, Some(0x3E9E)),
                (5.0, 0xF864, Some(0x9C45)),
                (12.0, 0xE220, Some(0x9C44)),
                (18.0, 0xF862, Some(0x4652)),
                (22.0, 0xF862, Some(0x4652)),
            ],
        ),
        timeline(0x0100_0000_0012_4546, &[(0.0, 0x3E93, None), (5.0, 0xF864, None), (12.0, 0xE220, None)]),
    ]
}

#[test]
fn the_timeline_is_the_first_that_changes_cube_through_the_day() {
    let all = all();
    assert_eq!(plan(&all).unwrap().timeline.id & 0xff_ffff, 0x69CBB);
    // A flat timeline is no plan, and neither is nothing.
    assert!(super::plan(&all[..1]).is_none());
    assert!(super::plan(&[]).is_none());
}

#[test]
fn the_cubes_are_listed_dry_then_wet_each_once() {
    let all = all();
    let plan = plan(&all).unwrap();
    assert!(has_rain(&plan));
    let names: Vec<String> = cubes(&plan).into_iter().map(cube_name).collect();
    assert_eq!(names, ["c003e93", "c00f864", "c00e220", "c00f862", "c003e9e", "c009c45", "c009c44", "c004652"]);
}

#[test]
fn names_and_files_follow_the_low_bits_of_the_id() {
    assert_eq!(cube_name(0x0100_0000_0000_3E93), "c003e93");
    assert_eq!(file_name(0x0100_0000_0012_43FB), "C1243FB.LUT");
    assert_eq!(file_name(0x0100_0000_0004_E220), "C04E220.LUT");
}

#[test]
fn the_profile_is_accepted_by_the_games_parser_with_the_hours_and_cubes_of_the_plan() {
    let all = all();
    let plan = plan(&all).unwrap();
    let text = profile_text(&plan);
    let profile = parse_grading_profile(&text).unwrap_or_else(|e| panic!("{e}\n{text}"));
    assert_eq!(profile.cubes.len(), 8);
    assert!(profile.cubes.iter().all(|cube| cube.file.starts_with("GRADING/MW2012/C") && cube.file.ends_with(".LUT")));
    let clear = profile.set(GradingWeather::Clear).unwrap();
    let hours: Vec<f32> = clear.keys.iter().map(|k| k.hour).collect();
    assert_eq!(hours, [0.0, 5.0, 12.0, 18.0, 22.0]);
    assert_eq!(profile.cubes[clear.keys[2].cube].name, cube_name(0x0100_0000_0000_E220));
    let rain = profile.set(GradingWeather::Rain).unwrap();
    assert_eq!(rain.keys.iter().map(|k| k.hour).collect::<Vec<_>>(), hours, "the same hours, the wet cubes");
    assert_eq!(profile.cubes[rain.keys[2].cube].name, cube_name(0x0100_0000_0000_9C44));
}

#[test]
fn a_timeline_without_wet_cubes_gives_a_profile_with_one_set() {
    let all = all();
    let dry = Plan { timeline: &all[3] };
    assert!(!has_rain(&dry));
    let profile = parse_grading_profile(&profile_text(&dry)).unwrap();
    assert_eq!((profile.sets.len(), profile.cubes.len()), (1, 3));
}

#[test]
fn a_written_cube_decodes_back_to_the_converted_entries() {
    let data = chameleon_postfx::environment::CubeRgb { size: 2, rgb: (0..24).collect() };
    let bytes = blackbox_modding::content::encode_lut(data.size, &data.rgb);
    let back = decode_cube("GRADING/MW2012/C1.LUT", &bytes).unwrap();
    assert_eq!((back.size, back.rgb), (2, data.rgb));
}

fn real_source() -> Option<Source> {
    let root = std::env::var_os("MW2012_GAME_DIR")?;
    Source::open(std::path::Path::new(&root)).ok()
}

#[test]
#[ignore = "reads the 2012 install (MW2012_GAME_DIR)"]
fn the_install_gives_the_two_sets_and_eleven_cubes() {
    let Some(source) = real_source() else { return };
    let out = std::env::temp_dir().join(format!("nfsmw-grading-real-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&out);
    write(&source, &out);
    let folder = out.join("files").join(FOLDER);
    let profile = parse_grading_profile(&std::fs::read_to_string(folder.join("profile.toml")).unwrap()).unwrap();
    assert_eq!(profile.cubes.len(), 11);
    assert_eq!(profile.set(GradingWeather::Clear).unwrap().keys.len(), 21);
    assert_eq!(profile.set(GradingWeather::Rain).unwrap().keys.len(), 21);
    let mut total = 0;
    for cube in &profile.cubes {
        let bytes = std::fs::read(out.join("files").join(&cube.file)).unwrap();
        let data = decode_cube(&cube.file, &bytes).unwrap();
        assert_eq!(data.size, 16);
        total += bytes.len();
    }
    assert_eq!(total, 11 * (8 + 12_288), "the whole output is about 135 KB");
    let _ = std::fs::remove_dir_all(&out);
}

#[test]
fn a_key_without_a_wet_cube_keeps_its_dry_cube_in_the_rain_set() {
    let wide = timeline(
        0x0100_0000_0006_9CBB,
        &[(0.0, 0x3E93, Some(0x3E9E)), (4.0, 0x4514, None), (12.0, 0xE220, Some(0x9C44)), (18.0, 0xF862, Some(0x4652))],
    );
    let plan = Plan { timeline: &wide };
    assert!(has_rain(&plan));
    let profile = parse_grading_profile(&profile_text(&plan)).unwrap();
    let rain = profile.set(GradingWeather::Rain).unwrap();
    assert_eq!(rain.keys.len(), 4);
    assert_eq!(profile.cubes[rain.keys[1].cube].name, cube_name(0x0100_0000_0000_4514), "the dry cube");
}

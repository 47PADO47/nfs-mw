use blackbox_modding::content::parse_nitrous_profile;
use chameleon_postfx::{Bloom, CUBE_LEN, ColourCube, Keyframe, State, Vignette};

use super::*;
use crate::nitrous::{Found, profile_text};

fn keyframe() -> Keyframe {
    Keyframe {
        vignette: Vignette {
            a: [0.526, 0.51],
            b: [0.459, 0.522],
            inner: [1.0; 4],
            outer: [0.037, 0.114, 0.089, 1.0],
            tail: [0.0, 0.0, 1.0, 0.0, 0.258],
        },
        bloom: Bloom { colour: [0.597, 0.831, 1.0, 1.0], a: 0.2, b: 1.35 },
        general: [1.0, 0.1, 1.0],
    }
}

fn cube(blue: f32, green: f32, red: f32) -> ColourCube {
    let mut block = vec![0u8; CUBE_LEN];
    for step in 0..16 {
        let at = 16 + step * 273 * 3;
        let v = step as f32 / 15.0;
        block[at] = (v * blue * 255.0).min(255.0) as u8;
        block[at + 1] = (v * green * 255.0).min(255.0) as u8;
        block[at + 2] = (v * red * 255.0).min(255.0) as u8;
    }
    ColourCube::parse(&block).unwrap()
}

fn state(blend: f32) -> State {
    State { expression: Some("Reporter.NitrousState.UsingNitrous".into()), blend_seconds: blend, weight: 1.0 }
}

fn screen(cube: Option<ColourCube>) -> Screen {
    Screen { keyframe: keyframe(), cube, state: state(2.0) }
}

#[test]
fn the_section_parses_with_the_games_own_reader_and_carries_the_decoded_numbers() {
    let text = format!("format = 1\n{}", section(&screen(Some(cube(0.9, 1.1, 1.2)))));
    let profile = parse_nitrous_profile(&text).unwrap().screen.unwrap();
    assert_eq!(profile.edge, [0.037, 0.114, 0.089]);
    assert!((profile.edge_inner - 0.4905).abs() < 1e-4, "the mean of the keyframe's second pair of numbers");
    assert!(profile.tint[0] > profile.tint[2], "red up, blue down: {:?}", profile.tint);
    assert_eq!((profile.fade_in, profile.fade_out), (2.0, 2.0), "the state's blend time");
    assert_eq!((profile.fov, profile.pull_back, profile.shake), (FOV, PULL_BACK, SHAKE));
    assert_eq!(
        (profile.radial_blur, profile.vignette),
        (RADIAL_BLUR, 0.0),
        "the 2012 vignette darkens, it does not add"
    );
    assert!(profile.has_grade() && profile.has_camera());
}

#[test]
fn an_identity_cube_or_none_leaves_the_colour_alone() {
    for cube in [None, Some(cube(1.0, 1.0, 1.0))] {
        let text = format!("format = 1\n{}", section(&screen(cube)));
        let profile = parse_nitrous_profile(&text).unwrap().screen.unwrap();
        assert_eq!((profile.tint, profile.contrast), ([1.0; 3], 1.0));
        assert_eq!(profile.edge[0], 0.037, "the vignette is still there");
    }
}

#[test]
fn the_screen_lands_in_the_profile_with_or_without_sounds() {
    let found = Found { screen: Some(screen(None)), ..Default::default() };
    assert!(!found.is_empty());
    let profile = parse_nitrous_profile(&profile_text(&found)).unwrap();
    assert!(profile.audio.is_none() && profile.screen.is_some());
}

#[test]
#[ignore = "reads the 2012 install (MW2012_GAME_DIR)"]
fn the_real_install_has_a_nitrous_state_with_a_dark_teal_vignette_and_a_warm_grade() {
    let root = std::path::PathBuf::from(std::env::var_os("MW2012_GAME_DIR").expect("MW2012_GAME_DIR"));
    let source = crate::source::Source::open(&root).unwrap();
    let found = find(&source).expect("POSTFX.BNDL has the nitrous state");
    println!("{found:#?}\n{}", section(&found));
    assert_eq!(found.state.blend_seconds, 2.0);
    assert!((found.keyframe.vignette.outer[1] - 0.114).abs() < 0.001);
    let cube = found.cube.clone().expect("the state imports a colour cube");
    assert!(!cube.is_identity());
    let grade = cube.grade();
    assert!(grade.tint[0] > grade.tint[2], "{grade:?}");
    let text = format!("format = 1\n{}", section(&found));
    assert!(parse_nitrous_profile(&text).is_ok());
}

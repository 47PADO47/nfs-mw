use super::super::{CarManifest, parse_car};
use super::*;

const FLARE: &str = "
[[lamp_flare]]
state = \"head\"
side = \"left\"
sprite = \"FASTCOUPE_FLARE0\"
position = [2.2, 0.7, 0.65]
direction = [2.0, 0.0, 0.0]
radius = [0.2, 0.6]
";

const BAR: &str = "
[[light_bar]]
siren = \"blue\"
sprite = \"FASTCOUPE_FLARE1\"
position = [0.0, 0.4, 1.4]
direction = [0.0, 0.0, 1.0]
radius = [0.15, 0.5]
pattern = \"9009\"
step_seconds = 0.125
phase_seconds = 0.5
";

fn car(text: &str) -> Result<CarManifest, ContentError> {
    parse_car("fastcoupe", text)
}

#[test]
fn a_car_has_no_lamp_flares_unless_it_gives_the_tables() {
    let manifest = car("name = \"x\"").unwrap();
    assert!(manifest.lamp_flares.is_empty() && manifest.light_bars.is_empty());
}

#[test]
fn a_flare_is_read_with_the_defaults_and_a_unit_direction() {
    let manifest = car(FLARE).unwrap();
    let flare = &manifest.lamp_flares[0];
    assert_eq!((flare.lit, flare.side), (LampLit::Head, LampSide::Left));
    assert_eq!(flare.look.sprite, "FASTCOUPE_FLARE0");
    assert_eq!(flare.look.direction, [1.0, 0.0, 0.0], "the direction is made a unit vector");
    assert_eq!((flare.look.colour, flare.look.alpha), ([1.0; 3], [1.0; 2]));
    assert_eq!((flare.look.range, flare.look.falloff, flare.look.z_bias), ([2.0, 100.0], 1.0, 0.0));
    assert_eq!(flare.look.radius, [0.2, 0.6]);
}

#[test]
fn a_flare_without_a_side_is_a_centre_lamp() {
    let text = FLARE.replace("side = \"left\"\n", "");
    assert_eq!(car(&text).unwrap().lamp_flares[0].side, LampSide::Centre);
}

#[test]
fn a_light_bar_lamp_is_read_with_its_pattern() {
    let manifest = car(BAR).unwrap();
    let bar = &manifest.light_bars[0];
    assert_eq!(bar.siren, SirenColour::Blue);
    assert_eq!(bar.pattern, vec![1.0, 0.0, 0.0, 1.0]);
    assert_eq!((bar.step_seconds, bar.phase_seconds), (0.125, 0.5));
    assert_eq!(bar.look.direction, [0.0, 0.0, 1.0]);
}

#[test]
fn both_tables_can_be_given_more_than_once() {
    let text = format!("{FLARE}{FLARE}{BAR}{BAR}{BAR}");
    let manifest = car(&text).unwrap();
    assert_eq!((manifest.lamp_flares.len(), manifest.light_bars.len()), (2, 3));
}

#[test]
fn a_bad_flare_is_refused_with_the_field() {
    for (from, to) in [
        ("sprite = \"FASTCOUPE_FLARE0\"", "sprite = \"no spaces\""),
        ("direction = [2.0, 0.0, 0.0]", "direction = [0.0, 0.0, 0.0]"),
        ("position = [2.2, 0.7, 0.65]", "position = [200.0, 0.7, 0.65]"),
        ("radius = [0.2, 0.6]", "radius = [0.2, -1.0]"),
        ("radius = [0.2, 0.6]", "radius = [0.2, 0.6]\nrange = [50.0, 10.0]"),
        ("radius = [0.2, 0.6]", "radius = [0.2, 0.6]\ncolour = [2.0, 1.0, 1.0]"),
        ("radius = [0.2, 0.6]", "radius = [0.2, 0.6]\nfalloff = 40.0"),
        ("state = \"head\"", "state = \"flash\""),
    ] {
        let text = FLARE.replace(from, to);
        assert!(car(&text).is_err(), "{to}");
    }
    assert!(car(&format!("{FLARE}colour_typo = 1\n")).is_err(), "an unknown key refuses the car");
}

#[test]
fn a_bad_light_bar_is_refused() {
    for (from, to) in [
        ("pattern = \"9009\"", "pattern = \"\""),
        ("pattern = \"9009\"", "pattern = \"90a9\""),
        ("step_seconds = 0.125", "step_seconds = 0.0"),
        ("step_seconds = 0.125", "step_seconds = 5.0"),
        ("phase_seconds = 0.5", "phase_seconds = -1.0"),
        ("siren = \"blue\"", "siren = \"green\""),
    ] {
        let text = BAR.replace(from, to);
        assert!(car(&text).is_err(), "{to}");
    }
}

#[test]
fn too_many_lamps_are_refused() {
    let flares = FLARE.repeat(MAX_FLARES + 1);
    assert!(car(&flares).is_err());
    let bars = BAR.repeat(MAX_BARS + 1);
    assert!(car(&bars).is_err());
    assert!(car(&FLARE.repeat(MAX_FLARES)).is_ok());
}

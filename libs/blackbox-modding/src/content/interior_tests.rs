use super::parse_interior as parse;
use super::*;

const HIDEOUT: &str = r#"
[interior]
label = "Hideout"
prompt = "Press Enter to go in"
zone = { shape = "sphere", pos = [1520.0, -840.0, 12.0], radius = 6.0 }
pack = "hideout"
spawn = { pos = [3010.0, 4020.0, 0.0], heading = 90.0 }
exit = { shape = "box", pos = [3016.0, 4020.0, 0.0], half = [2.0, 3.0, 2.0], yaw = 90.0, prompt = "Press Enter to leave" }
time = { hour = 22.0, lights = true }
suspend = ["traffic", "cops"]
"#;

fn with(from: &str, to: &str) -> String {
    assert!(HIDEOUT.contains(from), "the test text has {from:?}");
    HIDEOUT.replace(from, to)
}

fn refused(text: &str) -> ContentError {
    parse("hideout", text).expect_err("the interior is refused")
}

#[test]
fn a_valid_interior_parses() {
    let interior = parse("hideout", HIDEOUT).unwrap();
    assert_eq!(interior.key, "hideout");
    assert_eq!(interior.label, "Hideout");
    assert_eq!(interior.prompt.as_deref(), Some("Press Enter to go in"));
    assert_eq!(interior.pack, "hideout");
    assert_eq!(interior.entry, Shape::Sphere { centre: [1520.0, -840.0, 12.0], radius: 6.0 });
    assert_eq!(interior.spawn, Pose { pos: [3010.0, 4020.0, 0.0], heading: 90.0 });
    let exit = interior.exit.clone();
    assert_eq!(exit.prompt.as_deref(), Some("Press Enter to leave"));
    assert_eq!(exit.shape, Shape::Box { centre: [3016.0, 4020.0, 0.0], half: [2.0, 3.0, 2.0], yaw: 90.0 });
    assert_eq!(interior.time, Some(TimeOverride { hour: 22.0, freeze: true, lights: Some(true) }));
}

#[test]
fn suspend_lists_the_named_parts_and_defaults_to_all_three() {
    let interior = parse("hideout", HIDEOUT).unwrap();
    assert_eq!(interior.suspend, Suspend { traffic: true, cops: true, pursuit: false });
    let all = parse("hideout", &HIDEOUT.replace("suspend = [\"traffic\", \"cops\"]", "")).unwrap();
    assert_eq!(all.suspend, Suspend::default());
    assert!(all.suspend.pursuit);
}

#[test]
fn an_unknown_or_repeated_suspend_name_is_refused() {
    let unknown = with("\"cops\"]", "\"police\"]");
    assert!(matches!(refused(&unknown), ContentError::UnknownSuspend(name) if name == "police"));
    let repeated = with("\"cops\"]", "\"traffic\"]");
    assert!(matches!(refused(&repeated), ContentError::RepeatedSuspend(name) if name == "traffic"));
}

#[test]
fn the_kind_must_be_area() {
    let page = with("[interior]", "[interior]\nkind = \"page\"");
    assert!(matches!(refused(&page), ContentError::KindNotInV1(kind) if kind == "page"));
    assert!(parse("hideout", &with("[interior]", "[interior]\nkind = \"area\"")).is_ok());
}

#[test]
fn a_sphere_takes_a_radius_and_a_box_takes_half_sizes() {
    let both = with("radius = 6.0 }", "radius = 6.0, half = [1.0, 1.0, 1.0] }");
    assert!(matches!(refused(&both), ContentError::Field { field: "zone", .. }));
    let box_with_radius = with(
        "zone = { shape = \"sphere\", pos = [1520.0, -840.0, 12.0], radius = 6.0 }",
        "zone = { shape = \"box\", pos = [1520.0, -840.0, 12.0], radius = 6.0 }",
    );
    assert!(matches!(refused(&box_with_radius), ContentError::Field { field: "zone", .. }));
    let unknown = with("shape = \"sphere\", pos = [1520.0", "shape = \"cone\", pos = [1520.0");
    assert!(matches!(refused(&unknown), ContentError::UnknownShape(shape) if shape == "cone"));
}

#[test]
fn a_zone_radius_must_be_positive() {
    let zero = with("radius = 6.0 }", "radius = 0.0 }");
    assert!(matches!(refused(&zero), ContentError::Field { field: "zone", .. }));
}

#[test]
fn the_hour_is_from_zero_up_to_twenty_four() {
    assert!(matches!(refused(&with("hour = 22.0", "hour = 24.0")), ContentError::Field { field: "time", .. }));
    assert!(matches!(refused(&with("hour = 22.0", "hour = -1.0")), ContentError::Field { field: "time", .. }));
    assert!(parse("hideout", &with("hour = 22.0", "hour = 0.0")).is_ok());
}

#[test]
fn freeze_defaults_to_true_and_lights_are_kept_for_the_game_to_report() {
    let interior = parse("hideout", &with("hour = 22.0, lights = true", "hour = 22.0")).unwrap();
    assert_eq!(interior.time, Some(TimeOverride { hour: 22.0, freeze: true, lights: None }));
}

#[test]
fn a_sphere_holds_its_surface_and_nothing_beyond() {
    let zone = Shape::Sphere { centre: [1.0, 2.0, 3.0], radius: 6.0 };
    assert!(zone.contains([1.0, 2.0, 3.0]));
    assert!(zone.contains([7.0, 2.0, 3.0]), "the surface counts as inside");
    assert!(!zone.contains([1.0, 8.1, 3.0]));
}

#[test]
fn a_box_is_held_in_its_own_axes() {
    let unturned = Shape::Box { centre: [0.0; 3], half: [2.0, 1.0, 2.0], yaw: 0.0 };
    assert!(unturned.contains([1.5, 0.0, 0.0]));
    assert!(!unturned.contains([0.0, 1.5, 0.0]), "one metre to the side is outside a 1 m half size");
    // Turned 90 degrees: the box's own x runs along world +y.
    let turned = Shape::Box { centre: [0.0; 3], half: [2.0, 1.0, 2.0], yaw: 90.0 };
    assert!(turned.contains([0.0, 1.5, 0.0]));
    assert!(!turned.contains([1.5, 0.0, 0.0]));
    assert!(!turned.contains([0.0, 0.0, 2.5]), "the height is bounded too");
}

#[test]
fn a_key_that_is_not_the_file_name_is_refused() {
    let text = with("[interior]", "[interior]\nkey = \"den\"");
    assert!(matches!(refused(&text), ContentError::KeyMismatch { .. }));
}

#[test]
fn the_exit_zone_is_required() {
    let text = HIDEOUT.lines().filter(|line| !line.starts_with("exit =")).collect::<Vec<_>>().join("\n");
    assert!(parse("hideout", &text).is_err());
}

#[test]
fn a_blank_label_is_refused() {
    assert!(matches!(refused(&with("label = \"Hideout\"", "label = \"  \"")), ContentError::InvalidText(_)));
}

use super::*;

fn car(text: &str) -> Result<CarManifest, ContentError> {
    parse_car("fastcoupe", text)
}

const ONE: &str = r#"
[[damage_parts]]
name = "Bonnet"
model = "FASTCOUPE_KIT00_BONNET"
zones = ["front", "front_left"]
pivot = [0.7, 0.0, 0.9]
axis = [0.0, 2.0, 0.0]
angle = -15.0
stages = [0.04, 0.05, 0.5]
detach_at = 0.9
"#;

#[test]
fn a_car_has_no_damage_parts_unless_it_lists_them() {
    assert!(car("").unwrap().damage_parts.is_empty());
}

#[test]
fn a_part_is_read_and_its_axis_normalised() {
    let parts = car(ONE).unwrap().damage_parts;
    assert_eq!(parts.len(), 1);
    let part = &parts[0];
    assert_eq!(part.model, "FASTCOUPE_KIT00_BONNET");
    assert_eq!(part.zones, vec![DamageZone::Front, DamageZone::FrontLeft]);
    assert_eq!(part.axis, [0.0, 1.0, 0.0]);
    assert_eq!(part.stages, vec![0.04, 0.05, 0.5]);
    assert_eq!(part.detach_at, Some(0.9));
}

#[test]
fn stages_default_to_the_first_damage() {
    let text = ONE.replace("stages = [0.04, 0.05, 0.5]\n", "").replace("detach_at = 0.9\n", "");
    let part = &car(&text).unwrap().damage_parts[0];
    assert_eq!(part.stages, vec![0.0]);
    assert_eq!(part.detach_at, None);
    assert!(part.detach, "a part can fall off unless it says it cannot");
    assert_eq!(part.mass_kg, DEFAULT_MASS_KG);
}

#[test]
fn a_part_can_be_kept_on_the_car_and_given_a_mass() {
    let kept = ONE.replace("detach_at = 0.9\n", "detach = false\nmass_kg = 12.5\n");
    let part = &car(&kept).unwrap().damage_parts[0];
    assert!(!part.detach);
    assert_eq!((part.mass_kg, part.detach_at), (12.5, None));
}

#[test]
fn the_zone_order_is_the_games() {
    let all = ["front", "rear", "left", "right", "front_left", "front_right", "rear_left", "rear_right"];
    for (at, name) in all.iter().enumerate() {
        let zone = DamageZone::parse(name).unwrap();
        assert_eq!((zone.index(), zone.as_str()), (at, *name));
    }
    assert!(DamageZone::parse("roof").is_none());
}

#[test]
fn bad_parts_are_refused() {
    let bad = [
        ONE.replace("zones = [\"front\", \"front_left\"]", "zones = []"),
        ONE.replace("\"front_left\"", "\"front\""),
        ONE.replace("\"front_left\"", "\"roof\""),
        ONE.replace("axis = [0.0, 2.0, 0.0]", "axis = [0.0, 0.0, 0.0]"),
        ONE.replace("angle = -15.0", "angle = 270.0"),
        ONE.replace("[0.04, 0.05, 0.5]", "[0.5, 0.05]"),
        ONE.replace("[0.04, 0.05, 0.5]", "[0.5, 1.5]"),
        ONE.replace("detach_at = 0.9", "detach_at = 0.0"),
        ONE.replace("FASTCOUPE_KIT00_BONNET", "no spaces"),
        ONE.replace("pivot = [0.7, 0.0, 0.9]", "pivot = [0.7, 0.0, 900.0]"),
        ONE.replace("name = \"Bonnet\"", "name = \"\""),
        ONE.replace("angle = -15.0", "angle = -15.0\ncolour = 1"),
        ONE.replace("detach_at = 0.9", "detach = false\ndetach_at = 0.9"),
        ONE.replace("detach_at = 0.9", "mass_kg = 0.0"),
        ONE.replace("detach_at = 0.9", "mass_kg = 90000.0"),
        format!("{ONE}{ONE}"),
    ];
    for text in bad {
        assert!(car(&text).is_err(), "accepted:\n{text}");
    }
}

#[test]
fn too_many_parts_are_refused() {
    let many: String = (0..=MAX_DAMAGE_PARTS).map(|n| ONE.replace("Bonnet", &format!("Part{n}"))).collect();
    assert!(car(&many).is_err());
}

const ACTIVE: &str = "
[damage_parts.active]
offset = [-0.07, 0.0, 0.02]
above_mph = 70.0
below_mph = 40.0
seconds = 0.6
";

#[test]
fn a_part_can_move_with_the_speed() {
    let part = &car(&format!("{ONE}{ACTIVE}")).unwrap().damage_parts[0];
    let active = part.active.unwrap();
    assert_eq!((active.above_mph, active.below_mph, active.seconds), (70.0, 40.0, 0.6));
    assert_eq!(active.offset, [-0.07, 0.0, 0.02]);
    assert!(car(ONE).unwrap().damage_parts[0].active.is_none());
    let no_lower = ACTIVE.replace("below_mph = 40.0\n", "");
    assert_eq!(car(&format!("{ONE}{no_lower}")).unwrap().damage_parts[0].active.unwrap().below_mph, 70.0);
}

const TURNING: &str = "
[damage_parts.active]
rotation_axis = [0.0, 2.0, 0.0]
rotation_deg = 30.5
above_mph = 70.0
below_mph = 40.0
seconds = 0.6
";

#[test]
fn an_active_part_may_only_turn() {
    let part = &car(&format!("{ONE}{TURNING}")).unwrap().damage_parts[0];
    let active = part.active.unwrap();
    assert_eq!(active.rotation, Some(([0.0, 1.0, 0.0], 30.5)), "the axis is normalised");
    assert_eq!(active.offset, [0.0; 3]);
    let both = ACTIVE.replace("seconds", "rotation_axis = [0.0, 1.0, 0.0]\nrotation_deg = -10.0\nseconds");
    assert_eq!(car(&format!("{ONE}{both}")).unwrap().damage_parts[0].active.unwrap().rotation.unwrap().1, -10.0);
    assert!(car(&format!("{ONE}{ACTIVE}")).unwrap().damage_parts[0].active.unwrap().rotation.is_none());
}

#[test]
fn bad_rotations_are_refused() {
    let bad = [
        TURNING.replace("rotation_deg = 30.5\n", ""),
        TURNING.replace("rotation_axis = [0.0, 2.0, 0.0]\n", ""),
        TURNING.replace("[0.0, 2.0, 0.0]", "[0.0, 0.0, 0.0]"),
        TURNING.replace("30.5", "0.0"),
        TURNING.replace("30.5", "270.0"),
        TURNING.replace("rotation_deg = 30.5\n", "").replace("rotation_axis = [0.0, 2.0, 0.0]\n", ""),
    ];
    for table in bad {
        assert!(car(&format!("{ONE}{table}")).is_err(), "accepted:{table}");
    }
}

#[test]
fn a_vent_waits_before_it_opens_and_before_it_closes() {
    let vent = ACTIVE.replace("seconds = 0.6", "seconds = 0.6\nopen_after_seconds = 3.0\nclose_after_seconds = 5.0");
    let active = car(&format!("{ONE}{vent}")).unwrap().damage_parts[0].active.unwrap();
    assert_eq!(active.delay, Some((3.0, 5.0)));
    for bad in [
        ACTIVE.replace("seconds = 0.6", "seconds = 0.6\nopen_after_seconds = 3.0"),
        ACTIVE.replace("seconds = 0.6", "seconds = 0.6\nopen_after_seconds = -1.0\nclose_after_seconds = 5.0"),
        ACTIVE
            .replace("seconds = 0.6", "seconds = 0.6\ngas = true\nopen_after_seconds = 1.0\nclose_after_seconds = 5.0"),
    ] {
        assert!(car(&format!("{ONE}{bad}")).is_err(), "accepted:{bad}");
    }
}

const DRIVE: &str = "
[[damage_parts.drive]]
open = [{ input = \"speed_mph\", op = \">\", value = 50.0 }, { input = \"steering\", op = \">\", value = 0.3 }]
close = [{ input = \"speed_mph\", op = \"<=\", value = 50.0 }, { input = \"steering\", op = \"<=\", value = 0.3 }]
close_any = true
open_after_seconds = 0.4
";

#[test]
fn a_part_can_be_played_out_by_conditions() {
    let flap = ACTIVE.replace("above_mph = 70.0", "above_mph = 0.0").replace("below_mph = 40.0", "below_mph = 0.0");
    let part = &car(&format!("{ONE}{flap}{DRIVE}")).unwrap().damage_parts[0];
    let drive = &part.drives[0];
    assert_eq!((drive.open.len(), drive.open_any, drive.open_after_seconds, drive.close_any), (2, false, 0.4, true));
    assert_eq!((drive.open[1].input, drive.open[1].op, drive.open[1].value), (DriveInput::Steering, DriveOp::Gt, 0.3));
    assert_eq!(drive.close[0].op, DriveOp::Le);
}

#[test]
fn bad_drives_are_refused() {
    let flap = ACTIVE.replace("above_mph = 70.0", "above_mph = 0.0").replace("below_mph = 40.0", "below_mph = 0.0");
    let bad = [
        format!("{ONE}{DRIVE}"),
        format!("{ONE}{flap}{}", DRIVE.replace("speed_mph", "rpm")),
        format!("{ONE}{flap}{}", DRIVE.replace("\"<=\"", "\"=>\"")),
        format!("{ONE}{flap}{}", DRIVE.replace("open_after_seconds = 0.4", "open_after_seconds = -1.0")),
        format!("{ONE}{flap}{DRIVE}{DRIVE}{DRIVE}{DRIVE}{DRIVE}"),
        format!("{ONE}{flap}{}", DRIVE.replace("close = [{ input = \"speed_mph\", op = \"<=\", value = 50.0 }, { input = \"steering\", op = \"<=\", value = 0.3 }]", "close = []")),
    ];
    for text in bad {
        assert!(car(&text).is_err(), "accepted: {text}");
    }
}

#[test]
fn a_part_that_only_moves_with_the_speed_has_no_damage_turn() {
    let still = ONE.replace("angle = -15.0", "angle = 0.0");
    assert_eq!(car(&format!("{still}{ACTIVE}")).unwrap().damage_parts[0].angle, 0.0);
}

#[test]
fn bad_active_tables_are_refused() {
    let bad = [
        ACTIVE.replace("[-0.07, 0.0, 0.02]", "[0.0, 0.0, 0.0]"),
        ACTIVE.replace("[-0.07, 0.0, 0.02]", "[9.0, 0.0, 0.0]"),
        ACTIVE.replace("below_mph = 40.0", "below_mph = 80.0"),
        ACTIVE.replace("above_mph = 70.0", "above_mph = 900.0"),
        ACTIVE.replace("seconds = 0.6", "seconds = 0.0"),
        ACTIVE.replace("seconds = 0.6", "seconds = 60.0"),
        format!("{ACTIVE}colour = 1\n"),
    ];
    for table in bad {
        assert!(car(&format!("{ONE}{table}")).is_err(), "accepted:{table}");
    }
}

fn hung(parent: &str) -> String {
    let child = ONE.replace("Bonnet", "Flap");
    format!("{child}parent = \"{parent}\"{ACTIVE}")
}

#[test]
fn a_part_may_hang_from_an_earlier_active_part() {
    let text = format!("{ONE}{ACTIVE}{}", hung("Bonnet"));
    let parts = car(&text).unwrap().damage_parts;
    assert_eq!(parts[1].parent.as_deref(), Some("Bonnet"));
    assert_eq!(parts[0].parent, None);
}

#[test]
fn a_bad_parent_is_refused() {
    let child = ONE.replace("Bonnet", "Flap");
    let missing = format!("{ONE}{ACTIVE}{}", hung("Nothing"));
    let first = format!("{}{ONE}{ACTIVE}", hung("Bonnet"));
    let parent_still = format!("{ONE}{}", hung("Bonnet"));
    let child_still = format!("{ONE}{ACTIVE}{child}parent = \"Bonnet\"");
    for text in [missing, first, parent_still, child_still] {
        assert!(car(&text).is_err(), "accepted: {text}");
    }
}

const WING: &str = "
[wing]
open_mph = 140.0
brake_mph = 125.0
top_mph = 200.0
brake_hold_seconds = 0.4
fade_seconds = 0.25
slot_seconds = [1.5, 3.0, 1.6, 3.0, 1.5]
";

const TRACK: &str = "
[[damage_parts.track]]
slot = 0
translation = [[0.0, 0.0, 0.0], [0.0, 0.0, 0.1]]
rotation = [[0.0, 0.0, 0.0, 1.0], [0.0, 0.0, 0.2588, 0.9659]]
";

#[test]
fn a_part_can_be_moved_by_the_keys_of_a_wing_machine() {
    let manifest = car(&format!("{ONE}{TRACK}{WING}")).unwrap();
    let wing = manifest.wing.expect("the wing");
    assert_eq!(
        (wing.open_mph, wing.brake_mph, wing.top_mph, wing.brake_hold_seconds),
        (140.0, 125.0, Some(200.0), 0.4)
    );
    let no_top = WING.replace("top_mph = 200.0\n", "");
    assert_eq!(car(&format!("{ONE}{TRACK}{no_top}")).unwrap().wing.unwrap().top_mph, None);
    assert_eq!(wing.slot_seconds, [1.5, 3.0, 1.6, 3.0, 1.5]);
    let track = &manifest.damage_parts[0].tracks[0];
    assert_eq!((track.slot, track.translation.len(), track.rotation.len()), (0, 2, 2));
    assert!(car(ONE).unwrap().wing.is_none());
}

#[test]
fn a_part_may_hang_from_a_part_the_wing_moves() {
    let child = ONE.replace("Bonnet", "Arm");
    let text = format!("{ONE}{TRACK}{child}parent = \"Bonnet\"{TRACK}{WING}");
    assert_eq!(car(&text).unwrap().damage_parts[1].parent.as_deref(), Some("Bonnet"));
}

#[test]
fn bad_wings_and_tracks_are_refused() {
    let bad = [
        format!("{ONE}{TRACK}"),
        format!("{ONE}{WING}"),
        format!("{ONE}{TRACK}{}", WING.replace("brake_mph = 125.0", "brake_mph = 150.0")),
        format!("{ONE}{TRACK}{}", WING.replace("fade_seconds = 0.25", "fade_seconds = 0.0")),
        format!("{ONE}{TRACK}{}", WING.replace("[1.5, 3.0, 1.6, 3.0, 1.5]", "[1.5, 3.0, 1.6, 3.0, 0.0]")),
        format!("{ONE}{}{WING}", TRACK.replace("slot = 0", "slot = 5")),
        format!("{ONE}{}{WING}", TRACK.replace("[0.0, 0.0, 0.2588, 0.9659]", "[0.0, 0.0, 0.5, 0.5]")),
        format!("{ONE}{}{WING}", TRACK.replace(", [0.0, 0.0, 0.1]]", "]")),
        format!("{ONE}{TRACK}{TRACK}{WING}"),
        format!("{ONE}{ACTIVE}{TRACK}{WING}"),
    ];
    for text in bad {
        assert!(car(&text).is_err(), "accepted: {text}");
    }
}

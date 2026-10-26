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
        ONE.replace("angle = -15.0", "angle = 0.0"),
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

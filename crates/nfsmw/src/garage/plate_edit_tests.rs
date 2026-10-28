use super::*;

fn plate() -> Plate {
    Plate {
        text: Some("NEED4SPD".into()),
        picture: "CAR_PLATE".into(),
        glyphs: "ABCDEFGHIJKLMNOPQRSTUVWXYZ 123456789".chars().collect(),
        columns: 9,
        rows: 4,
        slots: 8,
        aliases: vec![('0', 'O')],
        designs: Some("designs.toml".into()),
        ..Default::default()
    }
}

fn designs() -> Vec<PlateDesignChoice> {
    vec![
        PlateDesignChoice { id: "1863272".into(), name: "Carbon".into(), group: None },
        PlateDesignChoice { id: "77".into(), name: "Flag Of Nowhere".into(), group: Some("Flags".into()) },
    ]
}

#[test]
fn a_car_without_a_plate_has_nothing_to_change() {
    // A stock car, and a car the registry does not know, have no plate: the catalogue says so.
    assert!(supported(&Catalog::default(), "NO_SUCH_CAR_ANYWHERE").is_none());
    let flagged = Catalog { plate: true, ..Catalog::default() };
    assert!(supported(&flagged, "NO_SUCH_CAR_ANYWHERE").is_none(), "the plate table itself is missing");
}

#[test]
fn typed_command_text_keeps_the_garage_rules() {
    let own = plate();
    assert_eq!(checked(&own, "velocity"), Ok("VELOCITY".to_owned()), "lower case is drawn as upper case");
    assert_eq!(checked(&own, "R0AD"), Ok("ROAD".to_owned()), "the alias draws a 0 as an O");
    assert_eq!(checked(&own, "A B"), Ok("A B".to_owned()), "the blank glyph is allowed");
    assert!(checked(&own, "NINECHARS").is_err(), "more than the 8 slots");
    let error = checked(&own, "AB#").unwrap_err();
    assert!(error.contains('#'), "the character the sheet lacks is named: {error}");
}

#[test]
fn a_field_keeps_only_what_the_plate_draws_up_to_its_slots() {
    let own = plate();
    assert_eq!(typed(&own, "ab-c d!"), "ABC D");
    assert_eq!(typed(&own, "ABCDEFGHIJ"), "ABCDEFGH");
    assert_eq!(typed(&own, ""), "");
}

#[test]
fn a_blank_text_gives_the_car_its_own_plate_back() {
    let own = plate();
    assert_eq!(chosen(&own, "FAST"), Some("FAST".to_owned()));
    assert_eq!(chosen(&own, "   "), None);
    assert_eq!(chosen(&own, ""), None);
}

#[test]
fn the_worn_text_is_the_session_one_else_the_manifest_one() {
    let own = plate();
    assert_eq!(worn(&own, "NO_SUCH_CAR_ANYWHERE", Some("SESSION")), "SESSION");
    assert_eq!(worn(&own, "NO_SUCH_CAR_ANYWHERE", None), "NEED4SPD");
    let blank = Plate { text: None, ..own };
    assert_eq!(worn(&blank, "NO_SUCH_CAR_ANYWHERE", None), "");
}

#[test]
fn a_design_is_picked_by_id_by_name_or_as_none() {
    let list = designs();
    assert_eq!(design_id(&list, "77"), Ok("77".to_owned()));
    assert_eq!(design_id(&list, "carbon"), Ok("1863272".to_owned()), "a name, case ignored");
    assert_eq!(design_id(&list, "Flag Of Nowhere"), Ok("77".to_owned()));
    assert_eq!(design_id(&list, "NONE"), Ok(String::new()), "the plain plate");
    assert_eq!(design_id(&list, "plain"), Ok(String::new()));
    assert!(design_id(&list, "9999").is_err());
    assert!(design_id(&[], "77").is_err(), "a car with no designs knows none");
}

#[test]
fn the_design_name_follows_the_pick_and_falls_back_to_the_plain_plate() {
    let own = plate();
    let catalog = Catalog { plate: true, plate_designs: designs(), ..Catalog::default() };
    assert_eq!(design_name(&own, &catalog, "NO_SUCH_CAR_ANYWHERE", Some("77")), "Flag Of Nowhere");
    assert_eq!(design_name(&own, &catalog, "NO_SUCH_CAR_ANYWHERE", Some("")), PLAIN, "picked on purpose");
    assert_eq!(design_name(&own, &catalog, "NO_SUCH_CAR_ANYWHERE", Some("gone")), PLAIN, "an id the list lacks");
    assert_eq!(design_name(&own, &catalog, "NO_SUCH_CAR_ANYWHERE", None), PLAIN, "no default");
    let defaulted = Plate { default_design: Some("1863272".into()), ..own };
    assert_eq!(design_name(&defaulted, &catalog, "NO_SUCH_CAR_ANYWHERE", None), "Carbon");
}

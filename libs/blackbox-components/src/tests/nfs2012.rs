//! A car of the 2012 kind: body and tyre tiers that change numbers and behaviour together. The upgrade names and
//! numbers are illustrative; the 2012 files name the categories (`Body`, `Tyre`...) but not their contents.

use super::{catalog, vehicle};
use crate::{Catalog, Note, OptionState, Pick, Problem, Selection, VehicleDef};

const CATALOG: &str = r#"
[[component]]
id = "body.stock"
[[component]]
id = "body.lightweight"
tier = 1
effects = [
  { kind = "param", key = "mass_kg", op = "mul", value = 0.9 },
  { kind = "behaviour", domain = "damage.profile", select = "fragile" },
]
[[component]]
id = "body.impact"
tier = 1
effects = [
  { kind = "param", key = "mass_kg", op = "mul", value = 1.1 },
  { kind = "behaviour", domain = "damage.profile", select = "armoured" },
]
[[component]]
id = "body.offroad"
tier = 1
effects = [
  { kind = "param", key = "ride_height_m", op = "add", value = 0.05 },
  { kind = "param", key = "mass_kg", op = "mul", value = 1.05 },
]

[[component]]
id = "tyres.stock"
effects = [{ kind = "part", show = ["tyre.stock"] }]
[[component]]
id = "tyres.slick"
when = [{ kind = "is_not", slot = "body", any_of = ["body.offroad"] }]
effects = [
  { kind = "param", key = "grip", op = "mul", value = 1.15 },
  { kind = "behaviour", domain = "tyre.compound", select = "slick" },
]
[[component]]
id = "tyres.offroad"
effects = [
  { kind = "param", key = "grip", op = "mul", value = 0.95 },
  { kind = "behaviour", domain = "tyre.compound", select = "offroad" },
]
[[component]]
id = "tyres.inflatable"
when = [{ kind = "is", slot = "body", any_of = ["body.offroad"] }]
effects = [
  { kind = "part", show = ["tyre.inflatable"], hide = ["tyre.stock"] },
  { kind = "param", key = "grip", op = "mul", value = 0.9 },
  { kind = "param", key = "buoyant", op = "set", value = true },
  { kind = "behaviour", domain = "tyre.compound", select = "inflatable" },
]
[[component]]
id = "tyres.drift"
effects = [
  { kind = "behaviour", domain = "tyre.compound", select = "drift" },
  { kind = "behaviour", domain = "handling.model", select = "drift" },
  { kind = "param", key = "drift_assist", op = "set", value = true },
]
"#;

const VEHICLE: &str = r#"
id = "sedan12"
parts = ["chassis"]
[[slot]]
id = "body"
group = "body"
accepts = ["body.stock", "body.lightweight", "body.impact", "body.offroad"]
default = "body.stock"
[[slot]]
id = "tyres"
group = "tyres"
accepts = ["tyres.stock", "tyres.slick", "tyres.offroad", "tyres.inflatable", "tyres.drift"]
default = "tyres.stock"
"#;

fn fixture() -> (Catalog, VehicleDef) {
    let (catalog, vehicle) = (catalog(CATALOG), vehicle(VEHICLE));
    vehicle.check(&catalog).expect("the fixture is a valid definition");
    (catalog, vehicle)
}

fn pick(body: &str, tyres: &str) -> Selection {
    let mut selection = Selection::default();
    selection.set("body", Pick::of(body));
    selection.set("tyres", Pick::of(tyres));
    selection
}

#[test]
fn a_body_and_a_tyre_change_numbers_and_behaviour_together() {
    let (catalog, car) = fixture();
    let resolved = car.resolve(&catalog, &pick("body.lightweight", "tyres.slick")).unwrap();
    assert_eq!(resolved.params.apply("mass_kg", 1500.0), 1350.0);
    assert_eq!(resolved.params.apply("grip", 1.2), 1.2 * 1.15);
    assert_eq!(resolved.behaviours["damage.profile"], "fragile");
    assert_eq!(resolved.behaviours["tyre.compound"], "slick");
    assert!(resolved.notes.is_empty());
}

#[test]
fn drift_tyres_pick_two_behaviours_and_set_a_flag() {
    let (catalog, car) = fixture();
    let resolved = car.resolve(&catalog, &pick("body.stock", "tyres.drift")).unwrap();
    assert_eq!(resolved.behaviours["handling.model"], "drift");
    assert!(resolved.params.flag("drift_assist"));
    assert!(!resolved.params.flag("buoyant"));
}

#[test]
fn offroad_body_excludes_slick_tyres_and_allows_inflatables() {
    let (catalog, car) = fixture();
    assert!(matches!(
        car.validate(&catalog, &pick("body.offroad", "tyres.slick")).as_slice(),
        [Problem::ConditionFailed { slot, component, .. }] if slot == "tyres" && component == "tyres.slick"
    ));
    assert!(matches!(
        car.validate(&catalog, &pick("body.stock", "tyres.inflatable")).as_slice(),
        [Problem::ConditionFailed { component, .. }] if component == "tyres.inflatable"
    ));
    let boat = car.resolve(&catalog, &pick("body.offroad", "tyres.inflatable")).unwrap();
    assert!(boat.parts.contains("tyre.inflatable") && !boat.parts.contains("tyre.stock"));
    assert!(boat.params.flag("buoyant"));
    assert!((boat.params.apply("ride_height_m", 0.12) - 0.17).abs() < 1e-12);
}

#[test]
fn changing_the_body_resets_the_tyres_that_needed_it() {
    let (catalog, car) = fixture();
    let mut selection = pick("body.offroad", "tyres.inflatable");
    selection.set("body", Pick::of("body.impact"));
    let repaired = car.normalize(&catalog, &selection);
    assert_eq!(repaired.get("tyres"), None);
    assert_eq!(repaired.get("body"), Some(&Pick::of("body.impact")));
}

#[test]
fn the_screen_shows_why_an_option_is_blocked() {
    let (catalog, car) = fixture();
    let views = car.slot_views(&catalog, &pick("body.offroad", "tyres.stock"), &());
    let tyres = views.iter().find(|view| view.slot == "tyres").unwrap();
    let state = |id: &str| tyres.options.iter().find(|o| o.component == id).unwrap().state.clone();
    assert!(matches!(state("tyres.slick"), OptionState::Blocked(conditions) if conditions.len() == 1));
    assert_eq!(state("tyres.inflatable"), OptionState::Available);
    assert_eq!(state("tyres.stock"), OptionState::Selected);
}

#[test]
fn two_components_naming_one_behaviour_are_a_note_not_a_failure() {
    let (catalog, car) = fixture();
    // The tyres and the body both pick a damage profile only in this synthetic case.
    let mut catalog = catalog;
    catalog
        .insert(crate::Component {
            id: "tyres.armoured".into(),
            name: String::new(),
            tier: 0,
            effects: vec![crate::Effect::Behaviour { domain: "damage.profile".into(), select: "armoured".into() }],
            when: vec![],
            price: 0,
            unlock: None,
        })
        .unwrap();
    let mut car = car;
    car.slots[1].accepts.push(toml::from_str::<crate::Offer>("component = \"tyres.armoured\"").unwrap());
    let resolved = car.resolve(&catalog, &pick("body.lightweight", "tyres.armoured")).unwrap();
    assert_eq!(resolved.behaviours["damage.profile"], "armoured", "the later slot wins");
    assert!(matches!(resolved.notes.as_slice(), [Note::BehaviourConflict { domain, slots }]
        if domain == "damage.profile" && slots == &["body", "tyres"]));
}

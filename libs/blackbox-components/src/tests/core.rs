//! The rules of the library on small definitions: composition order, notes, the checks a mod author sees, diffs.

use super::{catalog, vehicle};
use crate::{Catalog, Change, Input, Note, Op, Pick, Selection, Value, VehicleDef};

const CATALOG: &str = r#"
[[component]]
id = "a.stock"
effects = [{ kind = "part", show = ["p.a"] }]
[[component]]
id = "a.hot"
effects = [
  { kind = "param", key = "x", op = "set", value = 120 },
  { kind = "param", key = "x", op = "add", value = 10 },
  { kind = "param", key = "list", op = "set", value = [1.0, 2.0, 3.0] },
  { kind = "part", show = ["p.hot"] },
]
[[component]]
id = "b.stock"
[[component]]
id = "b.mul"
effects = [
  { kind = "param", key = "x", op = "mul", value = 1.1 },
  { kind = "param", key = "x", op = "min", value = 130 },
  { kind = "param", key = "x", op = "max", value = 50 },
  { kind = "param", key = "list", op = "mul", value = 2 },
  { kind = "part", hide = ["p.hot"] },
]
[[component]]
id = "b.set"
effects = [{ kind = "param", key = "x", op = "set", value = 7 }]
"#;

const VEHICLE: &str = r#"
id = "v"
parts = ["base"]
[[slot]]
id = "a"
accepts = ["a.stock", "a.hot"]
default = "a.stock"
[[slot]]
id = "b"
accepts = ["b.stock", "b.mul", "b.set"]
default = "b.stock"
"#;

fn fixture() -> (Catalog, VehicleDef) {
    (catalog(CATALOG), vehicle(VEHICLE))
}

fn pick(a: &str, b: &str) -> Selection {
    let mut selection = Selection::default();
    selection.set("a", Pick::of(a));
    selection.set("b", Pick::of(b));
    selection
}

#[test]
fn modifiers_compose_as_set_then_add_then_mul_then_limits() {
    let (catalog, car) = fixture();
    let resolved = car.resolve(&catalog, &pick("a.hot", "b.mul")).unwrap();
    // (120 + 10) * 1.1 = 143, capped at 130.
    assert_eq!(resolved.params.apply("x", 100.0), 130.0);
    // The base only matters when nothing sets the key.
    assert_eq!(resolved.params.apply("y", 5.0), 5.0);
    let without_set = car.resolve(&catalog, &pick("a.stock", "b.mul")).unwrap();
    assert_eq!(without_set.params.apply("x", 100.0), 110.00000000000001);
    assert_eq!(without_set.params.apply("x", 10.0), 50.0, "the floor");
}

#[test]
fn lists_are_replaced_by_a_set_and_scaled_element_wise() {
    let (catalog, car) = fixture();
    let resolved = car.resolve(&catalog, &pick("a.hot", "b.mul")).unwrap();
    assert_eq!(resolved.params.apply_list("list", &[9.0]), [2.0, 4.0, 6.0]);
    assert_eq!(resolved.params.apply_list("other", &[9.0]), [9.0]);
}

#[test]
fn a_part_hidden_by_another_component_stays_hidden_and_says_so() {
    let (catalog, car) = fixture();
    let resolved = car.resolve(&catalog, &pick("a.hot", "b.mul")).unwrap();
    assert!(!resolved.parts.contains("p.hot"));
    assert!(resolved.notes.contains(&Note::PartHidden { part: "p.hot".into(), hidden_by: "b".into() }));
}

#[test]
fn two_sets_of_one_key_keep_the_later_slot_and_say_so() {
    let (catalog, car) = fixture();
    let resolved = car.resolve(&catalog, &pick("a.hot", "b.set")).unwrap();
    // The later set (7) wins; the add of the first component still applies: 7 + 10.
    assert_eq!(resolved.params.apply("x", 0.0), 17.0);
    assert!(resolved.notes.contains(&Note::SetConflict { key: "x".into(), slots: vec!["a".into(), "b".into()] }));
}

#[test]
fn resolution_is_deterministic() {
    let (catalog, car) = fixture();
    let selection = pick("a.hot", "b.mul");
    let first = car.resolve(&catalog, &selection).unwrap();
    for _ in 0..8 {
        assert_eq!(car.resolve(&catalog, &selection).unwrap(), first);
    }
}

#[test]
fn canonical_forms_drop_defaults_and_diffs_replay() {
    let (catalog, car) = fixture();
    let mut selection = pick("a.stock", "b.mul");
    assert_eq!(selection.canonical(&car).len(), 1);
    assert_eq!(selection.canonical(&car), car.normalize(&catalog, &selection));

    let to = pick("a.hot", "b.stock").canonical(&car);
    let from = selection.canonical(&car);
    let changes = from.diff(&to);
    assert_eq!(changes.len(), 2);
    assert!(changes.contains(&Change::Reset { slot: "b".into() }));
    selection = from.clone();
    selection.apply(&changes);
    assert_eq!(selection.canonical(&car), to);
    assert!(to.diff(&to).is_empty());
}

#[test]
fn input_defaults_are_dropped_from_the_canonical_form() {
    let car = vehicle(
        r#"
id = "v"
[[slot]]
id = "paint"
accepts = ["a.stock"]
default = "a.stock"
inputs = [{ kind = "rgb", key = "colour", default = [1, 2, 3] }]
"#,
    );
    let mut selection = Selection::default();
    selection.set("paint", Pick::of("a.stock").with("colour", Input::Rgb([1, 2, 3])));
    assert!(selection.canonical(&car).is_empty());
    selection.set("paint", Pick::of("a.stock").with("colour", Input::Rgb([9, 2, 3])));
    assert_eq!(selection.canonical(&car).len(), 1);
}

#[test]
fn bad_inputs_are_refused() {
    let car = vehicle(
        r#"
id = "v"
[[slot]]
id = "s"
accepts = ["a.stock"]
default = "a.stock"
inputs = [{ kind = "num", key = "amount", default = 0.5, min = 0, max = 1 }, { kind = "text", key = "label", default = "", max_len = 3 }]
"#,
    );
    let catalog = catalog(CATALOG);
    for (key, input) in [
        ("amount", Input::Num(2.0)),
        ("amount", Input::Num(f64::NAN)),
        ("amount", Input::Text("x".into())),
        ("label", Input::Text("four".into())),
        ("nothing", Input::Num(0.0)),
    ] {
        let mut selection = Selection::default();
        selection.set("s", Pick::of("a.stock").with(key, input));
        assert_eq!(car.validate(&catalog, &selection).len(), 1, "{key}");
    }
}

#[test]
fn an_empty_slot_needs_permission() {
    let car = vehicle(
        r#"
id = "v"
[[slot]]
id = "fixed"
accepts = ["a.stock"]
default = "a.stock"
[[slot]]
id = "free"
accepts = ["b.mul"]
"#,
    );
    let catalog = catalog(CATALOG);
    car.check(&catalog).unwrap();
    let mut selection = Selection::default();
    selection.set("fixed", Pick::empty());
    assert_eq!(car.validate(&catalog, &selection).len(), 1);
    assert!(car.validate(&catalog, &Selection::default()).is_empty());
}

#[test]
fn the_author_sees_every_mistake_of_a_definition() {
    let catalog = catalog(CATALOG);
    let car = vehicle(
        r#"
id = "bad"
[[slot]]
id = "s"
accepts = ["a.stock", "missing", "a.stock"]
default = "b.stock"
when = [{ kind = "filled", slot = "later" }]
inputs = [{ kind = "num", key = "n", default = 5, min = 0, max = 1 }]
[[slot]]
id = "later"
accepts = ["a.hot"]
default = "a.hot"
when = [{ kind = "is", slot = "s", any_of = ["nowhere"] }]
[[slot]]
id = "s"
accepts = ["b.mul"]
"#,
    );
    let errors = car.check(&catalog).unwrap_err().join("\n");
    for expected in [
        "not an earlier slot",
        "the default b.stock is not accepted",
        "missing: not in the catalog",
        "accepted twice",
        "outside 0 to 1",
        "repeated",
        "nowhere, which is not in the catalog",
    ] {
        assert!(errors.contains(expected), "{expected} in\n{errors}");
    }
}

#[test]
fn a_default_with_rules_and_an_unbound_colour_are_refused() {
    let mut catalog = catalog(CATALOG);
    catalog
        .insert(crate::Component {
            id: "c.colour".into(),
            name: String::new(),
            tier: 0,
            effects: vec![crate::Effect::Surface {
                target: "t".into(),
                texture: None,
                finish: None,
                colours: [("base".to_string(), crate::ColourSource::Input("nope".into()))].into(),
            }],
            when: vec![crate::Condition::Filled { slot: "a".into() }],
            price: 0,
            unlock: None,
        })
        .unwrap();
    let car = vehicle(
        r#"
id = "v"
[[slot]]
id = "a"
accepts = ["a.stock"]
default = "a.stock"
[[slot]]
id = "c"
accepts = ["c.colour"]
default = "c.colour"
"#,
    );
    let errors = car.check(&catalog).unwrap_err().join("\n");
    assert!(errors.contains("must have no rules") && errors.contains("input nope"), "{errors}");
}

#[test]
fn bad_numbers_and_unknown_keys_are_refused() {
    let mut catalog = catalog(CATALOG);
    let nan = crate::Effect::Param { key: "k".into(), op: Op::Mul, value: Value::Num(f64::NAN) };
    let text = crate::Effect::Param { key: "k".into(), op: Op::Add, value: Value::Text("x".into()) };
    for effect in [nan, text] {
        let component =
            crate::Component { id: format!("bad{}", catalog.iter().count()), effects: vec![effect], ..empty() };
        let id = component.id.clone();
        catalog.insert(component).unwrap();
        let car = VehicleDef { id: "v".into(), parts: Default::default(), slots: vec![slot_of(&id)] };
        assert!(car.check(&catalog).is_err(), "{id}");
    }
    assert!(toml::from_str::<Catalog>("[[component]]\nid = \"x\"\nbogus = 1\n").is_err());
    assert!(toml::from_str::<Catalog>("[[component]]\nid = \"x\"\n[[component]]\nid = \"x\"\n").is_err());
    assert!(toml::from_str::<VehicleDef>("id = \"v\"\n[[slot]]\nid = \"s\"\naccepts = [\"x\"]\nbogus = 1\n").is_err());
    assert!(toml::from_str::<Catalog>("[[component]]\nid = \"x\"\neffects = [{ kind = \"param\", key = \"k\", op = \"mul\", value = 1, extra = 1 }]\n").is_err());
}

fn empty() -> crate::Component {
    crate::Component {
        id: String::new(),
        name: String::new(),
        tier: 0,
        effects: vec![],
        when: vec![],
        price: 0,
        unlock: None,
    }
}

fn slot_of(component: &str) -> crate::Slot {
    crate::Slot {
        id: "s".into(),
        group: String::new(),
        accepts: vec![crate::Offer { component: component.into(), price: None, when: vec![], bundle: vec![] }],
        default: None,
        optional: false,
        when: vec![],
        inputs: vec![],
    }
}

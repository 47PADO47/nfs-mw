//! A car of the 2005 kind: every car offers the same slots. Performance packages are tiers whose effect is the package
//! level the game's own tables resolve; rims, kits, spoilers, paints and vinyls are looks.

use super::{catalog, vehicle};
use crate::{Catalog, Input, OptionState, Pick, Problem, Selection, Value, VehicleDef};

const CATALOG: &str = r#"
[[component]]
id = "engine.stock"
[[component]]
id = "engine.stage1"
tier = 1
price = 1500
effects = [{ kind = "param", key = "package.engine", op = "set", value = 1 }]
[[component]]
id = "engine.stage2"
tier = 2
price = 3000
effects = [{ kind = "param", key = "package.engine", op = "set", value = 2 }]

[[component]]
id = "induction.none"
[[component]]
id = "induction.turbo"
tier = 1
price = 4000
unlock = "career.heat3"
when = [{ kind = "tier", slot = "engine", at_least = 1 }]
effects = [{ kind = "param", key = "package.induction", op = "set", value = 1 }]
[[component]]
id = "junkman.off"
[[component]]
id = "junkman.on"
effects = [{ kind = "param", key = "package.induction.junkman", op = "set", value = true }]

[[component]]
id = "rims.stock"
effects = [{ kind = "part", show = ["wheel.stock"] }]
[[component]]
id = "rims.five_spoke"
price = 900
effects = [{ kind = "part", show = ["wheel.five_spoke"] }]
[[component]]
id = "rimpaint.factory"
[[component]]
id = "rimpaint.custom"
effects = [{ kind = "surface", target = "rim", colours = { base = "colour" } }]

[[component]]
id = "bodykit.stock"
effects = [{ kind = "part", show = ["body.stock"] }]
[[component]]
id = "bodykit.wide"
price = 2500
effects = [
  { kind = "part", show = ["body.wide"], hide = ["spoiler.gt"] },
  { kind = "param", key = "aero.tuning_unlocked", op = "set", value = true },
]
[[component]]
id = "spoiler.none"
[[component]]
id = "spoiler.gt"
price = 1200
when = [{ kind = "is_not", slot = "bodykit", any_of = ["bodykit.wide"] }]
effects = [
  { kind = "part", show = ["spoiler.gt"] },
  { kind = "param", key = "aero.tuning_unlocked", op = "set", value = true },
]

[[component]]
id = "paint.gloss"
effects = [{ kind = "surface", target = "body", finish = "gloss", colours = { base = "colour" } }]
[[component]]
id = "paint.metallic"
price = 300
effects = [{ kind = "surface", target = "body", finish = "metallic", colours = { base = "colour" } }]
[[component]]
id = "vinyl.stripe"
price = 500
effects = [{ kind = "surface", target = "body.vinyl", texture = "stripe", colours = { ch1 = "ch1" } }]
[[component]]
id = "vinyl.none"
"#;

const VEHICLE: &str = r#"
id = "coupe05"
parts = ["chassis", "glass"]

[[slot]]
id = "engine"
group = "performance"
accepts = ["engine.stock", "engine.stage1", "engine.stage2"]
default = "engine.stock"
[[slot]]
id = "induction"
group = "performance"
accepts = ["induction.none", "induction.turbo"]
default = "induction.none"
[[slot]]
id = "induction_junkman"
group = "performance"
accepts = ["junkman.off", "junkman.on"]
default = "junkman.off"
when = [{ kind = "is", slot = "induction", any_of = ["induction.turbo"] }]
[[slot]]
id = "rims"
group = "looks"
accepts = ["rims.stock", "rims.five_spoke"]
default = "rims.stock"
[[slot]]
id = "rim_paint"
group = "looks"
accepts = ["rimpaint.factory", "rimpaint.custom"]
default = "rimpaint.factory"
inputs = [{ kind = "rgb", key = "colour", default = [120, 120, 120] }]
[[slot]]
id = "bodykit"
group = "looks"
accepts = ["bodykit.stock", "bodykit.wide"]
default = "bodykit.stock"
[[slot]]
id = "spoiler"
group = "looks"
accepts = ["spoiler.none", "spoiler.gt"]
default = "spoiler.none"
[[slot]]
id = "paint"
group = "looks"
accepts = ["paint.gloss", "paint.metallic"]
default = "paint.gloss"
inputs = [{ kind = "rgb", key = "colour", default = [173, 43, 44] }]
[[slot]]
id = "vinyl"
group = "looks"
accepts = ["vinyl.none", "vinyl.stripe"]
default = "vinyl.none"
inputs = [{ kind = "rgb", key = "ch1", default = [255, 255, 255] }]
"#;

fn fixture() -> (Catalog, VehicleDef) {
    let (catalog, vehicle) = (catalog(CATALOG), vehicle(VEHICLE));
    vehicle.check(&catalog).expect("the fixture is a valid definition");
    (catalog, vehicle)
}

#[test]
fn stock_car_has_stock_parts_and_no_modifiers() {
    let (catalog, car) = fixture();
    let stock = car.resolve(&catalog, &Selection::default()).unwrap();
    let parts: Vec<&str> = stock.parts.iter().map(String::as_str).collect();
    assert_eq!(parts, ["body.stock", "chassis", "glass", "wheel.stock"]);
    assert_eq!(stock.params.iter().count(), 0);
    assert_eq!(stock.installed["engine"].component, "engine.stock");
}

#[test]
fn performance_tier_is_the_package_level_the_game_resolves() {
    let (catalog, car) = fixture();
    let mut selection = Selection::default();
    selection.set("engine", Pick::of("engine.stage2"));
    let car = car.resolve(&catalog, &selection).unwrap();
    assert_eq!(car.params.set_value("package.engine"), Some(&Value::Num(2.0)));
    assert_eq!(car.installed["engine"].tier, 2);
}

#[test]
fn turbo_needs_an_engine_tier_and_a_reset_follows_the_engine() {
    let (catalog, car) = fixture();
    let mut selection = Selection::default();
    selection.set("induction", Pick::of("induction.turbo"));
    let problems = car.validate(&catalog, &selection);
    assert!(matches!(problems.as_slice(), [Problem::ConditionFailed { slot, .. }] if slot == "induction"));

    selection.set("engine", Pick::of("engine.stage1"));
    selection.set("induction_junkman", Pick::of("junkman.on"));
    assert!(car.validate(&catalog, &selection).is_empty());
    assert!(car.resolve(&catalog, &selection).unwrap().params.flag("package.induction.junkman"));

    // Back to the stock engine: the turbo goes, and the junkman choice that hung on the turbo goes with it.
    selection.reset("engine");
    assert_eq!(car.normalize(&catalog, &selection), Selection::default());
}

#[test]
fn junkman_slot_exists_only_with_the_turbo() {
    let (catalog, car) = fixture();
    let views = car.slot_views(&catalog, &Selection::default(), &());
    let junkman = views.iter().find(|view| view.slot == "induction_junkman").unwrap();
    assert!(!junkman.active);
    let mut selection = Selection::default();
    selection.set("induction_junkman", Pick::of("junkman.on"));
    assert!(car.validate(&catalog, &selection).contains(&Problem::InactiveSlot("induction_junkman".into())));
}

#[test]
fn wide_kit_and_gt_spoiler_exclude_each_other() {
    let (catalog, car) = fixture();
    let mut selection = Selection::default();
    selection.set("bodykit", Pick::of("bodykit.wide"));
    selection.set("spoiler", Pick::of("spoiler.gt"));
    assert!(
        matches!(car.validate(&catalog, &selection).as_slice(), [Problem::ConditionFailed { slot, .. }] if slot == "spoiler")
    );
    let repaired = car.normalize(&catalog, &selection);
    let resolved = car.resolve(&catalog, &repaired).unwrap();
    assert!(resolved.parts.contains("body.wide") && !resolved.parts.contains("spoiler.gt"));
    assert!(resolved.params.flag("aero.tuning_unlocked"));
}

#[test]
fn paint_colour_finish_and_vinyl_channels_come_from_the_picks() {
    let (catalog, car) = fixture();
    let mut selection = Selection::default();
    selection.set("paint", Pick::of("paint.metallic").with("colour", Input::Rgb([10, 20, 30])));
    selection.set("vinyl", Pick::of("vinyl.stripe").with("ch1", Input::Rgb([255, 0, 0])));
    let resolved = car.resolve(&catalog, &selection).unwrap();
    let body = &resolved.surfaces["body"];
    assert_eq!((body.finish.as_deref(), body.colours["base"]), (Some("metallic"), [10, 20, 30]));
    let vinyl = &resolved.surfaces["body.vinyl"];
    assert_eq!((vinyl.texture.as_deref(), vinyl.colours["ch1"]), (Some("stripe"), [255, 0, 0]));
    // A colour the player left out is the slot's default.
    selection.set("paint", Pick::of("paint.gloss"));
    assert_eq!(car.resolve(&catalog, &selection).unwrap().surfaces["body"].colours["base"], [173, 43, 44]);
}

#[test]
fn the_shop_quotes_a_change_and_locks_what_the_career_has_not_unlocked() {
    let (catalog, car) = fixture();
    let mut selection = Selection::default();
    selection.set("engine", Pick::of("engine.stage2"));
    selection.set("rims", Pick::of("rims.five_spoke"));
    selection.set("induction", Pick::of("induction.turbo"));
    let quote = car.quote(&catalog, &Selection::default(), &selection);
    assert_eq!(quote.install_total(), 3000 + 900 + 4000);
    assert!(quote.remove.iter().all(|line| line.price == 0));

    let gate = |unlock: &str| unlock != "career.heat3";
    let locked = car.locked(&catalog, &Selection::default(), &selection, &gate);
    assert!(matches!(locked.as_slice(), [Problem::Locked { component, .. }] if component == "induction.turbo"));
    // The same car loaded from a save resolves without asking the gate.
    assert!(car.resolve(&catalog, &selection).is_ok());
    let view = car.slot_views(&catalog, &Selection::default(), &gate);
    let turbo =
        view.iter().find(|v| v.slot == "induction").unwrap().options.iter().find(|o| o.component == "induction.turbo");
    assert!(matches!(turbo.unwrap().state, OptionState::Blocked(_)), "blocked by the engine tier before it is locked");
}

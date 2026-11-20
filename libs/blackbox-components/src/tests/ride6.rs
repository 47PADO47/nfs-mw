//! Bikes of the RIDE 6 kind: one shared catalog, and each bike lists only the slots it has. An exhaust change swaps the
//! muffler part, changes the power and the exhaust sound and flame behaviours; the part carries its own tail pipe marker
//! (the game maps part to marker), so the flame follows the muffler.

use super::{catalog, vehicle};
use crate::{Catalog, Input, Pick, Problem, Selection, VehicleDef};

const CATALOG: &str = r#"
[[component]]
id = "exhaust.stock"
effects = [
  { kind = "part", show = ["muffler.stock"] },
  { kind = "behaviour", domain = "exhaust.sound", select = "stock" },
]
[[component]]
id = "exhaust.race"
tier = 1
price = 900
effects = [
  { kind = "part", show = ["muffler.race"] },
  { kind = "param", key = "engine.power", op = "mul", value = 1.04 },
  { kind = "param", key = "mass_kg", op = "add", value = -1.5 },
  { kind = "behaviour", domain = "exhaust.sound", select = "race" },
  { kind = "behaviour", domain = "exhaust.flames", select = "race" },
]
[[component]]
id = "barpad.stock"
effects = [{ kind = "part", show = ["barpad.stock"] }]
[[component]]
id = "barpad.none"
[[component]]
id = "fork.stock"
[[component]]
id = "fork.stiff"
price = 700
effects = [{ kind = "param", key = "front.spring_n_per_m", op = "mul", value = 1.15 }]
[[component]]
id = "chain.stock"
effects = [{ kind = "part", show = ["chain.stock"] }]
[[component]]
id = "chain.race"
price = 250
effects = [{ kind = "part", show = ["chain.race"] }]
[[component]]
id = "chaincolour.paint"
effects = [{ kind = "surface", target = "chain", colours = { base = "colour" } }]
[[component]]
id = "chaincolour.factory"
"#;

const KTM: &str = r#"
id = "ktm450"
parts = ["chassis", "wheels"]
[[slot]]
id = "exhaust"
accepts = ["exhaust.stock", "exhaust.race"]
default = "exhaust.stock"
[[slot]]
id = "bar_pad"
accepts = ["barpad.stock", "barpad.none"]
default = "barpad.stock"
[[slot]]
id = "fork"
accepts = ["fork.stock", "fork.stiff"]
default = "fork.stock"
"#;

const DUCATI: &str = r#"
id = "panigale_v4s"
parts = ["chassis", "wheels"]
[[slot]]
id = "exhaust"
accepts = ["exhaust.stock", { component = "exhaust.race", price = 1800 }]
default = "exhaust.stock"
[[slot]]
id = "chain"
accepts = ["chain.stock", "chain.race"]
default = "chain.stock"
[[slot]]
id = "chain_colour"
accepts = ["chaincolour.factory", "chaincolour.paint"]
default = "chaincolour.factory"
when = [{ kind = "is", slot = "chain", any_of = ["chain.race"] }]
inputs = [{ kind = "rgb", key = "colour", default = [212, 175, 55] }]
"#;

fn fixture() -> (Catalog, VehicleDef, VehicleDef) {
    let (catalog, ktm, ducati) = (catalog(CATALOG), vehicle(KTM), vehicle(DUCATI));
    ktm.check(&catalog).expect("the KTM is valid");
    ducati.check(&catalog).expect("the Ducati is valid");
    (catalog, ktm, ducati)
}

#[test]
fn each_bike_has_the_slots_it_declares() {
    let (_, ktm, ducati) = fixture();
    let ids = |v: &VehicleDef| v.slots.iter().map(|s| s.id.clone()).collect::<Vec<_>>();
    assert_eq!(ids(&ktm), ["exhaust", "bar_pad", "fork"]);
    assert_eq!(ids(&ducati), ["exhaust", "chain", "chain_colour"]);
}

#[test]
fn a_slot_the_bike_does_not_have_is_refused_not_ignored() {
    let (catalog, ktm, ducati) = fixture();
    let mut selection = Selection::default();
    selection.set("chain", Pick::of("chain.race"));
    assert_eq!(ktm.validate(&catalog, &selection), [Problem::UnknownSlot("chain".into())]);
    assert!(ducati.validate(&catalog, &selection).is_empty());
    // The same for the suspension on the Ducati, and the bar pad.
    for slot in ["fork", "bar_pad"] {
        let mut selection = Selection::default();
        selection.set(slot, Pick::of("fork.stiff"));
        assert_eq!(ducati.validate(&catalog, &selection), [Problem::UnknownSlot(slot.into())]);
    }
    // A component the slot does not list is refused too: the KTM's bar pad takes no fork.
    let mut selection = Selection::default();
    selection.set("bar_pad", Pick::of("fork.stiff"));
    assert!(matches!(ktm.validate(&catalog, &selection).as_slice(), [Problem::NotAccepted { .. }]));
}

#[test]
fn the_ktm_can_drop_its_bar_pad_and_the_ducati_cannot_pad_at_all() {
    let (catalog, ktm, _) = fixture();
    let mut selection = Selection::default();
    selection.set("bar_pad", Pick::of("barpad.none"));
    let resolved = ktm.resolve(&catalog, &selection).unwrap();
    assert!(!resolved.parts.contains("barpad.stock"));
    let stock = ktm.resolve(&catalog, &Selection::default()).unwrap();
    assert!(stock.parts.contains("barpad.stock"));
}

#[test]
fn an_exhaust_swaps_the_muffler_and_changes_power_sound_and_flames() {
    let (catalog, ktm, _) = fixture();
    let stock = ktm.resolve(&catalog, &Selection::default()).unwrap();
    assert!(stock.parts.contains("muffler.stock") && !stock.parts.contains("muffler.race"));
    assert_eq!(stock.behaviours["exhaust.sound"], "stock");
    assert!(!stock.behaviours.contains_key("exhaust.flames"));

    let mut selection = Selection::default();
    selection.set("exhaust", Pick::of("exhaust.race"));
    let race = ktm.resolve(&catalog, &selection).unwrap();
    assert!(race.parts.contains("muffler.race") && !race.parts.contains("muffler.stock"));
    assert_eq!(race.behaviours["exhaust.sound"], "race");
    assert_eq!(race.behaviours["exhaust.flames"], "race");
    assert_eq!(race.params.apply("engine.power", 100.0), 104.0);
    assert_eq!(race.params.apply("mass_kg", 126.0), 124.5);
}

#[test]
fn the_chain_colour_exists_only_with_the_race_chain_and_resets_without_it() {
    let (catalog, _, ducati) = fixture();
    let mut selection = Selection::default();
    selection.set("chain", Pick::of("chain.race"));
    selection.set("chain_colour", Pick::of("chaincolour.paint").with("colour", Input::Rgb([200, 0, 0])));
    let resolved = ducati.resolve(&catalog, &selection).unwrap();
    assert_eq!(resolved.surfaces["chain"].colours["base"], [200, 0, 0]);

    selection.set("chain", Pick::of("chain.stock"));
    assert!(ducati.resolve(&catalog, &selection).is_err());
    let repaired = ducati.normalize(&catalog, &selection);
    assert_eq!(repaired, Selection::default());
}

#[test]
fn a_vehicle_prices_a_component_its_own_way() {
    let (catalog, ktm, ducati) = fixture();
    let mut selection = Selection::default();
    selection.set("exhaust", Pick::of("exhaust.race"));
    assert_eq!(ktm.quote(&catalog, &Selection::default(), &selection).install_total(), 900);
    assert_eq!(ducati.quote(&catalog, &Selection::default(), &selection).install_total(), 1800);
}

#[test]
fn the_digest_tells_bikes_and_catalog_changes_apart() {
    let (catalog, ktm, ducati) = fixture();
    assert_eq!(ktm.digest(&catalog), ktm.digest(&catalog));
    assert_ne!(ktm.digest(&catalog), ducati.digest(&catalog));
    let changed = super::catalog(&CATALOG.replace("1.04", "1.05"));
    assert_ne!(ktm.digest(&catalog), ktm.digest(&changed));
}

#[test]
fn the_catalog_lists_the_names_the_game_must_bind() {
    let used = catalog(CATALOG).used();
    assert!(used.params.contains("engine.power") && used.parts.contains("muffler.race"));
    assert_eq!(used.behaviours["exhaust.sound"].len(), 2);
    assert!(used.surfaces.contains("chain"));
}

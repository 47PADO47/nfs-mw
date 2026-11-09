//! A level that comes with its parts: an offer's `bundle`.

use super::{catalog, vehicle};
use crate::{Pick, Selection, VehicleDef};

const CATALOG: &str = r#"
[[component]]
id = "brakes.stock"
[[component]]
id = "brakes.l1"
tier = 1
[[component]]
id = "brakes.l2"
tier = 2
[[component]]
id = "disc.stock"
[[component]]
id = "disc.kit1"
[[component]]
id = "disc.kit2"
[[component]]
id = "pad.race"
"#;

const VEHICLE: &str = r#"
id = "bike"
[[slot]]
id = "perf_brakes"
accepts = ["brakes.stock", { component = "brakes.l1", bundle = ["disc.kit1"] }, { component = "brakes.l2", bundle = ["disc.kit2", "pad.race"] }]
default = "brakes.stock"
[[slot]]
id = "discs"
accepts = ["disc.stock", "disc.kit1", "disc.kit2"]
default = "disc.stock"
[[slot]]
id = "pads"
accepts = ["pad.race"]
optional = true
"#;

fn bike() -> VehicleDef {
    vehicle(VEHICLE)
}

fn picked(pairs: &[(&str, &str)]) -> Selection {
    let mut selection = Selection::default();
    for (slot, component) in pairs {
        selection.set(slot, Pick::of(component));
    }
    selection
}

/// `to` with its bundle changes applied, canonical: the selection the shop installs.
fn installed(vehicle: &VehicleDef, from: &Selection, to: &Selection) -> Selection {
    let mut result = to.canonical(vehicle);
    result.apply(&vehicle.bundle_changes(from, to));
    result
}

#[test]
fn the_catalog_and_vehicle_are_valid_and_the_bundles_are_read() {
    let vehicle = bike();
    vehicle.check(&catalog(CATALOG)).unwrap();
    let ladder = vehicle.slot("perf_brakes").unwrap();
    assert_eq!(ladder.offer("brakes.l2").unwrap().bundle, ["disc.kit2", "pad.race"]);
    assert!(ladder.offer("brakes.stock").unwrap().bundle.is_empty());
}

#[test]
fn raising_the_level_fits_the_parts_that_come_with_it() {
    let vehicle = bike();
    let result = installed(&vehicle, &Selection::default(), &picked(&[("perf_brakes", "brakes.l2")]));
    assert_eq!(result, picked(&[("perf_brakes", "brakes.l2"), ("discs", "disc.kit2"), ("pads", "pad.race")]));
}

#[test]
fn lowering_the_level_takes_its_parts_off_and_fits_the_lower_levels() {
    let vehicle = bike();
    let l2 = picked(&[("perf_brakes", "brakes.l2"), ("discs", "disc.kit2"), ("pads", "pad.race")]);
    let down = installed(&vehicle, &l2, &picked(&[("perf_brakes", "brakes.l1")]));
    assert_eq!(
        down,
        picked(&[("perf_brakes", "brakes.l1"), ("discs", "disc.kit1")]),
        "kit 2 and the pad are off, kit 1 is on"
    );
    let stock = installed(&vehicle, &l2, &Selection::default());
    assert_eq!(stock, Selection::default(), "level 0: the vehicle is stock again");
}

#[test]
fn a_part_that_is_not_the_bundled_one_stays() {
    let vehicle = bike();
    // The player put a part of their own in the pad slot: lowering the level does not touch what it did not bring.
    let from = picked(&[("perf_brakes", "brakes.l1"), ("discs", "disc.kit1")]);
    let to = picked(&[("discs", "disc.kit2")]);
    let result = installed(&vehicle, &from, &to);
    assert_eq!(result, picked(&[("discs", "disc.kit2")]), "the disc is not the level's any more");
}

#[test]
fn choosing_a_bundled_part_alone_does_not_change_the_level() {
    let vehicle = bike();
    let to = picked(&[("discs", "disc.kit2")]);
    assert!(vehicle.bundle_changes(&Selection::default(), &to).is_empty());
}

#[test]
fn nothing_changes_when_nothing_that_changed_has_a_bundle() {
    let vehicle = bike();
    let from = picked(&[("perf_brakes", "brakes.l2"), ("discs", "disc.kit2"), ("pads", "pad.race")]);
    let to = picked(&[("perf_brakes", "brakes.l2"), ("discs", "disc.kit2")]);
    assert!(
        vehicle.bundle_changes(&from, &to).is_empty(),
        "the pad was taken off by hand and the level did not change"
    );
}

#[test]
fn a_bundle_naming_a_component_no_slot_accepts_is_a_mistake_of_the_definition() {
    let text = VEHICLE.replace("bundle = [\"disc.kit1\"]", "bundle = [\"disc.nowhere\"]");
    let errors = vehicle(&text).check(&catalog(CATALOG)).unwrap_err();
    assert!(errors.iter().any(|error| error.contains("disc.nowhere")), "{errors:?}");
}

#[test]
fn a_bundle_is_not_followed_further() {
    let text = VEHICLE.replace(
        r#"accepts = ["disc.stock", "disc.kit1", "disc.kit2"]"#,
        r#"accepts = ["disc.stock", { component = "disc.kit1", bundle = ["pad.race"] }, "disc.kit2"]"#,
    );
    let vehicle = vehicle(&text);
    let result = installed(&vehicle, &Selection::default(), &picked(&[("perf_brakes", "brakes.l1")]));
    assert_eq!(
        result,
        picked(&[("perf_brakes", "brakes.l1"), ("discs", "disc.kit1")]),
        "the kit's own bundle is not read"
    );
}

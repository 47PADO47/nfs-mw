//! A level that comes with its parts, on authored data: `Brakes level 2` fits the brake discs of kit 2.

use std::collections::BTreeMap;
use std::sync::Arc;

use blackbox_components::{Catalog as Components, VehicleDef};
use nfsmw_data::car::customization::{Catalog, Category, Choice as Part, Selection};
use nfsmw_data::car::mods::components::{self, select};
use nfsmw_data::car::physics::{PerformanceCatalog, PerformanceCategory, PerformancePackage};

use super::*;

const FOLDER: &str = "BUNDLEBIKE";

const CATALOG: &str = r#"
[[component]]
id = "t:brakes.stock"
[[component]]
id = "t:brakes.1"
name = "Brake kit level 1"
tier = 1
[[component]]
id = "t:brakes.2"
name = "Brake kit level 2"
tier = 2
[[component]]
id = "t:disc.stock"
name = "Stock discs"
[[component]]
id = "t:disc.kit1"
name = "Discs of kit 1"
[[component]]
id = "t:disc.kit2"
name = "Discs of kit 2"
"#;

const VEHICLE: &str = r#"
id = "bundlebike"
[[slot]]
id = "brake_discs"
group = "looks"
accepts = ["t:disc.stock", "t:disc.kit1", "t:disc.kit2"]
default = "t:disc.stock"
[[slot]]
id = "perf_brakes"
group = "performance"
accepts = ["t:brakes.stock", { component = "t:brakes.1", bundle = ["t:disc.kit1"] }, { component = "t:brakes.2", bundle = ["t:disc.kit2"] }]
default = "t:brakes.stock"
"#;

fn publish() {
    let catalog: Components = toml::from_str(CATALOG).unwrap();
    let vehicle: VehicleDef = toml::from_str(VEHICLE).unwrap();
    vehicle.check(&catalog).unwrap();
    components::publish(FOLDER, components::Components { vehicle, catalog: Arc::new(catalog), parts: BTreeMap::new() });
    nfsmw_data::car::mods::bike::publish(FOLDER, blackbox_vehicle::bike::BikeSpec::example());
}

fn hash(component: &str) -> u32 {
    select::choice_hash(FOLDER, component)
}

fn garage_catalog() -> Catalog {
    let mut catalog = Catalog::default();
    // The brake discs are kept in the Body kit category, by the hash of the car and the component.
    catalog.choices[Category::Body as usize] = ["t:disc.kit1", "t:disc.kit2"]
        .map(|id| Part { hash: hash(id), label: "from the catalogue".into(), language_hash: None, carbon: false })
        .to_vec();
    catalog
}

fn levels() -> PerformanceCatalog {
    let mut levels = PerformanceCatalog::default();
    levels.categories[PerformanceCategory::Brakes as usize].max_level = 2;
    levels
}

/// What the shop holds: the selection and the package, and the options read from them.
struct Shop {
    catalog: Catalog,
    levels: PerformanceCatalog,
    parts: Selection,
    package: PerformancePackage,
}

impl Shop {
    fn new() -> Self {
        publish();
        Self {
            catalog: garage_catalog(),
            levels: levels(),
            parts: Selection::default(),
            package: PerformancePackage::default(),
        }
    }

    fn options(&self) -> VehicleOptions {
        VehicleOptions::build(&Inputs {
            folder: FOLDER,
            name: "Bundle bike",
            catalog: Some(&self.catalog),
            parts: &self.parts,
            color: None,
            paintable: false,
            original_color: None,
            performance: Some((&self.levels, self.package)),
        })
    }

    fn apply(&mut self, change: Change) -> Result<String, String> {
        let options = self.options();
        let mut color = None;
        let mut edit = Edit { parts: &mut self.parts, color: &mut color, performance: &mut self.package };
        options.apply(Some(&self.catalog), &mut edit, &change)
    }

    fn set(&mut self, slot: &str, choice: &str) {
        self.apply(Change::Set { slot: slot.into(), choice: choice.into() }).unwrap();
    }

    fn discs(&self) -> Option<u32> {
        self.parts.parts[Category::Body as usize]
    }

    fn level(&self) -> u8 {
        self.package.levels[PerformanceCategory::Brakes as usize]
    }
}

#[test]
fn the_brakes_level_fits_the_discs_of_its_kit() {
    let mut shop = Shop::new();
    shop.set("perf_brakes", "2");
    assert_eq!((shop.level(), shop.discs()), (2, Some(hash("t:disc.kit2"))));
    let discs = shop.options();
    assert_eq!(discs.slot("brake_discs").unwrap().installed().label, "Discs of kit 2", "the shop shows them fitted");
}

#[test]
fn lowering_the_level_takes_the_discs_off_or_fits_the_lower_kit() {
    let mut shop = Shop::new();
    shop.set("perf_brakes", "2");
    shop.set("perf_brakes", "1");
    assert_eq!((shop.level(), shop.discs()), (1, Some(hash("t:disc.kit1"))));
    shop.set("perf_brakes", "stock");
    assert_eq!((shop.level(), shop.discs()), (0, None), "level 0: the stock discs");
}

#[test]
fn the_discs_alone_do_not_change_the_level_and_another_discs_survive_a_level_that_does_not_bring_them() {
    let mut shop = Shop::new();
    shop.set("brake_discs", "t:disc.kit2");
    assert_eq!((shop.level(), shop.discs()), (0, Some(hash("t:disc.kit2"))));
    shop.set("perf_brakes", "1");
    assert_eq!(shop.discs(), Some(hash("t:disc.kit1")), "the level's own discs replace them");
}

#[test]
fn the_performance_shortcuts_bring_the_parts_too() {
    let mut shop = Shop::new();
    shop.apply(Change::Shortcut(Shortcut::Maximum)).unwrap();
    assert_eq!((shop.level(), shop.discs()), (2, Some(hash("t:disc.kit2"))));
    shop.apply(Change::Shortcut(Shortcut::Stock)).unwrap();
    assert_eq!((shop.level(), shop.discs()), (0, None));
}

#[test]
fn a_reset_puts_the_discs_back_with_the_level() {
    let mut shop = Shop::new();
    shop.set("perf_brakes", "2");
    shop.apply(Change::Reset).unwrap();
    assert_eq!((shop.level(), shop.discs()), (0, None));
}

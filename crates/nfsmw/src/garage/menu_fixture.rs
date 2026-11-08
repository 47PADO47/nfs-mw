//! A bike with components for the shop's tests: the slots, rules and prices a mod would write, on authored data (no game files).
//!
//! `SHOPBIKE` has an exhaust (kept in the Spoiler category, a race one priced at 900 and a loud one that needs the engine's first
//! stage), an engine ladder (two stages, the second priced), side mirrors (a slot only the components know, two choices and the
//! option of none) and a rims slot with one aftermarket rim. `shop_model` is the shop's model with it on show.

use std::collections::BTreeMap;
use std::sync::Arc;

use blackbox_components::{Catalog as Components, VehicleDef};
use nfsmw_data::car::customization::{Catalog, Category, Choice};
use nfsmw_data::car::mods::components::{self, select};
use nfsmw_data::car::physics::{PerformanceCatalog, PerformanceCategory};

use super::{Car, Model, Selection};

pub(crate) const FOLDER: &str = "SHOPBIKE";

const CATALOG: &str = r#"
[[component]]
id = "t:exhaust.stock"
name = "Stock exhaust"
[[component]]
id = "t:exhaust.race"
name = "Race exhaust"
price = 900
[[component]]
id = "t:exhaust.loud"
name = "Loud exhaust"
when = [{ kind = "tier", slot = "perf_engine", at_least = 1 }]
[[component]]
id = "t:rims.stock"
name = "Stock rims"
[[component]]
id = "t:rims.forged"
name = "Forged rims"
price = 1200
[[component]]
id = "t:engine.stock"
[[component]]
id = "t:engine.1"
name = "Engine kit"
tier = 1
[[component]]
id = "t:engine.2"
name = "Engine kit 2"
tier = 2
price = 2500
[[component]]
id = "t:mirrors.on"
name = "Mirrors"
[[component]]
id = "t:mirrors.chrome"
name = "Chrome mirrors"
price = 250
"#;

const VEHICLE: &str = r#"
id = "shopbike"
[[slot]]
id = "exhaust"
group = "looks"
accepts = ["t:exhaust.stock", "t:exhaust.race", "t:exhaust.loud"]
default = "t:exhaust.stock"
[[slot]]
id = "rims"
group = "looks"
accepts = ["t:rims.stock", "t:rims.forged"]
default = "t:rims.stock"
[[slot]]
id = "perf_engine"
group = "performance"
accepts = ["t:engine.stock", "t:engine.1", "t:engine.2"]
default = "t:engine.stock"
[[slot]]
id = "road_mirrors"
group = "road"
accepts = ["t:mirrors.on", "t:mirrors.chrome"]
default = "t:mirrors.on"
optional = true
"#;

/// Publishes `SHOPBIKE` as a bike with the components above (once is enough: a second publication keeps the first).
pub(crate) fn publish() {
    let catalog: Components = toml::from_str(CATALOG).unwrap();
    let vehicle: VehicleDef = toml::from_str(VEHICLE).unwrap();
    vehicle.check(&catalog).unwrap();
    components::publish(FOLDER, components::Components { vehicle, catalog: Arc::new(catalog), parts: BTreeMap::new() });
    nfsmw_data::car::mods::bike::publish(FOLDER, blackbox_vehicle::bike::BikeSpec::example());
}

fn choice(component: &str) -> Choice {
    Choice {
        hash: select::choice_hash(FOLDER, component),
        label: "from the catalogue".into(),
        language_hash: None,
        carbon: false,
    }
}

/// The garage's catalogue of the bike: the components of its slots that a model category holds, as the host lists them.
pub(crate) fn catalog() -> Catalog {
    let mut catalog = Catalog::default();
    catalog.choices[Category::Spoiler as usize] = vec![choice("t:exhaust.race"), choice("t:exhaust.loud")];
    catalog.choices[Category::Wheels as usize] = vec![choice("t:rims.forged")];
    for hash in catalog.choices.iter().flatten().map(|choice| choice.hash).collect::<Vec<_>>() {
        catalog.part_groups.insert(hash, 0);
    }
    catalog
}

/// The shop's model with the bike on show and in the cart as it is (stock).
pub(crate) fn shop_model() -> Model {
    publish();
    let mut performance = PerformanceCatalog::default();
    performance.categories[PerformanceCategory::Engine as usize].max_level = 2;
    let car = Car {
        folder: FOLDER.into(),
        name: "Shop bike".into(),
        performance,
        parts: Some(catalog().into()),
        ..Default::default()
    };
    let draft = Selection { car: FOLDER.into(), ..Default::default() };
    Model { cars: vec![car], applied: draft.clone(), draft, shop: true, ..Default::default() }
}

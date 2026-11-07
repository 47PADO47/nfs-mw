//! The components of a mod's cars (ADR 0014, docs/modding/ride6-bike-upgrades.md): the mod's `components/*.toml` catalog files are
//! read once per mod, and each car that lists slots is checked against them and published for the game.

use std::collections::{BTreeMap, HashMap};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, OnceLock};

use blackbox_components::{Catalog, VehicleDef};
use blackbox_modding::content::{
    COMPONENTS_FOLDER, Components as Manifest, PartBinding, PartRole, merge_catalog, parse_catalog,
};
use nfsmw_data::car::mods::components::{self, Binding, Role, keys};

use super::cars::manifest_files;

/// The catalog of each mod folder, read at the first car that needs it.
fn catalog_of(owner: &Path) -> Arc<Catalog> {
    static CATALOGS: OnceLock<Mutex<HashMap<PathBuf, Arc<Catalog>>>> = OnceLock::new();
    let mut catalogs = CATALOGS.get_or_init(Default::default).lock().unwrap_or_else(|error| error.into_inner());
    if let Some(known) = catalogs.get(owner) {
        return Arc::clone(known);
    }
    let catalog = Arc::new(read_catalog(owner));
    catalogs.insert(owner.to_owned(), Arc::clone(&catalog));
    catalog
}

/// Every `[[component]]` of the mod's `components/` files, in path order. A file that does not parse, or repeats an id of an
/// earlier file, is logged and left out.
fn read_catalog(owner: &Path) -> Catalog {
    let mut catalog = Catalog::default();
    for path in manifest_files(&owner.join(COMPONENTS_FOLDER)) {
        let text = match std::fs::read_to_string(&path) {
            Ok(text) => text,
            Err(error) => {
                log::warn!("{}: not read: {error}", path.display());
                continue;
            }
        };
        let read = parse_catalog(&text).and_then(|more| merge_catalog(&mut catalog, more));
        if let Err(error) = read {
            log::warn!("{}: not used: {error}", path.display());
        }
    }
    for line in keys::unbound(&catalog.used()) {
        log::warn!("{}: {line}", owner.display());
    }
    catalog
}

fn binding(part: &PartBinding) -> Binding {
    let role = match part.role {
        PartRole::Body => Role::Body,
        PartRole::Steered => Role::Steered,
        PartRole::SteeredSlid => Role::SteeredSlid,
        PartRole::WheelFront => Role::WheelFront,
        PartRole::WheelRear => Role::WheelRear,
    };
    Binding { solid: part.solid.clone(), pack: part.pack.clone(), role, matrix: part.matrix, base: part.base }
}

/// Publish the components of the car in `folder`, whose mod is in `owner`. A car whose slots do not fit its mod's catalog is
/// logged and published with its own stock parts only (no choices): its stock parts are solids of their own, and must still draw.
pub(super) fn publish(owner: &Path, folder: &str, own: &Manifest) {
    let catalog = catalog_of(owner);
    let base = own.parts.iter().filter(|(_, part)| part.base).map(|(id, _)| id.clone());
    let vehicle = VehicleDef { id: folder.to_ascii_lowercase(), parts: base.collect(), slots: own.slots.clone() };
    let parts: BTreeMap<String, Binding> = own.parts.iter().map(|(id, part)| (id.clone(), binding(part))).collect();
    let Err(errors) = vehicle.check(&catalog) else {
        components::publish(folder, components::Components { vehicle, catalog, parts });
        return;
    };
    for error in errors.iter().take(5) {
        log::warn!("mod car {folder}: {error}");
    }
    log::warn!(
        "mod car {folder}: its components are not offered ({} mistakes; is {} of {} missing?); its stock parts are drawn",
        errors.len(),
        COMPONENTS_FOLDER,
        owner.display()
    );
    components::publish(folder, components::Components::stock_only(folder, parts));
}

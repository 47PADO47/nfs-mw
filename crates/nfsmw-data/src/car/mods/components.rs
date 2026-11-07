//! The components of a mod car (ADR 0014, docs/modding/ride6-bike-upgrades.md): a vehicle that lists slots, the catalog of
//! components its slots accept and the parts those components show, bound to the garage's machinery.
//!
//! The model is `blackbox-components`. This module is the first binder of the game's side, for a bike: [`select`] turns what the
//! garage keeps (the model choices of [`crate::car::customization::Selection`], the performance package of the save) into a
//! component selection and resolves it, [`keys`] maps the resolved numbers onto the bike's `[bike]` data, [`parts`] draws the
//! resolved parts on the assembled model, and [`catalog`] lists the choices in the garage's categories.
//!
//! A mod publishes its cars' components with [`publish`] (the host reads `[[slot]]`, `[parts]` and `components/*.toml`); a car
//! that never publishes any is untouched by everything here.

use std::collections::BTreeMap;
use std::sync::{Arc, PoisonError, RwLock};

use blackbox_components::{Catalog, VehicleDef};
use blackbox_vehicle::bike::BikeSpec;

use crate::car::customization::Selection;
use crate::car::physics::PerformancePackage;

pub mod catalog;
pub mod keys;
pub mod parts;
pub mod select;
#[cfg(test)]
mod tests;

/// The frame a part's solid is modelled in, which says how the game poses it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Role {
    /// The body's: fixed on the car.
    Body,
    /// A bike's bars (the front hub on the origin, turning with the handlebar).
    Steered,
    /// A bike's fork (the front hub on the origin, turning and travelling with the front wheel).
    SteeredSlid,
    /// The front wheel's (the hub on the origin, spinning with the wheel).
    WheelFront,
    /// The rear wheel's.
    WheelRear,
}

/// Where the solid of a part is and how it sits on the car (`[parts."<id>"]`).
#[derive(Debug, Clone, PartialEq)]
pub struct Binding {
    /// The solid's name without its level-of-detail letter.
    pub solid: String,
    /// The folder under `files/` with the part's own `GEOMETRY.BIN` and `TEXTURES.BIN`; `None`: the car's own files.
    pub pack: Option<String>,
    pub role: Role,
    /// The mount: three rows of a 3 by 4 affine matrix taking the solid's points to the frame of `role`.
    pub matrix: Option<[f32; 12]>,
}

/// One car's components.
#[derive(Debug)]
pub struct Components {
    /// The slots of the car, in screen order.
    pub vehicle: VehicleDef,
    /// Every component the car's mod defines, shared by the mod's cars.
    pub catalog: Arc<Catalog>,
    /// The parts the components show, by part id.
    pub parts: BTreeMap<String, Binding>,
}

impl Components {
    /// The car with its own solids only: no slots and no catalog, so nothing is offered, but every part bound to the car's own
    /// files (the stock exhaust, rims, discs and pad a bike's importer split out of its body, bars and wheels) is still drawn.
    /// What a car gets whose slots do not fit its mod's catalog (a catalog file missing from the install): without it the split
    /// stock parts would vanish and the bike would roll on bare tyres.
    pub fn stock_only(id: &str, parts: BTreeMap<String, Binding>) -> Self {
        let own = parts.iter().filter(|(_, binding)| binding.pack.is_none()).map(|(part, _)| part.clone());
        let vehicle = VehicleDef { id: id.to_ascii_lowercase(), parts: own.collect(), slots: Vec::new() };
        Self { vehicle, catalog: Arc::new(Catalog::default()), parts }
    }

    /// The highest tier of the components `slot` accepts: the levels of a performance ladder. 0 when the car has no such slot.
    pub fn max_tier(&self, slot: &str) -> u8 {
        let Some(slot) = self.vehicle.slot(slot) else { return 0 };
        let tiers = slot.accepts.iter().filter_map(|offer| self.catalog.get(&offer.component)).map(|c| c.tier);
        u8::try_from(tiers.max().unwrap_or(0)).unwrap_or(u8::MAX)
    }
}

/// The bike `spec` of the mod car in `folder` with the upgrade levels of `package` installed: the number modifiers of the
/// components the levels pick ([`keys`]). A bike without components, or with the package at stock, is `spec` itself.
pub fn upgraded(folder: &str, spec: Arc<BikeSpec>, package: PerformancePackage) -> Arc<BikeSpec> {
    let Some(components) = of(folder) else { return spec };
    if package.levels.iter().all(|level| *level == 0) {
        return spec;
    }
    let Some(resolved) = select::resolve(folder, components, &Selection::default(), package) else { return spec };
    Arc::new(keys::apply(&spec, &resolved.params))
}

/// The cars with components, by folder. Each publication leaks the new list, as the other per-car tables do.
static PUBLISHED: RwLock<&'static [(String, &'static Components)]> = RwLock::new(&[]);

/// Register the components of the mod car in `folder`. A folder already registered keeps its first.
pub fn publish(folder: &str, components: Components) {
    let mut published = PUBLISHED.write().unwrap_or_else(PoisonError::into_inner);
    if published.iter().any(|(known, _)| known.eq_ignore_ascii_case(folder)) {
        return;
    }
    let mut all: Vec<(String, &'static Components)> = published.to_vec();
    let leaked: &'static Components = Box::leak(Box::new(components));
    all.push((folder.to_owned(), leaked));
    *published = Box::leak(all.into_boxed_slice());
}

/// Whether `hash` stands for a component a car lists in a slot the garage has a category for ([`select::choice_hash`]): like
/// every choice of a mod car it is priced at the first tier.
pub fn is_choice(hash: u32) -> bool {
    let published = *PUBLISHED.read().unwrap_or_else(PoisonError::into_inner);
    published.iter().any(|(folder, components)| {
        let slots = components.vehicle.slots.iter().filter(|slot| select::category_of(&slot.id).is_some());
        slots.flat_map(|slot| &slot.accepts).any(|offer| select::choice_hash(folder, &offer.component) == hash)
    })
}

/// Whether `hash` stands for a component the car in `folder` lists in a slot the garage has a category for: a choice of the
/// car's components, which the car's own model lists (`custom.rims` and the like) do not hold.
pub fn is_choice_of(folder: &str, hash: u32) -> bool {
    let Some(components) = of(folder) else { return false };
    let slots = components.vehicle.slots.iter().filter(|slot| select::category_of(&slot.id).is_some());
    slots.flat_map(|slot| &slot.accepts).any(|offer| select::choice_hash(folder, &offer.component) == hash)
}

/// The components of the mod car in `folder`. `None` for a stock car and for a mod car that gives none.
pub fn of(folder: &str) -> Option<&'static Components> {
    let published = *PUBLISHED.read().unwrap_or_else(PoisonError::into_inner);
    published.iter().find(|(known, _)| known.eq_ignore_ascii_case(folder)).map(|(_, components)| *components)
}

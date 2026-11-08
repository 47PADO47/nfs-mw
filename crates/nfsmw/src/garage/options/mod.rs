//! What the vehicle on show can change, as one list: the view model of the developer tools' vehicle panel and of the `vehicle`
//! console command, and of the garage's Parts tab (`docs/devtools-vehicle.md`).
//!
//! A vehicle lists **slots**; each slot lists the choices it takes and says where its choice is kept. The list comes from the
//! vehicle's own data and nothing here names a part:
//!
//! * a car that has **components** (a RIDE 6 bike, ADR 0014) offers the slots of its component definition, with the
//!   components' names, prices and rules, in the group the definition gives them (`looks`, `performance`...);
//! * every vehicle also offers its **category model**: the nine model categories of the garage (body kit, spoiler, hood, roof
//!   scoop, rims, paint, window tint, rim paint, vinyl) with the choices its catalogue has, and the performance levels its
//!   upgrade catalogue has. A car of the 2005 game and a car of the 2012 mod has only this: its categories are its components.
//!   A category a component slot maps onto (`mods::components::select::category_of`) takes the slot's name, group and rules, so a
//!   bike's exhaust is listed as the exhaust and not as the "spoiler" it is kept as.
//!
//! [`VehicleOptions::build`] reads the data; [`VehicleOptions::apply`] edits the part of a selection that holds the choices
//! ([`Edit`]) and refuses what the vehicle does not accept. The scenes apply the edited selection with their own machinery
//! (the world's `apply_garage_selection`, the car viewer's reload), so a change is on the vehicle at once.

mod build;
mod change;
mod groups;
mod names;
mod report;
#[cfg(test)]
mod tests;

use nfsmw_data::car::customization::Category;
use nfsmw_data::car::physics::PerformanceCategory;

pub(crate) use build::Inputs;
pub(crate) use change::{Change, Edit, Shortcut};
pub(crate) use groups::Group;
pub(crate) use names::humanize;

/// Where a slot's choice is kept.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Store {
    /// A hash in one of the garage's model categories (kept in `my-cars.toml`).
    Category(Category),
    /// A level of one of the garage's performance categories (kept in the save).
    Level(PerformanceCategory),
    /// The session's own list of a component slot the garage has no category for (`Selection::slots`).
    Session,
}

/// What a choice stands for when it is applied.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Value {
    /// The stock part: the slot's default, or nothing.
    Stock,
    /// A part of a model category, by the hash the garage keeps.
    Hash(u32),
    /// A performance level.
    Level(u8),
    /// A component of a session slot, by id.
    Component(String),
    /// The slot emptied on purpose.
    Empty,
}

/// One thing a slot can hold.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Choice {
    /// What `vehicle set` also accepts: the component's id, a part's hash, `stock`.
    pub id: String,
    pub label: String,
    /// The component's price, when the data gives one.
    pub price: Option<u32>,
    /// The component's place in a ladder (`0` for the rest).
    pub tier: u32,
    /// Why it cannot be picked with the rest of the selection, when a rule of the vehicle says so.
    pub blocked: Option<String>,
    pub value: Value,
}

/// The Junkman package of a performance category.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Junkman {
    /// The vehicle's data has a package for the category and its equipment allows it.
    pub available: bool,
    pub on: bool,
}

/// A quick finish of the body paint: the catalogue's finish tabs (gloss, metallic...).
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Finish {
    pub label: String,
    pub hash: u32,
    pub active: bool,
}

/// A place on the vehicle that takes one choice.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Slot {
    /// What the console calls it: the component slot's id (`exhaust`), else the category's word (`rims`, `body`).
    pub id: String,
    pub label: String,
    /// The section it is listed under: the component slot's `group`, else `parts`, `appearance` or `performance`.
    pub group: String,
    /// Whether the slot comes from the vehicle's components or from the category model.
    pub components: bool,
    pub store: Store,
    /// The choice now installed: an index of `choices`.
    pub current: usize,
    /// The choices, the stock one first. The numbers `vehicle set` takes are the positions here.
    pub choices: Vec<Choice>,
    /// For a performance slot of a vehicle that has the upgrade catalogue: its Junkman package.
    pub junkman: Option<Junkman>,
}

impl Slot {
    /// The choice installed now.
    pub fn installed(&self) -> &Choice {
        &self.choices[self.current.min(self.choices.len() - 1)]
    }

    /// Whether the slot holds anything but its stock choice.
    pub fn changed(&self) -> bool {
        self.current != 0
    }

    /// The position of the choice that empties the slot, when it has one.
    pub fn empty_choice(&self) -> Option<usize> {
        self.choices.iter().position(|choice| choice.value == Value::Empty)
    }
}

/// The body colour of a vehicle that can be repainted.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Paintwork {
    /// The colour picked, `None` for the paint the vehicle has.
    pub custom: Option<[u8; 3]>,
    /// The paint it comes with.
    pub original: Option<[u8; 3]>,
}

/// Everything the tools can change about one vehicle.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct VehicleOptions {
    pub folder: String,
    pub name: String,
    /// A bike (a mod car with a bike's data).
    pub bike: bool,
    /// The slots, in the order of the vehicle's own definition for a component slot and of the garage's categories for the rest.
    pub slots: Vec<Slot>,
    /// `None`: the vehicle has a fixed livery.
    pub paintwork: Option<Paintwork>,
    /// What the data of this vehicle does not have, and the tools would need (the panel and `vehicle groups` say it).
    pub missing: Vec<String>,
    /// The quick finishes of the body paint the catalogue offers.
    pub finishes: Vec<Finish>,
    /// The vehicle's upgrade catalogue, for the shortcuts and the Junkman rules; `None` where the scene has no physics.
    pub levels: Option<nfsmw_data::car::physics::PerformanceCatalog>,
}

impl VehicleOptions {
    /// The slot a console word names: its id, or its label in any case.
    pub fn slot(&self, word: &str) -> Option<&Slot> {
        let word = word.to_ascii_lowercase();
        self.slots
            .iter()
            .find(|slot| slot.id.to_ascii_lowercase() == word)
            .or_else(|| self.slots.iter().find(|slot| slot.label.to_ascii_lowercase() == word))
    }

    /// The slots listed under `group`.
    pub fn in_group<'a>(&'a self, group: &'a str) -> impl Iterator<Item = &'a Slot> {
        self.slots.iter().filter(move |slot| slot.group == group)
    }
}

impl VehicleOptions {
    /// The options of the vehicle the model has applied (the one in the world). `None` while its part catalogue is not read.
    pub(crate) fn of_applied(model: &super::Model) -> Option<Self> {
        let applied = &model.applied;
        let car = model.cars.iter().find(|car| car.folder == applied.car)?;
        let catalog = car.parts.as_deref()?;
        Some(Self::build(&Inputs {
            folder: &car.folder,
            name: &car.name,
            catalog: Some(catalog),
            parts: &applied.parts,
            color: applied.color,
            paintable: car.paintable,
            original_color: car.original_color,
            performance: Some((&car.performance, applied.performance)),
        }))
    }
}

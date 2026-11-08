//! The data model: a [`Catalog`] of components shared by many vehicles, and a [`VehicleDef`] that says which slots a
//! vehicle has and which components each slot accepts.

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

use crate::effect::{Effect, Input, PartId};

pub type SlotId = String;
pub type ComponentId = String;

/// A rule on the rest of the selection. Slots that are not active or hold nothing count as empty.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum Condition {
    /// The slot holds one of these components.
    Is { slot: SlotId, any_of: Vec<ComponentId> },
    /// The slot holds none of these (an empty slot passes).
    IsNot { slot: SlotId, any_of: Vec<ComponentId> },
    /// The slot holds some component.
    Filled { slot: SlotId },
    /// The slot holds nothing.
    Empty { slot: SlotId },
    /// The slot's component has at least this tier (an empty slot has tier 0).
    Tier { slot: SlotId, at_least: u32 },
}

impl Condition {
    pub(crate) fn slot(&self) -> &str {
        match self {
            Self::Is { slot, .. }
            | Self::IsNot { slot, .. }
            | Self::Filled { slot }
            | Self::Empty { slot }
            | Self::Tier { slot, .. } => slot,
        }
    }
}

/// What the player may type or pick when installing a component in a slot.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum InputDef {
    Rgb { key: String, default: [u8; 3] },
    Num { key: String, default: f64, min: f64, max: f64 },
    Text { key: String, default: String, max_len: usize },
}

impl InputDef {
    pub fn key(&self) -> &str {
        match self {
            Self::Rgb { key, .. } | Self::Num { key, .. } | Self::Text { key, .. } => key,
        }
    }

    pub(crate) fn default_input(&self) -> Input {
        match self {
            Self::Rgb { default, .. } => Input::Rgb(*default),
            Self::Num { default, .. } => Input::Num(*default),
            Self::Text { default, .. } => Input::Text(default.clone()),
        }
    }

    /// Why `input` does not fit this definition.
    pub(crate) fn refuse(&self, input: &Input) -> Option<String> {
        match (self, input) {
            (Self::Rgb { .. }, Input::Rgb(_)) => None,
            (Self::Num { min, max, .. }, Input::Num(n)) => match n.is_finite() && (*min..=*max).contains(n) {
                true => None,
                false => Some(format!("{n} is outside {min} to {max}")),
            },
            (Self::Text { max_len, .. }, Input::Text(text)) => match text.chars().count() <= *max_len {
                true => None,
                false => Some(format!("longer than {max_len} characters")),
            },
            _ => Some("wrong kind of value".into()),
        }
    }
}

/// A component: one thing that can be installed, shared by every vehicle that accepts it.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Component {
    pub id: ComponentId,
    /// What the screens call it (a translation key or a plain name). Empty: the id.
    #[serde(default)]
    pub name: String,
    /// Its place in a ladder of upgrades (`Condition::Tier`). 0 for stock and for components that are not tiers.
    #[serde(default)]
    pub tier: u32,
    #[serde(default)]
    pub effects: Vec<Effect>,
    /// Rules that must hold wherever it is installed.
    #[serde(default)]
    pub when: Vec<Condition>,
    /// The price in the game's currency; the game decides what a price means. 0: free.
    #[serde(default)]
    pub price: u32,
    /// The name of an unlock the game checks ([`crate::Gate`]); `None`: always available.
    #[serde(default)]
    pub unlock: Option<String>,
}

impl Component {
    pub fn display_name(&self) -> &str {
        match self.name.is_empty() {
            true => &self.id,
            false => &self.name,
        }
    }
}

/// Every component a mod or game defines, by id.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(try_from = "CatalogRepr", into = "CatalogRepr")]
pub struct Catalog {
    components: BTreeMap<ComponentId, Component>,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct CatalogRepr {
    #[serde(default, rename = "component")]
    components: Vec<Component>,
}

impl TryFrom<CatalogRepr> for Catalog {
    type Error = String;

    fn try_from(repr: CatalogRepr) -> Result<Self, String> {
        let mut catalog = Catalog::default();
        for component in repr.components {
            catalog.insert(component)?;
        }
        Ok(catalog)
    }
}

impl From<Catalog> for CatalogRepr {
    fn from(catalog: Catalog) -> Self {
        Self { components: catalog.components.into_values().collect() }
    }
}

impl Catalog {
    /// Add a component. An id that is already there is an error: a mod cannot replace another mod's component.
    pub fn insert(&mut self, component: Component) -> Result<(), String> {
        if self.components.contains_key(&component.id) {
            return Err(format!("component {} is defined twice", component.id));
        }
        self.components.insert(component.id.clone(), component);
        Ok(())
    }

    pub fn get(&self, id: &str) -> Option<&Component> {
        self.components.get(id)
    }

    pub fn iter(&self) -> impl Iterator<Item = &Component> {
        self.components.values()
    }

    /// Every name the components use, for the game to check against what it can bind before the first resolve.
    pub fn used(&self) -> Used {
        let mut used = Used::default();
        for effect in self.components.values().flat_map(|component| &component.effects) {
            match effect {
                Effect::Part { show, hide } => used.parts.extend(show.iter().chain(hide).cloned()),
                Effect::Surface { target, .. } => {
                    used.surfaces.insert(target.clone());
                }
                Effect::Param { key, .. } => {
                    used.params.insert(key.clone());
                }
                Effect::Behaviour { domain, select } => {
                    used.behaviours.entry(domain.clone()).or_default().insert(select.clone());
                }
            }
        }
        used
    }
}

/// The part ids, surface targets, parameter keys and behaviours (by domain) a catalog uses.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Used {
    pub parts: BTreeSet<PartId>,
    pub surfaces: BTreeSet<String>,
    pub params: BTreeSet<String>,
    pub behaviours: BTreeMap<String, BTreeSet<String>>,
}

/// A component a slot accepts, with what only this vehicle changes about it.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(from = "OfferRepr")]
pub struct Offer {
    pub component: ComponentId,
    /// This vehicle's price, instead of the component's.
    pub price: Option<u32>,
    /// Rules that hold for this vehicle only.
    pub when: Vec<Condition>,
}

#[derive(Deserialize)]
#[serde(untagged)]
enum OfferRepr {
    Id(ComponentId),
    Full(OfferFull),
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct OfferFull {
    component: ComponentId,
    #[serde(default)]
    price: Option<u32>,
    #[serde(default)]
    when: Vec<Condition>,
}

impl From<OfferRepr> for Offer {
    fn from(repr: OfferRepr) -> Self {
        match repr {
            OfferRepr::Id(component) => Self { component, price: None, when: Vec::new() },
            OfferRepr::Full(full) => Self { component: full.component, price: full.price, when: full.when },
        }
    }
}

/// A place on a vehicle where one component can be installed.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Slot {
    pub id: SlotId,
    /// The screen section it is listed under (`engine`, `looks`...). Empty: none.
    #[serde(default)]
    pub group: String,
    /// The components it accepts, in the order the screens list them.
    pub accepts: Vec<Offer>,
    /// The stock component. `None`: the slot is empty on a stock vehicle.
    #[serde(default)]
    pub default: Option<ComponentId>,
    /// The slot can be emptied. A slot with no default is always optional.
    #[serde(default)]
    pub optional: bool,
    /// The slot exists only while these hold. They name earlier slots of the vehicle only.
    #[serde(default)]
    pub when: Vec<Condition>,
    /// What the player gives when picking a component here.
    #[serde(default)]
    pub inputs: Vec<InputDef>,
}

impl Slot {
    pub fn offer(&self, component: &str) -> Option<&Offer> {
        self.accepts.iter().find(|offer| offer.component == component)
    }

    pub(crate) fn can_be_empty(&self) -> bool {
        self.optional || self.default.is_none()
    }
}

/// A base vehicle: the parts it is built with and the slots it offers.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct VehicleDef {
    pub id: String,
    /// The model parts the vehicle is drawn with before any component.
    #[serde(default)]
    pub parts: BTreeSet<PartId>,
    #[serde(default, rename = "slot")]
    pub slots: Vec<Slot>,
}

impl VehicleDef {
    pub fn slot(&self, id: &str) -> Option<&Slot> {
        self.slots.iter().find(|slot| slot.id == id)
    }
}

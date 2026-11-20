//! Checking a selection against a vehicle, repairing it, and checking what a change would need unlocked.

use std::fmt;

use crate::model::{Catalog, Condition, SlotId, VehicleDef};
use crate::selection::Selection;
use crate::state::State;

/// Why a selection is not valid for a vehicle.
#[derive(Debug, Clone, PartialEq)]
pub enum Problem {
    /// The vehicle has no such slot.
    UnknownSlot(SlotId),
    /// The slot is not active (a rule of its `when` fails) but the selection sets it.
    InactiveSlot(SlotId),
    /// The slot does not accept this component, or the catalog does not have it.
    NotAccepted { slot: SlotId, component: String },
    /// The slot cannot be empty.
    EmptyNotAllowed(SlotId),
    /// A rule of the component or of the vehicle's offer does not hold.
    ConditionFailed { slot: SlotId, component: String, condition: Condition },
    /// An input the slot does not have, or one out of range or of the wrong kind.
    BadInput { slot: SlotId, key: String, reason: String },
    /// The component needs an unlock the player does not have (only [`VehicleDef::locked`] reports it).
    Locked { slot: SlotId, component: String, unlock: String },
}

impl Problem {
    pub fn slot(&self) -> &str {
        match self {
            Self::UnknownSlot(slot)
            | Self::InactiveSlot(slot)
            | Self::EmptyNotAllowed(slot)
            | Self::NotAccepted { slot, .. }
            | Self::ConditionFailed { slot, .. }
            | Self::BadInput { slot, .. }
            | Self::Locked { slot, .. } => slot,
        }
    }
}

impl fmt::Display for Problem {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnknownSlot(slot) => write!(f, "the vehicle has no slot {slot}"),
            Self::InactiveSlot(slot) => write!(f, "slot {slot} is not available with the other choices"),
            Self::NotAccepted { slot, component } => write!(f, "slot {slot} does not accept {component}"),
            Self::EmptyNotAllowed(slot) => write!(f, "slot {slot} cannot be empty"),
            Self::ConditionFailed { slot, component, .. } => {
                write!(f, "{component} in slot {slot} needs a different combination of parts")
            }
            Self::BadInput { slot, key, reason } => write!(f, "slot {slot}, input {key}: {reason}"),
            Self::Locked { slot, component, unlock } => write!(f, "{component} in slot {slot} needs {unlock}"),
        }
    }
}

/// Who says what the player has unlocked: the career, the shop, a test. Resolving never asks it (a car in a save keeps
/// the parts it was bought with); only a change does.
pub trait Gate {
    fn unlocked(&self, unlock: &str) -> bool;
}

impl Gate for () {
    fn unlocked(&self, _: &str) -> bool {
        true
    }
}

impl<F: Fn(&str) -> bool> Gate for F {
    fn unlocked(&self, unlock: &str) -> bool {
        self(unlock)
    }
}

impl VehicleDef {
    /// Every way `selection` is not valid for this vehicle; empty when it is. Needs no unlocks.
    pub fn validate(&self, catalog: &Catalog, selection: &Selection) -> Vec<Problem> {
        let mut problems: Vec<Problem> = selection
            .iter()
            .filter(|(slot, _)| self.slot(slot).is_none())
            .map(|(slot, _)| Problem::UnknownSlot(slot.clone()))
            .collect();
        let state = State::new(self, catalog, selection);
        for item in &state.items {
            let slot = &item.slot.id;
            let explicit = selection.get(slot);
            if !item.active {
                if explicit.is_some() {
                    problems.push(Problem::InactiveSlot(slot.clone()));
                }
                continue;
            }
            for key in item.pick.inputs.keys() {
                let reason = match item.slot.inputs.iter().find(|def| def.key() == key) {
                    None => Some("the slot has no such input".to_string()),
                    Some(def) => def.refuse(&item.pick.inputs[key]),
                };
                if let Some(reason) = reason {
                    problems.push(Problem::BadInput { slot: slot.clone(), key: key.clone(), reason });
                }
            }
            let Some(component_id) = &item.pick.component else {
                if !item.slot.can_be_empty() {
                    problems.push(Problem::EmptyNotAllowed(slot.clone()));
                }
                continue;
            };
            let (Some(offer), Some(component)) = (item.slot.offer(component_id), catalog.get(component_id)) else {
                problems.push(Problem::NotAccepted { slot: slot.clone(), component: component_id.clone() });
                continue;
            };
            for condition in offer.when.iter().chain(&component.when).filter(|c| !state.holds(c)) {
                problems.push(Problem::ConditionFailed {
                    slot: slot.clone(),
                    component: component_id.clone(),
                    condition: condition.clone(),
                });
            }
        }
        problems
    }

    /// The nearest valid selection: every pick that causes a problem goes back to its default (a default has no rules,
    /// [`VehicleDef::check`]), until none does, and the result is canonical. Used after the player changes a slot, so
    /// that what depended on the old choice resets.
    pub fn normalize(&self, catalog: &Catalog, selection: &Selection) -> Selection {
        let mut selection = selection.clone();
        for _ in 0..=self.slots.len() {
            let problems = self.validate(catalog, &selection);
            if problems.is_empty() {
                break;
            }
            for problem in problems {
                selection.reset(problem.slot());
            }
        }
        selection.canonical(self)
    }

    /// The components `to` installs over `from` that need an unlock the gate refuses. A change checks this; a saved
    /// car does not.
    pub fn locked(&self, catalog: &Catalog, from: &Selection, to: &Selection, gate: &dyn Gate) -> Vec<Problem> {
        let before = State::new(self, catalog, from);
        let after = State::new(self, catalog, to);
        let mut out = Vec::new();
        for (old, new) in before.items.iter().zip(&after.items) {
            let Some(component_id) = new.pick.component.as_deref().filter(|_| new.active) else { continue };
            if old.active && old.pick.component.as_deref() == Some(component_id) {
                continue;
            }
            let unlock = catalog.get(component_id).and_then(|component| component.unlock.as_deref());
            if let Some(unlock) = unlock.filter(|unlock| !gate.unlocked(unlock)) {
                out.push(Problem::Locked {
                    slot: new.slot.id.clone(),
                    component: component_id.into(),
                    unlock: unlock.into(),
                });
            }
        }
        out
    }
}

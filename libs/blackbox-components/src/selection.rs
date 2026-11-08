//! A selection: what the player installed on a vehicle, in the form that is saved and sent.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::effect::Input;
use crate::model::{ComponentId, SlotId, VehicleDef};

/// What one slot holds: a component (or nothing) and the values the player gave for it.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Pick {
    /// `None`: the slot is emptied on purpose.
    #[serde(default)]
    pub component: Option<ComponentId>,
    /// Values for the slot's inputs; one left out takes the input's default.
    #[serde(default)]
    pub inputs: BTreeMap<String, Input>,
}

impl Pick {
    pub fn of(component: &str) -> Self {
        Self { component: Some(component.into()), inputs: BTreeMap::new() }
    }

    pub fn empty() -> Self {
        Self::default()
    }

    pub fn with(mut self, key: &str, input: Input) -> Self {
        self.inputs.insert(key.into(), input);
        self
    }
}

/// The picks that differ from stock, by slot. A slot that is not in it holds its default. The empty selection is the
/// stock vehicle.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Selection {
    pub(crate) picks: BTreeMap<SlotId, Pick>,
}

/// One edit of a selection, as sent over the network.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Change {
    /// Install this in the slot.
    Set { slot: SlotId, pick: Pick },
    /// Put the slot back to its default.
    Reset { slot: SlotId },
}

impl Selection {
    pub fn get(&self, slot: &str) -> Option<&Pick> {
        self.picks.get(slot)
    }

    pub fn iter(&self) -> impl Iterator<Item = (&SlotId, &Pick)> {
        self.picks.iter()
    }

    pub fn len(&self) -> usize {
        self.picks.len()
    }

    pub fn is_empty(&self) -> bool {
        self.picks.is_empty()
    }

    pub fn set(&mut self, slot: &str, pick: Pick) {
        self.picks.insert(slot.into(), pick);
    }

    pub fn reset(&mut self, slot: &str) {
        self.picks.remove(slot);
    }

    /// The same selection in its one canonical form: a pick that holds the slot's default component with default
    /// inputs is dropped, and so is an input equal to its default. Two selections of one vehicle are the same car
    /// exactly when their canonical forms are equal.
    pub fn canonical(&self, vehicle: &VehicleDef) -> Self {
        let mut out = Self::default();
        for (slot_id, pick) in &self.picks {
            let Some(slot) = vehicle.slot(slot_id) else {
                out.picks.insert(slot_id.clone(), pick.clone());
                continue;
            };
            let mut pick = pick.clone();
            pick.inputs
                .retain(|key, input| !slot.inputs.iter().any(|def| def.key() == key && def.default_input() == *input));
            if pick.component == slot.default && pick.inputs.is_empty() {
                continue;
            }
            out.picks.insert(slot_id.clone(), pick);
        }
        out
    }

    /// The edits that turn `self` into `to`. Both should be canonical.
    pub fn diff(&self, to: &Self) -> Vec<Change> {
        let mut changes = Vec::new();
        for (slot, pick) in &to.picks {
            if self.picks.get(slot) != Some(pick) {
                changes.push(Change::Set { slot: slot.clone(), pick: pick.clone() });
            }
        }
        for slot in self.picks.keys().filter(|slot| !to.picks.contains_key(*slot)) {
            changes.push(Change::Reset { slot: slot.clone() });
        }
        changes
    }

    pub fn apply(&mut self, changes: &[Change]) {
        for change in changes {
            match change {
                Change::Set { slot, pick } => self.set(slot, pick.clone()),
                Change::Reset { slot } => self.reset(slot),
            }
        }
    }
}

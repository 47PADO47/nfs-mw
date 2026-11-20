//! The picks in effect: every slot with its pick (the default where the selection has none) and whether it is active.

use crate::model::{Catalog, Condition, Slot, VehicleDef};
use crate::selection::{Pick, Selection};

pub(crate) struct Item<'a> {
    pub slot: &'a Slot,
    pub pick: Pick,
    /// The slot's `when` rules hold. An inactive slot installs nothing.
    pub active: bool,
}

pub(crate) struct State<'a> {
    pub items: Vec<Item<'a>>,
    catalog: &'a Catalog,
}

impl<'a> State<'a> {
    /// Slots are taken in order and a slot's rules name earlier slots only, so one pass settles every `active`.
    pub fn new(vehicle: &'a VehicleDef, catalog: &'a Catalog, selection: &Selection) -> Self {
        let mut state = Self { items: Vec::with_capacity(vehicle.slots.len()), catalog };
        for slot in &vehicle.slots {
            let pick = selection
                .get(&slot.id)
                .cloned()
                .unwrap_or_else(|| Pick { component: slot.default.clone(), inputs: Default::default() });
            let active = slot.when.iter().all(|condition| state.holds(condition));
            state.items.push(Item { slot, pick, active });
        }
        state
    }

    /// The component an active slot holds.
    pub fn component(&self, slot: &str) -> Option<&str> {
        let item = self.items.iter().find(|item| item.slot.id == slot)?;
        match item.active {
            true => item.pick.component.as_deref(),
            false => None,
        }
    }

    pub fn holds(&self, condition: &Condition) -> bool {
        let held = self.component(condition.slot());
        match condition {
            Condition::Is { any_of, .. } => held.is_some_and(|c| any_of.iter().any(|a| a == c)),
            Condition::IsNot { any_of, .. } => held.is_none_or(|c| !any_of.iter().any(|a| a == c)),
            Condition::Filled { .. } => held.is_some(),
            Condition::Empty { .. } => held.is_none(),
            Condition::Tier { at_least, .. } => {
                held.and_then(|c| self.catalog.get(c)).map_or(0, |component| component.tier) >= *at_least
            }
        }
    }
}

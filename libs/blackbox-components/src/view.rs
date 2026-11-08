//! What the garage and shop screens need: the slots of a vehicle with the state of each option, and the price of a change.

use crate::model::{Catalog, ComponentId, Condition, SlotId, VehicleDef};
use crate::selection::{Pick, Selection};
use crate::state::State;
use crate::validate::Gate;

/// Whether an option can be picked right now.
#[derive(Debug, Clone, PartialEq)]
pub enum OptionState {
    /// It is what the slot holds.
    Selected,
    Available,
    /// It needs an unlock the gate does not give.
    Locked {
        unlock: String,
    },
    /// Rules of the component or the offer fail with the rest of the selection; the screen can say which.
    Blocked(Vec<Condition>),
}

#[derive(Debug, Clone, PartialEq)]
pub struct OptionView {
    pub component: ComponentId,
    pub name: String,
    pub tier: u32,
    pub price: u32,
    pub state: OptionState,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SlotView {
    pub slot: SlotId,
    pub group: String,
    /// The slot is shown only while it is active.
    pub active: bool,
    pub selected: Option<ComponentId>,
    pub can_be_empty: bool,
    pub options: Vec<OptionView>,
}

/// One line of a quote.
#[derive(Debug, Clone, PartialEq)]
pub struct Line {
    pub slot: SlotId,
    pub component: ComponentId,
    pub price: u32,
}

/// What a change installs and what it takes off. The game decides what the removed parts are worth back.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Quote {
    pub install: Vec<Line>,
    pub remove: Vec<Line>,
}

impl Quote {
    pub fn install_total(&self) -> u64 {
        self.install.iter().map(|line| u64::from(line.price)).sum()
    }
}

impl VehicleDef {
    /// The slots in the vehicle's order, each with its options and their states under `selection`.
    pub fn slot_views(&self, catalog: &Catalog, selection: &Selection, gate: &dyn Gate) -> Vec<SlotView> {
        let state = State::new(self, catalog, selection);
        let mut views = Vec::with_capacity(state.items.len());
        for item in &state.items {
            let held = item.pick.component.as_deref();
            let mut options = Vec::with_capacity(item.slot.accepts.len());
            for offer in &item.slot.accepts {
                let Some(component) = catalog.get(&offer.component) else { continue };
                let blocked: Vec<Condition> = {
                    // The option as if it were picked: only the rules about *other* slots can fail.
                    let mut trial = selection.clone();
                    trial.set(&item.slot.id, Pick::of(&offer.component));
                    let trial = State::new(self, catalog, &trial);
                    offer.when.iter().chain(&component.when).filter(|c| !trial.holds(c)).cloned().collect()
                };
                let state = match (&component.unlock, held == Some(offer.component.as_str())) {
                    (_, true) => OptionState::Selected,
                    _ if !blocked.is_empty() => OptionState::Blocked(blocked),
                    (Some(unlock), _) if !gate.unlocked(unlock) => OptionState::Locked { unlock: unlock.clone() },
                    _ => OptionState::Available,
                };
                options.push(OptionView {
                    component: component.id.clone(),
                    name: component.display_name().into(),
                    tier: component.tier,
                    price: offer.price.unwrap_or(component.price),
                    state,
                });
            }
            views.push(SlotView {
                slot: item.slot.id.clone(),
                group: item.slot.group.clone(),
                active: item.active,
                selected: held.map(Into::into),
                can_be_empty: item.slot.can_be_empty(),
                options,
            });
        }
        views
    }

    /// The price of going from `from` to `to`: every active slot whose component differs.
    pub fn quote(&self, catalog: &Catalog, from: &Selection, to: &Selection) -> Quote {
        let (before, after) = (State::new(self, catalog, from), State::new(self, catalog, to));
        let mut quote = Quote::default();
        let price = |slot: &crate::model::Slot, component: &str| {
            let offer = slot.offer(component);
            offer.and_then(|o| o.price).or_else(|| catalog.get(component).map(|c| c.price)).unwrap_or(0)
        };
        for (old, new) in before.items.iter().zip(&after.items) {
            let old_id = old.pick.component.as_deref().filter(|_| old.active);
            let new_id = new.pick.component.as_deref().filter(|_| new.active);
            if old_id == new_id {
                continue;
            }
            if let Some(id) = old_id {
                quote.remove.push(Line { slot: old.slot.id.clone(), component: id.into(), price: price(old.slot, id) });
            }
            if let Some(id) = new_id {
                quote.install.push(Line {
                    slot: new.slot.id.clone(),
                    component: id.into(),
                    price: price(new.slot, id),
                });
            }
        }
        quote
    }
}

//! Components fitted together: an [`Offer`]'s `bundle`.
//!
//! A vehicle can say, on the offer of a component, which components of **other slots** come with it: a performance level
//! (`brakes level 2`) and the parts that belong to it (`brake discs of kit 2`). The bundle is the vehicle's, not the
//! component's, because the same level component is shared by many vehicles whose parts differ. When the player changes a slot,
//! [`VehicleDef::bundle_changes`] says what else changes with it:
//!
//! 1. the bundle of the component the slot **held** is taken away: a bundled component that is still installed goes back to
//!    its slot's default (lowering the level removes its parts);
//! 2. the bundle of the component the slot **now holds** is installed (raising it fits them).
//!
//! The link is one way: choosing a bundled part by itself does not change the level. A bundle is not followed further (the
//! bundle of a bundled component is not read), so a mod cannot make a chain.

use crate::model::{Slot, VehicleDef};
use crate::selection::{Change, Pick, Selection};

impl VehicleDef {
    /// The component `slot` holds in `selection` (its default when the selection has no pick for it).
    fn held<'a>(slot: &'a Slot, selection: &'a Selection) -> Option<&'a str> {
        match selection.get(&slot.id) {
            Some(pick) => pick.component.as_deref(),
            None => slot.default.as_deref(),
        }
    }

    /// The slot of this vehicle other than `except` that accepts `component`.
    fn slot_accepting(&self, component: &str, except: &str) -> Option<&Slot> {
        self.slots.iter().find(|slot| slot.id != except && slot.offer(component).is_some())
    }

    /// The edits that go with the change from `from` to `to`, so that `to` plus them is the selection to install. Empty when
    /// nothing that changed has a bundle. Both selections are read as they are (a slot with no pick holds its default).
    pub fn bundle_changes(&self, from: &Selection, to: &Selection) -> Vec<Change> {
        let changed: Vec<&Slot> =
            self.slots.iter().filter(|slot| Self::held(slot, from) != Self::held(slot, to)).collect();
        let mut result = to.clone();
        for slot in &changed {
            let Some(old) = Self::held(slot, from).and_then(|id| slot.offer(id)) else { continue };
            for id in &old.bundle {
                let Some(target) = self.slot_accepting(id, &slot.id) else { continue };
                if Self::held(target, &result) == Some(id.as_str()) {
                    result.reset(&target.id);
                }
            }
        }
        for slot in &changed {
            let Some(new) = Self::held(slot, to).and_then(|id| slot.offer(id)) else { continue };
            for id in &new.bundle {
                let Some(target) = self.slot_accepting(id, &slot.id) else { continue };
                if target.default.as_deref() == Some(id.as_str()) {
                    result.reset(&target.id);
                    continue;
                }
                result.set(&target.id, Pick::of(id));
            }
        }
        to.canonical(self).diff(&result.canonical(self))
    }
}

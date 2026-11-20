//! Groups of slots: the sections a vehicle's data lists its slots under, and the switch of a group of optional ones.

use nfsmw_data::car::customization::Catalog;

use super::{Edit, Slot, Store, VehicleOptions};

/// The slots one section lists.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Group {
    pub name: String,
    /// The ids of its slots.
    pub slots: Vec<String>,
    /// How many of them hold something but their stock choice.
    pub changed: usize,
    /// Whether the group can be switched off and on: every slot of it can be emptied.
    pub switchable: bool,
    /// For a switchable group: whether every slot holds a component.
    pub on: bool,
}

/// Whether the slot can be emptied: a slot kept for the session that has a choice for nothing.
fn emptiable(slot: &Slot) -> bool {
    slot.store == Store::Session && slot.empty_choice().is_some()
}

/// Whether the slot holds a component now (it is not emptied).
fn filled(slot: &Slot) -> bool {
    slot.empty_choice() != Some(slot.current)
}

impl VehicleOptions {
    /// The groups, in the order their first slot appears.
    pub(crate) fn groups(&self) -> Vec<Group> {
        let mut groups: Vec<Group> = Vec::new();
        for slot in &self.slots {
            if !groups.iter().any(|group| group.name == slot.group) {
                groups.push(Group {
                    name: slot.group.clone(),
                    slots: Vec::new(),
                    changed: 0,
                    switchable: true,
                    on: true,
                });
            }
            let Some(group) = groups.iter_mut().find(|group| group.name == slot.group) else { continue };
            group.slots.push(slot.id.clone());
            group.changed += usize::from(slot.changed());
            group.switchable &= emptiable(slot);
            group.on &= filled(slot);
        }
        groups
    }

    pub(super) fn unknown_group(&self, name: &str) -> String {
        let known: Vec<String> = self.groups().into_iter().map(|group| group.name).collect();
        format!("{} has no group {name:?} (it has: {})", self.name, known.join(", "))
    }

    /// Switches a group of optional slots: off empties each, on gives each its stock component (or its first when the stock one
    /// is nothing). A group that has a slot which cannot be emptied is refused.
    pub(super) fn switch_group(
        &self,
        catalog: Option<&Catalog>,
        edit: &mut Edit<'_>,
        name: &str,
        on: bool,
    ) -> Result<String, String> {
        let slots: Vec<&Slot> = self.in_group(name).collect();
        if slots.is_empty() {
            return Err(self.unknown_group(name));
        }
        if let Some(slot) = slots.iter().find(|slot| !emptiable(slot)) {
            return Err(format!(
                "group {name} cannot be switched: its slot {} always holds a component (use vehicle set, or vehicle group {name} stock)",
                slot.id
            ));
        }
        for slot in slots {
            let empty = slot.empty_choice().unwrap_or(0);
            let installed = (0..slot.choices.len()).find(|at| *at != empty).unwrap_or(0);
            // A default that is nothing counts as the empty choice, so "on" takes the first real one.
            let target = match on {
                true => installed,
                false => empty,
            };
            self.put(catalog, edit, slot, target)?;
        }
        Ok(format!(
            "group {name}: {}",
            match on {
                true => "on",
                false => "off",
            }
        ))
    }
}

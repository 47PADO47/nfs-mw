//! Which kind of real car each car slot of a NIS scene takes, and which car a cop channel takes. Slot 0 is the player's car
//! (`car1`). Slots 1 to 7 (`car2`..`car8`) are civilian channels: they take traffic cars and never a police car. Slots 8 to
//! 15 (`cop1`..`cop8`) are police channels: they take the nearest free police car, or a police car the scene spawns when
//! none is free. Rules: `docs/specs/nis-player.md` sections 7.1 and 7.5. The links are made in `car_world.rs`.

use super::cars::{CAR_SLOTS, Target};

/// The first slot of the police channels: slots from here on are `cop1`..`cop8`.
const FIRST_COP_SLOT: usize = CAR_SLOTS / 2;

/// What a car slot takes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SlotKind {
    /// The player's car (slot 0).
    Player,
    /// A civilian traffic car (slots 1 to 7).
    Civilian,
    /// A police car (slots 8 to 15).
    Cop,
}

/// The kind of car slot `slot` is, or `None` for a slot past the last one.
pub fn slot_kind(slot: usize) -> Option<SlotKind> {
    if slot >= CAR_SLOTS {
        return None;
    }
    let kind = match slot {
        0 => SlotKind::Player,
        s if s < FIRST_COP_SLOT => SlotKind::Civilian,
        _ => SlotKind::Cop,
    };
    Some(kind)
}

/// The car a cop channel takes: `borrowed` is the nearest free police car of the chase, when there is one; without it the
/// car `spawn` puts on the road for the scene is taken. `None` when neither gives a car. `spawn` is only called when no
/// police car is free.
pub fn choose_cop(borrowed: Option<usize>, spawn: impl FnOnce() -> Option<usize>) -> Option<Target> {
    if let Some(index) = borrowed {
        return Some(Target::Traffic(index));
    }
    spawn().map(Target::Spawned)
}

#[cfg(test)]
#[path = "cop_cars_tests.rs"]
mod tests;

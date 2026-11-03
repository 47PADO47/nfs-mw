//! Which kind of real car each car slot of a NIS scene takes, and which car a cop channel takes. Slot 0 is the player's car
//! (`car1`). Slots 1 to 7 (`car2`..`car8`) are civilian channels: they take traffic cars and never a police car. Slots 8 to
//! 15 (`cop1`..`cop8`) are police channels: they take the nearest free police car, or a police car the scene spawns when
//! none is free. An arrest spawns its police car first: the original's arrest cop is a NIS car of its own, not a car of
//! the chase. Rules: `docs/specs/nis-player.md` sections 7.1, 7.5 and 8. The links are made in `car_world.rs`.

use super::cars::{CAR_SLOTS, Target};
use super::kind::NisKind;

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

/// Where a cop channel looks for its police car first.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CopSource {
    /// The nearest free police car of the chase, then a car the scene spawns.
    BorrowFirst,
    /// A car the scene spawns (the scene's own police model), then the nearest free police car of the chase.
    SpawnFirst,
}

/// Where the cop channels of a scene of `kind` look first. An arrest's cop channel is the original's NIS police car: a car
/// the scene creates from its special car list (one `copmidsize_nis` for every arrest), owned by the NIS and in neither
/// the traffic nor the pursuit lists (`nis-player.md` sections 7.1 and 8) [decomp; verified table]. A chase car would put
/// the perp into whatever the chase drives (an SUV, a roadblock car) and leave a hole where it stood. The fallback to a
/// borrowed car, so the perp is never walked into an empty spot, is [ours]. Other scenes keep borrowing first [ours].
pub fn cop_source(kind: NisKind) -> CopSource {
    match kind {
        NisKind::Arrest => CopSource::SpawnFirst,
        _ => CopSource::BorrowFirst,
    }
}

/// The car a cop channel takes: `borrowed` is the nearest free police car of the chase, when there is one, and `spawn`
/// puts a police car on the road for the scene. `source` says which is tried first. `None` when neither gives a car.
/// `spawn` is called only when the source tries it: first, or when no police car is free.
pub fn choose_cop(source: CopSource, borrowed: Option<usize>, spawn: impl FnOnce() -> Option<usize>) -> Option<Target> {
    if source == CopSource::BorrowFirst
        && let Some(index) = borrowed
    {
        return Some(Target::Traffic(index));
    }
    if let Some(index) = spawn() {
        return Some(Target::Spawned(index));
    }
    borrowed.map(Target::Traffic)
}

#[cfg(test)]
#[path = "cop_cars_tests.rs"]
mod tests;

//! Instance keys in the world: a stable name for each placed object, so a renderer that keeps objects
//! between frames (and temporal methods that need last frame's pose) can follow them.
//!
//! Scenery is `(tile, index in the tile's placed list)`; the shared sets, which are not a tile, have
//! their own group. A car is `(car, part)` in groups that start above every tile.

use blackbox_gfx::InstanceKey;

/// The group of the shared sets' scenery.
const SHARED_GROUP: u32 = 0x7FFF_FFFF;
/// Car groups start here; a tile is a number below it.
const CAR_GROUPS: u32 = 0x8000_0000;

/// The car the player drives.
pub const PLAYER_CAR: u32 = 0;

/// A fresh identity for one placed vehicle, including a trailer. Shared rigs keep their own default identity.
pub fn next_car() -> u32 {
    use std::sync::atomic::{AtomicU32, Ordering};

    static NEXT: AtomicU32 = AtomicU32::new(1);
    let mut id = NEXT.load(Ordering::Relaxed);
    loop {
        assert!(id < CAR_GROUPS, "vehicle instance-key groups exhausted");
        match NEXT.compare_exchange_weak(id, id + 1, Ordering::Relaxed, Ordering::Relaxed) {
            Ok(_) => return id,
            Err(current) => id = current,
        }
    }
}

/// The `index`th placed object of map tile `tile`.
pub fn tile(tile: usize, index: usize) -> InstanceKey {
    InstanceKey::new(tile as u32, index as u32)
}

/// The `index`th placed object of the shared sets.
pub fn shared(index: usize) -> InstanceKey {
    InstanceKey::new(SHARED_GROUP, index as u32)
}

/// The `part`th placement of the model of car `car`.
pub fn car_part(car: u32, part: usize) -> InstanceKey {
    InstanceKey::new(CAR_GROUPS + car, part as u32)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scenery_and_cars_never_share_a_key() {
        let (first, second) = (next_car(), next_car());
        let keys = [
            tile(0, 0),
            tile(0, 1),
            tile(1, 0),
            shared(0),
            shared(1),
            car_part(PLAYER_CAR, 0),
            car_part(first, 0),
            car_part(first, 1),
            car_part(second, 0),
            car_part(second, 1),
        ];
        assert_eq!(car_part(first, 0).parts().0, car_part(first, 1).parts().0);
        for (i, a) in keys.iter().enumerate() {
            assert!(!a.is_transient());
            for b in &keys[i + 1..] {
                assert_ne!(a, b);
            }
        }
    }
}

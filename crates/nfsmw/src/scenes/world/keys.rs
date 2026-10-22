//! Instance keys in the world: a stable name for each placed object, so a renderer that keeps objects
//! between frames (and temporal methods that need last frame's pose) can follow them.
//!
//! Scenery is `(tile, index in the tile's placed list)`; the shared sets, which are not a tile, have
//! their own group. A car is `(car, part)` in groups that start above every tile. A NIS character is
//! `(actor, part)` in one group near the top of the range.

use blackbox_gfx::InstanceKey;

/// The group of the shared sets' scenery.
const SHARED_GROUP: u32 = 0x7FFF_FFFF;
/// Car groups start here; a tile is a number below it.
const CAR_GROUPS: u32 = 0x8000_0000;
/// The group of the characters of a NIS scene. Car groups count up from [`CAR_GROUPS`] and never reach it.
const CHARACTER_GROUP: u32 = 0xFFFF_0000;

/// The car the player drives.
pub const PLAYER_CAR: u32 = 0;

/// The car render keys of one world: every placed vehicle, trailer and uploaded car rig takes the next one. Each world
/// owns its own (`WorldScene::car_keys`, shared with the traffic models through an `Rc`), so two worlds in one process (a
/// server and a client, or two tests) number their cars independently and the same way every run
/// (`docs/specs/multiplayer/determinism.md`, D5). Shared rigs keep their own default identity.
#[derive(Debug)]
pub struct CarKeys {
    next: std::cell::Cell<u32>,
}

impl Default for CarKeys {
    /// Starts at 1: [`PLAYER_CAR`] (0) belongs to the lead's car.
    fn default() -> Self {
        Self::starting_at(1)
    }
}

impl CarKeys {
    /// An allocator whose first key is `first`.
    pub fn starting_at(first: u32) -> Self {
        Self { next: std::cell::Cell::new(first) }
    }

    /// A fresh identity for one placed vehicle, including a trailer.
    pub fn next(&self) -> u32 {
        let id = self.next.get();
        assert!(id < CAR_GROUPS, "vehicle instance-key groups exhausted");
        self.next.set(id + 1);
        id
    }
}

/// Where the keys of tests that run without a world start: far above what a test world's own allocator reaches.
#[cfg(test)]
const TEST_KEYS: u32 = 0x4000_0000;

/// The allocator of tests that place cars without a world, one per test thread.
#[cfg(test)]
pub fn test_keys() -> std::rc::Rc<CarKeys> {
    thread_local! {
        static KEYS: std::rc::Rc<CarKeys> = std::rc::Rc::new(CarKeys::starting_at(TEST_KEYS));
    }
    KEYS.with(std::rc::Rc::clone)
}

/// A fresh key from [`test_keys`], for tests that upload a rig without a world.
#[cfg(test)]
pub fn next_car() -> u32 {
    test_keys().next()
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

/// The `part`th placement of character `actor` of the playing NIS scene. The actor takes the high 16 bits of the index.
pub fn character(actor: usize, part: usize) -> InstanceKey {
    InstanceKey::new(CHARACTER_GROUP, ((actor as u32) << 16) | part as u32)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn two_worlds_number_their_cars_independently() {
        let (server, client) = (CarKeys::default(), CarKeys::default());
        assert_eq!([server.next(), server.next(), server.next()], [1, 2, 3]);
        assert_eq!([client.next(), client.next()], [1, 2], "one world's cars do not advance the other's keys");
        assert_eq!(server.next(), 4);
    }

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
            character(0, 0),
            character(0, 1),
            character(1, 0),
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

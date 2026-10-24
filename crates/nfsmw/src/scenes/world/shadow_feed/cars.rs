//! The traffic cars as every view without per-car detail draws them, built once for the consumers of one frame.
//!
//! The sun-shadow feed and the car environment capture draw the same cars from the same poses, so the second one reads
//! the first one's list. The list is only good until the traffic moves: the frame calls [`SharedCars::invalidate`]
//! once both have run, and a consumer that asks twice in a row gets a new list the second time.
use blackbox_gfx::Instance;

use crate::scenes::world::ai::TrafficWorld;

/// The views that read the shared list.
#[derive(Clone, Copy)]
pub(in crate::scenes::world) enum User {
    Shadows,
    Reflections,
}

#[derive(Default)]
pub(in crate::scenes::world) struct SharedCars {
    list: Vec<Instance>,
    fresh: bool,
    /// Which users have read the list since it was built, by `User` order.
    taken: [bool; 2],
}

impl SharedCars {
    /// The instances of `traffic` (`TrafficWorld::instances`) for `user`: the list another user built this frame, or a
    /// new one when there is none or `user` has read it already.
    pub fn get(&mut self, traffic: &TrafficWorld, user: User) -> &[Instance] {
        let slot = user as usize;
        if self.fresh && !self.taken[slot] {
            self.taken[slot] = true;
            return &self.list;
        }
        self.list.clear();
        traffic.instances(&mut self.list);
        self.fresh = true;
        self.taken = [false; 2];
        self.taken[slot] = true;
        &self.list
    }

    /// The traffic is about to change: the next `get` builds the list again.
    pub fn invalidate(&mut self) {
        self.fresh = false;
    }
}

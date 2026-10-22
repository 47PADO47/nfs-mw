//! The cop channels of a NIS scene that wait for a police car. A cop channel with no free police car to borrow waits while
//! the police car model the scene spawns (`ai/nis_poses.rs`) loads and while its spawn is tried. The world retries the
//! waiting channels every frame and gives them up after [`WAIT_LIMIT`] real seconds. This holds the slots and the time
//! only; the links the channels get are in `player.rs`, and the retries in `car_world.rs`.

/// How long, in real seconds, the cop channels of a scene wait for a police car before they give up and stay off.
pub const WAIT_LIMIT: f32 = 2.0;

/// The cop channels still waiting for a police car, and the real time they have waited.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct CopWait {
    /// The slots still waiting, in slot order.
    slots: Vec<usize>,
    /// The real seconds since the wait began.
    elapsed: f32,
}

impl CopWait {
    /// A wait for `slots`. Empty when no channel waits.
    pub fn new(slots: Vec<usize>) -> Self {
        Self { slots, elapsed: 0.0 }
    }

    /// The slots still waiting.
    pub fn slots(&self) -> &[usize] {
        &self.slots
    }

    /// Takes `slot` off the wait: its channel got its car.
    pub fn resolve(&mut self, slot: usize) {
        self.slots.retain(|&waiting| waiting != slot);
    }

    /// Spends `dt` real seconds on the wait. Returns nothing while the time is under [`WAIT_LIMIT`]. Once it is reached,
    /// the slots still waiting give up: they are returned, and the wait is empty from then on.
    pub fn tick(&mut self, dt: f32) -> Vec<usize> {
        if self.slots.is_empty() {
            return Vec::new();
        }
        self.elapsed += dt;
        if self.elapsed < WAIT_LIMIT {
            return Vec::new();
        }
        std::mem::take(&mut self.slots)
    }
}

#[cfg(test)]
#[path = "cop_wait_tests.rs"]
mod tests;

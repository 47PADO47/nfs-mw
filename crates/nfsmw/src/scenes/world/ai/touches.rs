//! The car pairs that touched in the last tick, for a server's contact events (`docs/specs/multiplayer/world-host.md`).
//! `collide` starts the list each tick and `collide_guests` adds to it; nothing in the world reads it, so single player
//! runs as before.

use super::TrafficWorld;

/// One side of a contact.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Toucher {
    /// The player's car `collide` was given: the one the police follow.
    Followed,
    /// The guest at this index of the list `collide_guests` was given.
    Guest(usize),
    /// A traffic, police or roadblock car (or the trailer it tows), by entity id (`AiCar::key`).
    Car(u32),
}

/// Two bodies that touched in a tick and how fast they closed before (m/s).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CarTouch {
    pub a: Toucher,
    pub b: Toucher,
    pub speed: f32,
}

impl TrafficWorld {
    /// The pairs `collide` found, by car index (`None` is the followed player), as entity ids.
    pub(super) fn note_touches(&mut self, touched: &[(Option<usize>, usize, f32)]) {
        let side = |index: Option<usize>| match index {
            Some(index) => Toucher::Car(self.cars[index].key()),
            None => Toucher::Followed,
        };
        let touches: Vec<CarTouch> =
            touched.iter().map(|&(a, b, speed)| CarTouch { a: side(a), b: side(Some(b)), speed }).collect();
        self.touches.extend(touches);
    }

    /// The pairs that touched in the last tick, each once.
    pub fn touches(&self) -> &[CarTouch] {
        &self.touches
    }
}

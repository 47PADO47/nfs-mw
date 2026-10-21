//! The state of a police car's siren, as pure rules. Two pieces:
//!
//! - [`Machine`]: the car side. Each tick it picks the siren state (off, yelp, wail, scream, die) from what the chase
//!   is doing: the opening exchange, lost contact, a formation being pressed, a roadblock, the light bar.
//! - [`Voice`]: the sound side. It follows the car's state with a random lag, decides whether the siren voice is
//!   switched on and which of the siren sound's types it plays (see [`voice`]).
//!
//! Nothing here reads a clock or a random source: the caller passes the time (seconds on one monotonic clock) and
//! uniform random numbers in `0..1`. Spec: `docs/specs/cop-siren-audio.md`.

mod machine;
mod voice;

pub use machine::{Input, Machine, OpeningGap, Phase, Tuning};
pub use voice::Voice;

/// The siren state of one car.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum State {
    /// Silent.
    #[default]
    Off,
    /// The long rising and falling tone of a chase underway.
    Wail,
    /// The fast two-tone of a chase that has just begun, or that is pressing.
    Yelp,
    /// The harsh tone of a hot chase that has just begun, or whose formation is closing.
    Scream,
    /// The wind-down of a wrecked car's siren.
    Die,
}

impl State {
    /// The word the siren sound's `TYPE` input takes: 0 wail, 1 yelp, 2 scream, 3 die; -1 for off. These are the
    /// original's numbers.
    pub fn word(self) -> i32 {
        match self {
            State::Off => -1,
            State::Wail => 0,
            State::Yelp => 1,
            State::Scream => 2,
            State::Die => 3,
        }
    }
}

#[cfg(test)]
mod tests;

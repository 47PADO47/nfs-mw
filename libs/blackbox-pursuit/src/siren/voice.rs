//! The sound side of the siren: one siren voice, attached to a police car near the listener. It does not play the
//! car's state at once: it holds the state it last adopted and takes the car's state again after a random wait, so
//! several sirens drift out of step. Spec: `docs/specs/cop-siren-audio.md` section 3.

use super::State;

/// A new voice adopts nothing for this long (plus up to [`START_SPREAD`]), seconds.
const START_DELAY: f32 = 6.0;
const START_SPREAD: f32 = 2.0;
/// After each adoption the next one waits up to this long, seconds.
const ADOPT_SPREAD: f32 = 3.0;
/// A siren that has been dying this long is switched off, seconds.
const DEATH_SECONDS: f64 = 10.0;

/// One siren voice's follower.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Voice {
    /// The state it plays now.
    held: State,
    /// Seconds until it looks at the car's state again.
    wait: f32,
    /// When it last played a state other than the dying one, or began to die (seconds on the caller's clock).
    alive_at: f64,
}

impl Voice {
    /// A voice attached at time `now`. `random` is a uniform number in `0..1`.
    pub fn new(now: f64, random: f32) -> Self {
        Self { held: State::Off, wait: START_DELAY + random.clamp(0.0, 1.0) * START_SPREAD, alive_at: now }
    }

    /// The state the voice holds (off until it adopts one).
    pub fn held(&self) -> State {
        self.held
    }

    /// Runs one tick of `dt` seconds at time `now` with the car showing `car`. Returns the `TYPE` word the siren
    /// sound should play, or `None` when the voice is switched off.
    ///
    /// A voice that has not yet adopted a state plays the wail (type 0): the original only switches a voice off by
    /// the car's own state, never by the voice's.
    pub fn update(&mut self, now: f64, dt: f32, car: State, random: f32) -> Option<i32> {
        if car == State::Off || (self.held == State::Die && now - self.alive_at > DEATH_SECONDS) {
            return None;
        }
        self.wait -= dt;
        let before = self.held;
        if self.wait < 0.0 || car == State::Scream {
            self.wait = random.clamp(0.0, 1.0) * ADOPT_SPREAD;
            self.held = car;
        }
        match self.held {
            State::Wail | State::Yelp | State::Scream => self.alive_at = now,
            State::Die if before != State::Die => self.alive_at = now,
            State::Die | State::Off => {}
        }
        Some(self.held.word().max(0))
    }
}

//! The weather cue: the one chase edge the speech machine does not own. The lines of the chase itself (a sighting lost
//! or regained, the heat jumping, the search's time running out, a roadblock, a hit, a cop wrecked) come from the speech
//! machine in `blackbox-pursuit`, run by `chase/radio`; the rain is the scene's, so its rising edge is read here. Pure:
//! no sound, no game.
//!
//! The edge is a rising one: the first frame of a chase only records the rain, so a chase that opens already raining
//! says nothing.

use crate::audio::PursuitFeed;

/// The rain, 0 (dry) to 1 (heavy), at which the weather is reported. **[guess]**: the feed does not say where the
/// original sets this; half the scale is the middle of a rain that a line would mention.
pub const RAIN_THRESHOLD: f32 = 0.5;

/// A change of the chase that the speech answers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Cue {
    /// The rain rose to `RAIN_THRESHOLD` during the chase.
    Rain,
}

/// The state of the chase the rain edge is found from. Kept across frames by the audio.
#[derive(Default)]
pub struct Cues {
    /// Whether it rained on the last frame of the chase; `None` before the chase's first frame.
    raining: Option<bool>,
}

impl Cues {
    pub fn new() -> Self {
        Self::default()
    }

    /// Once per frame: the cue of this frame. `None` (no chase) forgets the chase, so the next one starts fresh.
    pub fn step(&mut self, chase: Option<&PursuitFeed>) -> Option<Cue> {
        let Some(chase) = chase else {
            self.raining = None;
            return None;
        };
        let now = chase.rain_intensity >= RAIN_THRESHOLD;
        let before = self.raining.replace(now);
        match (before, now) {
            (Some(false), true) => Some(Cue::Rain),
            _ => None,
        }
    }
}

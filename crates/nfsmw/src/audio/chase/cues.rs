//! The mid-chase cues: edges of the chase as the sound reads it each frame. A cue fires once per edge, so the
//! speech is asked once where the chase changes, not every frame it stays changed. Pure: no sound, no game.
//!
//! Sight is read from the feed's `intensity`, which the world sets to exactly 1 while the cops see the player and
//! below 1 once the unseen progress has begun (see `chase_intensity` in the world's police code). So "seen" is
//! `intensity >= 1`. The one blur: an unseen spell that has not yet progressed reads as seen for that frame.

use crate::audio::PursuitFeed;
use crate::audio::speech::random::Rng;

/// Seconds without sight before the cops call the visual lost: the original's `NoLOSCommentaryTime` (2 s).
pub const NO_CONTACT_SECS: f32 = 2.0;
/// Seconds into a chase before a heat jump may be said: the original's "after 5 s of chase".
pub const HEAT_JUMP_AFTER_SECS: f32 = 5.0;
/// The heat levels whose crossing is a heat jump (`heatjump` says the radio's reaction to them).
const JUMP_LEVELS: [u32; 4] = [2, 3, 4, 5];
/// The chance that a heat jump is said at all: the original's coin flip.
const HEAT_JUMP_CHANCE: f32 = 0.5;

/// A change of the chase that the speech answers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Cue {
    /// The cops have not seen the player for `NO_CONTACT_SECS`.
    Lost,
    /// The cops see the player again after a `Lost`.
    Regained,
    /// The heat rose across one of `JUMP_LEVELS`.
    HeatJump,
}

/// The state of the chase the edges are found from. Kept across frames by the audio.
pub struct Cues {
    /// The heat of the last frame of the chase.
    heat: Option<u32>,
    /// The chase time when the sight was lost, while it stays lost.
    unseen_since: Option<f32>,
    /// A `Lost` was said and no `Regained` has been since.
    lost_said: bool,
    rng: Rng,
}

impl Cues {
    pub fn new(rng: Rng) -> Self {
        Self { heat: None, unseen_since: None, lost_said: false, rng }
    }

    /// Once per frame: the cues of this frame. `None` (no chase) forgets the chase, so the next one starts fresh.
    pub fn step(&mut self, chase: Option<&PursuitFeed>) -> Vec<Cue> {
        let Some(chase) = chase else {
            self.forget();
            return Vec::new();
        };
        let mut cues = Vec::new();
        cues.extend(self.sight(chase));
        cues.extend(self.heat_jump(chase));
        cues
    }

    fn forget(&mut self) {
        self.heat = None;
        self.unseen_since = None;
        self.lost_said = false;
    }

    /// The sight edges: `Lost` once the player has been unseen for `NO_CONTACT_SECS`, `Regained` when seen again
    /// after that. A short unseen spell says nothing.
    fn sight(&mut self, chase: &PursuitFeed) -> Option<Cue> {
        if chase.intensity >= 1.0 {
            self.unseen_since = None;
            return self.regained();
        }
        let since = *self.unseen_since.get_or_insert(chase.secs);
        if self.lost_said || chase.secs - since < NO_CONTACT_SECS {
            return None;
        }
        self.lost_said = true;
        Some(Cue::Lost)
    }

    fn regained(&mut self) -> Option<Cue> {
        if !self.lost_said {
            return None;
        }
        self.lost_said = false;
        Some(Cue::Regained)
    }

    /// The heat edge: the heat rising across one of `JUMP_LEVELS`, after `HEAT_JUMP_AFTER_SECS` of chase, with the
    /// coin flip. The first frame of a chase only records the heat.
    fn heat_jump(&mut self, chase: &PursuitFeed) -> Option<Cue> {
        let before = self.heat.replace(chase.heat)?;
        let crossed = JUMP_LEVELS.iter().any(|&level| before < level && level <= chase.heat);
        if !crossed || chase.secs < HEAT_JUMP_AFTER_SECS {
            return None;
        }
        if self.rng.next_f32() >= HEAT_JUMP_CHANCE {
            return None;
        }
        Some(Cue::HeatJump)
    }
}

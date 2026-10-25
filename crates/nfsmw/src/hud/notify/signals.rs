//! The sounds and pad rumbles the notifications ask for (`docs/specs/custom-hud/feedback.md` sections 2.2 and 3),
//! collected here and handed to the HUD's audio and haptics once a frame (`output.rs`).
//!
//! The rules:
//!
//! - **One per notice.** A notice's cue and rumble are raised when it comes up on the screen: a repeat folded into it
//!   (the queue keeps its `started`) raises nothing, and a notice that waits raises them when it shows, or never if it
//!   goes stale first.
//! - **Feedback at once.** A kind with no notice (the nitrous) or a chip of the feedback lane raises its own when its
//!   event comes.
//! - **No stutter.** A cue does not play again within [`CUE_GAP`] seconds of itself.

use super::catalog::{AudioCue, Haptic, NoticeSpec};

/// Seconds within which a cue does not replay.
pub const CUE_GAP: f32 = 0.5;

/// The cues and rumbles raised since the plugin last took them.
#[derive(Debug, Clone, Default)]
pub struct Signals {
    cues: Vec<AudioCue>,
    haptics: Vec<Haptic>,
    /// The clock at which each cue last played.
    last: Vec<(AudioCue, f32)>,
}

impl Signals {
    /// Raise the cue and the rumble `entry` asks for, at `clock` seconds (a monotonic clock).
    pub fn raise(&mut self, entry: &NoticeSpec, clock: f32) {
        if let Some(haptic) = entry.haptic {
            self.haptics.push(haptic);
        }
        let Some(cue) = entry.audio else { return };
        if self.last.iter().any(|&(played, at)| played == cue && clock - at < CUE_GAP) {
            return;
        }
        self.last.retain(|&(played, _)| played != cue);
        self.last.push((cue, clock));
        self.cues.push(cue);
    }

    /// The cues raised since the last call, in order.
    pub fn take_cues(&mut self) -> Vec<AudioCue> {
        std::mem::take(&mut self.cues)
    }

    /// The rumbles raised since the last call, in order.
    pub fn take_haptics(&mut self) -> Vec<Haptic> {
        std::mem::take(&mut self.haptics)
    }
}

//! What a step of the machine gives back: lines to say and things for the game to do.

use super::line::{Line, Speaker};
use super::params::Params;

/// The speaking unit as the queue's checks see it: how far it is and whether it sees the player.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Talker {
    pub distance: f32,
    pub los: bool,
    pub in_view: bool,
}

/// A line to say.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Request {
    pub line: Line,
    /// The voice, 0 when the line has none.
    pub speaker: Speaker,
    pub params: Params,
    pub talker: Option<Talker>,
}

/// Something the game has to do besides speaking.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Effect {
    /// Stop the line that sounds and drop the queue (the original's `ClearPlayback`).
    ClearQueue,
    /// The 911 report is over: the cops stay away for this many seconds and the lockout ends.
    Force911 { secs: f32 },
    /// The busted level for the radio's muffling, 0 to 1, sent on every observation step while a pursuit runs.
    BustedLevel(f32),
    /// Withdraw this line from the queue if it waits there.
    Recall(Line),
    /// Forget what has been said (a pursuit reset starts with a clean history).
    ResetHistory,
}

/// The result of a step.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Output {
    pub requests: Vec<Request>,
    pub effects: Vec<Effect>,
}

impl Output {
    pub fn is_empty(&self) -> bool {
        self.requests.is_empty() && self.effects.is_empty()
    }
}

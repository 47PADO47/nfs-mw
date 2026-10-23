//! A session's race starts when the server says: the racers stand on the grid ("staged") and the countdown waits to be
//! released, so the world's GO falls on the server's start tick (`docs/specs/multiplayer/world-host.md` section 7).

use super::Stage;
use crate::scenes::world::WorldScene;

impl WorldScene {
    /// Whether the next race stages on the grid and waits to be released, instead of counting down at once.
    pub(in crate::scenes::world) fn hold_next_race(&mut self, hold: bool) {
        self.race_hold = hold;
    }

    /// The ticks the staged race's countdown takes to reach GO, `None` unless a race stands on the grid waiting.
    pub(in crate::scenes::world) fn staged_race(&self) -> Option<u32> {
        match self.race.as_ref()?.stage {
            Stage::Staged { lead } => Some(lead),
            _ => None,
        }
    }

    /// Starts the staged race's countdown (or its rolling start). Returns whether a race was waiting.
    pub(in crate::scenes::world) fn release_staged_race(&mut self) -> bool {
        let Some(mut session) = self.race.take() else { return false };
        let waiting = matches!(session.stage, Stage::Staged { .. });
        if waiting {
            self.start_countdown(&mut session);
        }
        self.race = Some(session);
        waiting
    }
}

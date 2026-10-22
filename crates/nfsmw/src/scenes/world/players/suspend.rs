//! Player 2 waits out the story's cutscenes (`docs/specs/coop-career.md` section 12): while an NIS scene or a story
//! movie plays, the guest is parked and the screen is whole; when it is over the guest comes back behind player 1.
//! The story's races park the guest through `race.rs`.

use super::super::WorldScene;
use super::coop::GUEST;

impl WorldScene {
    /// Parks or brings back player 2 around a cutscene. Does nothing outside co-op or while a race holds the guest.
    pub(in crate::scenes::world) fn update_coop_suspend(&mut self) {
        let Some(session) = self.coop.as_mut() else { return };
        let cutscene = self.nis.is_playing() || self.story_movie.is_playing();
        if cutscene && !session.suspended && !self.players.is_parked(GUEST) {
            session.suspended = true;
            self.players.park(GUEST, true);
            return;
        }
        if cutscene || !session.suspended {
            return;
        }
        session.suspended = false;
        self.return_guest_behind_lead();
    }
}

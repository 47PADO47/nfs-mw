//! The guests wait out the story's cutscenes (`docs/specs/coop-career.md` section 12): while an NIS scene or a story
//! movie plays, every guest in play is parked and the screen is whole; when it is over they come back behind player 1.
//! The story's races park player 2 through `race.rs`, and a guest a race holds is not parked or brought back here.

use super::super::WorldScene;

impl WorldScene {
    /// Parks or brings back the guests around a cutscene. Does nothing outside co-op, or while no guest is in play.
    pub(in crate::scenes::world) fn update_coop_suspend(&mut self) {
        let cutscene = self.nis.is_playing() || self.story_movie.is_playing();
        let in_play: Vec<_> = self.players.guests().map(|guest| guest.id).collect();
        let Some(session) = self.coop.as_mut() else { return };
        if cutscene && !session.suspended && !in_play.is_empty() {
            session.suspended = true;
            for &id in &in_play {
                self.players.park(id, true);
            }
            session.cutscene_parked = in_play;
            return;
        }
        if cutscene || !session.suspended {
            return;
        }
        session.suspended = false;
        let parked = std::mem::take(&mut session.cutscene_parked);
        self.return_guests_behind_lead(&parked);
    }
}

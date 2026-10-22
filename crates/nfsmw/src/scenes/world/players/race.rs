//! A co-op race (`docs/specs/coop-career.md` section 7): player 2 drives a race seat on the grid, the same human racer a
//! split-screen race seats (`race/split.rs`), next to player 1 and the race's own opponents (a Blacklist race: the
//! rival). The guest's free-roam car is parked for the race and comes back behind player 1 when it ends.

use super::super::WorldScene;
use nfsmw_data::gameplay::Race;

use super::super::race::{RacerPlan, SplitSetup, is_story_race};
use super::coop::GUEST;
use super::tether::behind;
use crate::input::PlayerDevice;

/// How far behind player 1 player 2 comes back after a race, in metres.
const BACK_BEHIND: f32 = 12.0;

impl WorldScene {
    /// The two people of a co-op race, as a split-screen race takes them; `None` outside co-op, and for the story's
    /// scripted chase and final race, which player 1 drives alone while player 2 waits parked (section 12).
    pub(in crate::scenes::world) fn coop_race_setup(&self, race: &Race) -> Option<SplitSetup> {
        let session = self.coop.as_ref()?;
        if is_story_race(race) {
            return None;
        }
        let lead = self.players.lead()?;
        Some(SplitSetup {
            cars: [lead.car_name.clone(), session.guest_car.clone()],
            // The world never reads the devices; the app seats them from the co-op session (`input/split.rs`).
            devices: [PlayerDevice::Keyboard, PlayerDevice::KeyboardArrows],
            transmissions: [lead.transmission(), session.transmission],
        })
    }

    /// The racers of a co-op race carry the players' names, "<alias> 1" and "<alias> 2".
    pub(in crate::scenes::world) fn name_coop_racers(&self, plan: &mut [RacerPlan]) {
        let Some(session) = self.coop.as_ref() else { return };
        for (racer, name) in plan.iter_mut().zip(&session.names) {
            racer.name = name.clone();
        }
    }

    /// Player 2's free-roam car leaves play for the race.
    pub(in crate::scenes::world) fn park_guest_for_race(&mut self) {
        if self.coop.is_some() {
            self.players.park(GUEST, true);
        }
    }

    /// The race is over: player 2's free-roam car comes back behind player 1.
    pub(in crate::scenes::world) fn unpark_guest_after_race(&mut self) {
        self.return_guest_behind_lead();
    }

    /// Player 2 back in play, behind player 1 and facing the same way.
    pub(in crate::scenes::world) fn return_guest_behind_lead(&mut self) {
        if !self.players.is_parked(GUEST) {
            return;
        }
        let Some(lead) = self.players.lead() else { return };
        let heading = lead.heading();
        let spot = behind(lead.position(), glam::Vec3::new(heading.cos(), heading.sin(), 0.0), BACK_BEHIND);
        self.players.park(GUEST, false);
        if let Some(guest) = self.players.get_mut(GUEST) {
            guest.respawn_near([spot.x, spot.y], Some(heading));
        }
    }
}

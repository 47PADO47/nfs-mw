//! The co-op guests as the traffic sees them (`docs/specs/coop-career.md` section 11): new cars spawn around each
//! player in turn, a traffic car stays while any player sees it or is near it, and the cars steer around the guests'
//! cars as they do around the lead's. The police and their respawns follow the lead until the shared pursuit reads
//! every player (ADR 0010).

use super::visibility::Viewing;
use crate::scenes::world::ai::{Focus, TrafficWorld};
use crate::viewer::camera::Lens;

impl TrafficWorld {
    /// Each guest's focus and camera this tick, the guests in id order. An empty list outside co-op.
    pub fn set_guests(&mut self, guests: impl IntoIterator<Item = (Focus, Lens)>) {
        self.guests.clear();
        self.guests.extend(guests);
        if self.guests.is_empty() {
            self.spawn_turn = 0;
        }
    }

    /// Where the population manager spawns this time: the lead and each guest take turns, so the car budget is
    /// shared out between the players. Always the lead outside co-op.
    pub(in crate::scenes::world::ai) fn next_spawn_focus(&mut self, lead: Focus) -> Focus {
        if self.guests.is_empty() {
            return lead;
        }
        self.spawn_turn = (self.spawn_turn + 1) % (self.guests.len() + 1);
        match self.spawn_turn {
            0 => lead,
            turn => self.guests[turn - 1].0,
        }
    }

    /// The guests' cars as bodies for the traffic to steer around.
    pub(in crate::scenes::world::ai) fn guest_bodies(&self) -> impl Iterator<Item = blackbox_roads::Body> + '_ {
        self.guests.iter().filter_map(|(focus, _)| focus.body)
    }

    /// What each guest sees, with the same retire rules as the lead's view.
    pub(super) fn guest_views(&self) -> Vec<Viewing> {
        self.guests
            .iter()
            .map(|(focus, camera)| {
                let mut view = Viewing::new(camera, *focus);
                view.retire_distance = self.policy.retire_distance as f32;
                view.grace = self.policy.offscreen_seconds as f32;
                view
            })
            .collect()
    }
}

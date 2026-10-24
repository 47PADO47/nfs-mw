//! A co-op race (`docs/specs/coop-career.md` section 7): player 2 drives a race seat on the grid, the same human racer a
//! split-screen race seats (`race/split.rs`), next to player 1 and the race's own opponents (a Blacklist race: the
//! rival). The guest's free-roam car is parked for the race and comes back behind player 1 when it ends.

use super::super::WorldScene;
use nfsmw_data::gameplay::Race;

use super::super::drive::Drive;
use super::super::race::{Opponent, RacerPlan, SplitSetup, is_story_race};
use super::coop::GUEST;
use super::tether::behind;
use super::{LEAD, PlayerId};
use crate::input::PlayerDevice;

/// How far behind player 1 player 2 comes back after a race, in metres.
const BACK_BEHIND: f32 = 12.0;

/// A network seat that races beside the lead: its player, its car folder and its gearbox.
#[derive(Debug, Clone, PartialEq)]
pub(in crate::scenes::world) struct RaceSeat {
    pub id: PlayerId,
    pub car: String,
    pub automatic: bool,
}

impl RaceSeat {
    /// The seat as a racer of the plan: their own car on the next grid slot. Their skill and bias are not used (a person
    /// drives them).
    pub(in crate::scenes::world) fn opponent(&self) -> Opponent {
        Opponent {
            name: "Player".into(),
            name_label: None,
            skill: 1.0,
            bias: 1.0,
            car: Some(self.car.clone()),
            start: None,
        }
    }
}

impl WorldScene {
    /// The two people of a co-op race, as a split-screen race takes them; `None` outside co-op, and for the story's
    /// scripted chase and final race, which player 1 drives alone while player 2 waits parked (section 12).
    pub(in crate::scenes::world) fn coop_race_setup(&self, race: &Race) -> Option<SplitSetup> {
        let session = self.coop.as_ref()?;
        // A network session without player 2 has no guest car for the race seat: player 1 races alone.
        if is_story_race(race) || session.guest_car.is_empty() {
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

    /// The network seats a race takes as people, in id order. A network seat is a player whose input a server holds
    /// ([`Drive::remote_pedals`]); none for the story's scripted races, which the lead drives alone.
    pub(in crate::scenes::world) fn net_race_people(&self, race: &Race) -> Vec<RaceSeat> {
        if is_story_race(race) {
            return Vec::new();
        }
        self.players
            .ids()
            .filter(|&id| id != LEAD)
            .filter_map(|id| {
                let drive = self.players.get(id).filter(|drive| drive.remote_pedals().is_some())?;
                Some(RaceSeat { id, car: drive.car_name.clone(), automatic: drive.transmission().is_automatic() })
            })
            .collect()
    }

    /// Names the people of `plan` (racers 1 to `people.len()`), seats their gearboxes and remembers who they are. A
    /// plan without a slot for each of them (no road network for the racers) is refused.
    pub(in crate::scenes::world) fn seat_race_people(
        &mut self,
        plan: &mut [RacerPlan],
        people: &[RaceSeat],
    ) -> Result<(), String> {
        if plan.len() <= people.len() {
            return Err("a race with remote players needs the road network of the track".into());
        }
        for (racer, seat) in plan.iter_mut().skip(1).zip(people) {
            racer.name = self
                .player_name(seat.id)
                .map_or_else(|| crate::i18n::t!("coop-player", n = u16::from(seat.id.0) + 1), str::to_owned);
            racer.person = Some(seat.automatic);
        }
        self.race_people = people.iter().map(|seat| seat.id).collect();
        Ok(())
    }

    /// Tells each person's racer what their seat's pedals ask, before the physics steps.
    pub(in crate::scenes::world) fn feed_race_people(&mut self) {
        let Some(traffic) = self.traffic.as_mut() else { return };
        for (index, id) in self.race_people.iter().enumerate() {
            let pedals = self.players.get(*id).and_then(Drive::remote_pedals).unwrap_or_default();
            traffic.set_racer_pedals(index + 1, pedals);
        }
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
        if !self.race_people.is_empty() {
            for id in self.race_people.clone() {
                self.players.park(id, true);
            }
            return;
        }
        if self.coop.is_some() {
            self.players.park(GUEST, true);
        }
    }

    /// The race is over: player 2's free-roam car comes back behind player 1.
    pub(in crate::scenes::world) fn unpark_guest_after_race(&mut self) {
        let people = std::mem::take(&mut self.race_people);
        if !people.is_empty() {
            self.return_guests_behind_lead(&people);
            return;
        }
        self.return_guests_behind_lead(&[GUEST]);
    }

    /// The parked players among `ids` back in play, in a line behind player 1 (the first `BACK_BEHIND` metres behind,
    /// each next one as far again) and facing the same way. A player in play or not in the world is left alone.
    pub(in crate::scenes::world) fn return_guests_behind_lead(&mut self, ids: &[PlayerId]) {
        let Some(lead) = self.players.lead() else { return };
        let (position, heading) = (lead.position(), lead.heading());
        let forward = glam::Vec3::new(heading.cos(), heading.sin(), 0.0);
        let parked = ids.iter().copied().filter(|&id| id != LEAD && self.players.is_parked(id));
        let mut rank = 0.0;
        for id in parked.collect::<Vec<_>>() {
            rank += 1.0;
            let spot = behind(position, forward, BACK_BEHIND * rank);
            self.players.park(id, false);
            if let Some(guest) = self.players.get_mut(id) {
                guest.respawn_near([spot.x, spot.y], Some(heading));
            }
        }
    }
}

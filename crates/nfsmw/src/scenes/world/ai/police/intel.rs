//! The police as the HUD may show them (`hud/state/police.rs`): the phase of the chase and what the player can know of
//! the cops. Read-only views of the pursuit's state; the fair-knowledge rules are the HUD state's. Child of `police`, so
//! it reads the police state directly.

use super::*;
use crate::hud::{HelicopterIntel, LastKnown, PoliceIntel, PursuitPhase, known_roadblocks, sighted};
use crate::scenes::world::ai::pursuit_phase::{PhaseInput, phase};

impl super::super::TrafficWorld {
    /// The phase of the chase (`ai/pursuit_phase.rs`).
    pub fn pursuit_phase(&self) -> PursuitPhase {
        phase(&PhaseInput {
            pursuing: self.is_pursuing(),
            status: self.police.cooldown_status(),
            contact: self.police.has_contact() || self.police.helicopter_sees(),
            arrest: self.arrest_active(),
            end: self.police.end_reason(),
            since_end: self.police.since_end(),
        })
    }

    /// What the player can know of the police this frame.
    pub fn police_intel(&self) -> PoliceIntel {
        let cops = self.cars.iter().filter(|car| car.is_live_police() && !car.is_roadblock()).map(|car| {
            let at = car.position();
            ([at.x, at.y], car.sees_player())
        });
        let helicopter = self.police.helicopter_status().zip(self.police.helicopter_blip());
        let reported = self.police.roadblock_standing();
        PoliceIntel {
            sightings: sighted(cops),
            last_known: self.last_known(),
            helicopter: helicopter.map(|(status, (position, _))| HelicopterIntel::new(status.in_sight, position)),
            roadblock_reported: reported,
            roadblocks: known_roadblocks(reported, self.roadblock_sites()),
            cost_to_state: self.cost_to_state(),
            units_involved: self.pursuit.as_ref().map_or(0, |pursuit| pursuit.involved),
        }
    }

    /// Where the police last saw the player in the running chase (the place the search works from), with the seconds
    /// since; `None` without a chase or before a sighting.
    pub fn last_known(&self) -> Option<LastKnown> {
        let seen = self.pursuit.as_ref()?.last_seen?;
        let at = space::to_render(seen.position.to_array());
        Some(LastKnown { position: [at.x, at.y], age_secs: self.police.cooldown.since_seen().max(0.0) })
    }
}

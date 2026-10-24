//! Offence collection on each physics step. Spec: ai-pursuit.md.

use super::super::traffic_events::SPEEDING_MPS;
use super::*;
use blackbox_pursuit::rules::{self, Contact};

impl Police {
    /// The offences of this frame: speeding, and losing the cops (resisting arrest).
    pub(super) fn record_infractions(&mut self, player: &PlayerView, sensed: Sensed) {
        let limits = extra_options::active();
        if rules::speeding(player.speed, limits.speeding_limit().unwrap_or(SPEEDING_MPS)) {
            self.infractions.record(Infraction::Speeding, &self.settings.fines);
        }
        if limits.reckless_limit().is_some_and(|limit| rules::speeding(player.speed, limit)) {
            self.infractions.record(Infraction::RecklessDriving, &self.settings.fines);
        }
        if rules::off_road(sensed.on_road, sensed.seen, sensed.nearest_cop) {
            self.infractions.record(Infraction::OffRoad, &self.settings.fines);
        }
        if sensed.hit_traffic {
            self.record_contact(rules::Target::Traffic, player, sensed, false);
        }
        if sensed.struck_cop {
            self.record_contact(rules::Target::Cop, player, sensed, true);
        }
        if sensed.struck_property {
            self.record_contact(rules::Target::Breakable, player, sensed, false);
        }
        self.seen = sensed.seen;
    }

    /// Records the infraction of a contact with `target`, if the rules make one (`rules::contact_infraction`).
    fn record_contact(&mut self, target: rules::Target, player: &PlayerView, sensed: Sensed, first_damage: bool) {
        let contact = Contact {
            target,
            deliberate: player.throttle > DELIBERATE_THROTTLE,
            closing_speed: player.speed.abs(),
            in_pursuit: true,
            cop_sees: sensed.seen,
            nearest_cop: sensed.nearest_cop,
            first_damage,
        };
        let Some(infraction) = rules::contact_infraction(&contact) else { return };
        self.infractions.record(infraction, &self.settings.fines);
    }
}

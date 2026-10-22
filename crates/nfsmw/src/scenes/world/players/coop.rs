//! A co-op session in the world: who plays, what they are called and the second car coming in
//! (`docs/specs/coop-career.md` sections 2 and 3, ADR 0010). The front end queues it; the world brings the guest in on
//! its next frame, with a renderer to load the car.

use blackbox_gfx::RenderBackend;

use super::super::{WorldScene, car_loader};
use super::tether::Tether;
use super::{LEAD, PlayerId};
use crate::settings::Transmission;

/// The second player of a co-op session, as the front end chose it.
#[derive(Debug, Clone, PartialEq)]
pub struct CoopSetup {
    /// The guest's car as `CAR[:PRESET]` (the folder name), from the shared garage.
    pub car: String,
    /// The guest's gearbox.
    pub transmission: Transmission,
    /// The active alias; the players are "<alias> 1" and "<alias> 2".
    pub alias: String,
    /// The guest car's career record handle, when it is a career car (its bounty, heat and impound).
    pub car_handle: Option<u32>,
}

/// The co-op session in play.
#[derive(Debug, Clone)]
pub struct CoopSession {
    /// Each player's name, in id order (the lead first).
    pub names: Vec<String>,
    /// The guest car's career record handle (`CoopSetup::car_handle`).
    pub guest_handle: Option<u32>,
    /// The soft catch-up distance.
    pub tether: Tether,
}

/// The guest's id: co-op seats one guest after the lead.
pub const GUEST: PlayerId = PlayerId(1);

/// How far behind the lead a joining guest starts, in metres.
const JOIN_BEHIND: f32 = 12.0;

impl WorldScene {
    /// Start co-op on the next frame with `setup`'s player 2 (the front end's "Co-op Career").
    pub fn queue_coop(&mut self, setup: CoopSetup) {
        self.queued_coop = Some(setup);
    }

    /// Whether a co-op session is on.
    pub fn coop_active(&self) -> bool {
        self.coop.is_some()
    }

    /// A player's name in this session: "<alias> N" in co-op, `None` outside it.
    pub(in crate::scenes::world) fn player_name(&self, id: PlayerId) -> Option<&str> {
        self.coop.as_ref()?.names.get(usize::from(id.0)).map(String::as_str)
    }

    /// Bring the queued guest in behind the lead. A car that cannot load is logged and co-op does not start.
    pub(in crate::scenes::world) fn start_queued_coop(&mut self, renderer: &mut dyn RenderBackend) {
        let Some(setup) = self.queued_coop.take() else { return };
        let Some(lead) = self.players.lead() else {
            log::warn!("co-op needs the lead's car in the world first");
            return;
        };
        let (position, heading) = (lead.position(), lead.heading());
        let forward = glam::Vec3::new(heading.cos(), heading.sin(), 0.0);
        let near = super::tether::behind(position, forward, JOIN_BEHIND);
        let (folder, model) = match car_loader::load_named_car(&self.dir, &setup.car) {
            Ok(car) => car,
            Err(error) => {
                log::warn!("co-op: player 2's car {} did not load: {error:#}", setup.car);
                return;
            }
        };
        if let Err(error) = self.add_player(renderer, GUEST, folder.clone(), model, [near.x, near.y], Some(heading)) {
            log::warn!("co-op: player 2 did not join: {error:#}");
            return;
        }
        if let Some(guest) = self.players.get_mut(GUEST) {
            guest.set_transmission(setup.transmission);
        }
        let names = [LEAD, GUEST].map(|id| nfsmw_career::coop::player_name(&setup.alias, id.0 + 1)).to_vec();
        log::info!("co-op: {} joins in {folder}", names[1]);
        self.coop = Some(CoopSession { names, guest_handle: setup.car_handle, tether: Tether::default() });
        self.second_view = super::super::split::SecondView::new();
    }

    /// End co-op: player 2's car leaves the world and the screen is whole again.
    pub fn end_coop(&mut self, renderer: &mut dyn RenderBackend) {
        self.queued_coop = None;
        let Some(session) = self.coop.take() else { return };
        self.remove_player(renderer, GUEST);
        self.inputs.clear();
        log::info!("co-op ended; {} left", session.names.get(1).map_or("player 2", String::as_str));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn coop_names_both_players() {
        let names = [LEAD, GUEST].map(|id| nfsmw_career::coop::player_name("Razor", id.0 + 1));
        assert_eq!(names, ["Razor 1", "Razor 2"]);
    }
}

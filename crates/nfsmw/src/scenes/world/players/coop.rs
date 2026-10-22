//! A co-op session in the world: who plays, what they are called and the second car coming in
//! (`docs/specs/coop-career.md` sections 2 and 3, ADR 0010). The front end queues it; the world brings the guest in on
//! its next frame, with a renderer to load the car.

use blackbox_gfx::RenderBackend;

use super::super::{WorldScene, car_loader};
use super::tether::Tether;
use super::{LEAD, PlayerId};
use crate::input::CoopDevice;
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
    /// The device the guest drives with (the arrow block, or a pad).
    pub device: CoopDevice,
}

/// The co-op session in play.
#[derive(Debug, Clone)]
pub struct CoopSession {
    /// Each player's name, in id order (the lead first).
    pub names: Vec<String>,
    /// The guest car's career record handle (`CoopSetup::car_handle`).
    pub guest_handle: Option<u32>,
    /// The guest's car folder and gearbox, for the race seat it drives in a race (`race.rs`).
    pub guest_car: String,
    pub transmission: Transmission,
    /// The soft catch-up distance.
    pub tether: Tether,
    /// The players offered the "Press Reset to catch up" prompt this frame, in id order (`catch_up.rs`).
    pub catch_up: Vec<PlayerId>,
    /// The shared chase: its targets, its primary and the per-target busts (`chase.rs`).
    pub chase: super::chase::CoopChase,
    /// The guests are parked for a cutscene (`suspend.rs`).
    pub suspended: bool,
    /// The guests the cutscene parked, brought back when it ends.
    pub cutscene_parked: Vec<PlayerId>,
    /// The device the guest drives with (`CoopSetup::device`).
    pub device: CoopDevice,
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

    /// A player's name in this session: "<alias> N" in co-op, `None` outside it.
    pub(in crate::scenes::world) fn player_name(&self, id: PlayerId) -> Option<&str> {
        self.coop.as_ref()?.names.get(usize::from(id.0)).map(String::as_str)
    }

    /// Ask for co-op to end on the next frame, when `start_queued_coop` has a renderer to take player 2's car out.
    pub fn queue_end_coop(&mut self) {
        self.end_coop_queued = true;
    }

    /// Bring the queued guest in behind the lead. A car that cannot load is logged and co-op does not start. A queued
    /// end comes first, so a session ended and started in the same frame ends before the new one joins.
    pub(in crate::scenes::world) fn start_queued_coop(&mut self, renderer: &mut dyn RenderBackend) {
        if self.end_coop_queued {
            self.end_coop_queued = false;
            self.end_coop(renderer);
        }
        if self.queued_coop.is_none() {
            return;
        }
        // Player 2 starts behind player 1, so the join waits until player 1's car stands on the road.
        let Some(lead) = self.players.lead().filter(|drive| drive.has_sim() && drive.waiting_for_road().is_none())
        else {
            return;
        };
        let Some(setup) = self.queued_coop.take() else { return };
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
        self.coop = Some(CoopSession {
            names,
            guest_handle: setup.car_handle,
            guest_car: folder.clone(),
            transmission: setup.transmission,
            tether: Tether::default(),
            device: setup.device,
            catch_up: Vec::new(),
            chase: Default::default(),
            suspended: false,
            cutscene_parked: Vec::new(),
        });
        self.second_view = super::super::split::SecondView::new();
    }

    /// Whether co-op may not end now: player 2 races (its free-roam car is parked for the race) or is chased.
    pub(in crate::scenes::world) fn coop_end_locked(&self) -> bool {
        let Some(session) = self.coop.as_ref() else { return false };
        (self.race.is_some() && self.players.is_parked(GUEST)) || session.chase.is_chased(GUEST)
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

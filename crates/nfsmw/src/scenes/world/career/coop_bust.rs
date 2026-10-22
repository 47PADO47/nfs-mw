//! The bust of a co-op player, per car (`docs/specs/coop-career.md` section 5.2, ADR 0010).
//!
//! A target busted by its own meter while the chase goes on records the bust on its own car, pays the fine in cash
//! and goes to the safe house; the chase goes on for the others ([`WorldScene::coop_bust`]). The last target busted is
//! busted the normal way, but co-op has no arrest scene and no ticket screen: [`WorldScene::coop_auto_pay_ticket`]
//! pays the ticket through the police and settles that car. The chase's own end (the bounty, the evade) is in
//! `pursuit.rs`.

use blackbox_pursuit::EndReason;
use nfsmw_career::coop::pursuit::settle_target_bust;
use nfsmw_career::economy::{InfractionMask, PursuitResult};

use super::bust::{pursuit_mask, settle_bust};
use super::pursuit::{infraction_mask, pursuit_result};
use crate::scenes::world::WorldScene;
use crate::scenes::world::players::coop::GUEST;
use crate::scenes::world::players::{LEAD, PlayerId};

impl WorldScene {
    /// Player `id` was busted by its own meter while the chase goes on: its car takes the bust and pays the fine in
    /// cash, and the player goes to the safe house.
    pub(in crate::scenes::world) fn coop_bust(&mut self, id: PlayerId) {
        log::info!("co-op: player {} is busted; the chase goes on", id.0 + 1);
        let Some(result) = self.coop_chase_result() else {
            return;
        };
        self.coop_record_bust(id, &result);
        self.coop_pay_fine(id, result.infractions);
        self.coop_to_safe_house(id);
    }

    /// Co-op has no ticket screen. When the police's bust is ready, the busted car pays its fine in cash through the
    /// police (`pay_bust`, as a cash ticket does) and the player goes to the safe house. The busted player is the
    /// chase's last primary, which the police followed to the end (`CoopChase::last_primary`). Does nothing outside
    /// co-op.
    pub(in crate::scenes::world) fn coop_auto_pay_ticket(&mut self) {
        let Some(session) = self.coop.as_ref() else {
            return;
        };
        let busted = session.chase.last_primary;
        let Some(traffic) = self.traffic.as_mut() else {
            return;
        };
        if traffic.take_bust_ticket().is_none() {
            return;
        }
        let pursuit = pursuit_mask(&traffic.bust_infractions());
        if !traffic.pay_bust(false, 0) {
            log::warn!("co-op: the police refused the cash ticket of player {}", busted.0 + 1);
            return;
        }
        self.coop_pay_fine(busted, pursuit);
        self.coop_to_safe_house(busted);
    }

    /// The chase so far as one car's bust sees it: its infractions and rep points. `None` without a chase.
    fn coop_chase_result(&self) -> Option<PursuitResult> {
        let traffic = self.traffic.as_ref()?;
        let mask = infraction_mask(traffic.infractions());
        Some(pursuit_result(EndReason::Busted, mask, traffic.rep_normal_points(), traffic.rep_destroyed_points()))
    }

    /// Records the bust on the car of player `id` (`settle_target_bust`), for a target busted mid-chase.
    fn coop_record_bust(&mut self, id: PlayerId, result: &PursuitResult) {
        let Some(handle) = self.car_handle_of(id) else {
            log::warn!("co-op: player {} has no career car, so no bust is recorded", id.0 + 1);
            return;
        };
        self.career.handle.with(|career| settle_target_bust(&mut career.stable, handle, result));
    }

    /// Pays the fine of the car of player `id` in cash from the shared wallet (`settle_bust` with no marker). A car
    /// with no career record has no fine.
    fn coop_pay_fine(&mut self, id: PlayerId, pursuit: InfractionMask) {
        let Some(handle) = self.car_handle_of(id) else {
            log::warn!("co-op: player {} has no career car, so no fine is paid", id.0 + 1);
            return;
        };
        let outcome = self.career.handle.with(|career| settle_bust(career, handle, pursuit, false)).flatten();
        log::info!("co-op: player {} paid the bust in cash, outcome {outcome:?}", id.0 + 1);
    }

    /// Sends the busted player to the safe house. The lead's car goes to the respawn marker when the game runs again
    /// (`respawn.rs`). A guest is put on the road at the marker now, as the lead is by its own respawn; with no safe
    /// house visited, the car stays where it is.
    fn coop_to_safe_house(&mut self, id: PlayerId) {
        if id == LEAD {
            self.career.respawn.pending = true;
            return;
        }
        let Some(at) = self.career.respawn.target() else {
            log::info!("co-op: no safe house visited, player {} stays where it is", id.0 + 1);
            return;
        };
        let Some(guest) = self.players.get_mut(id) else {
            return;
        };
        guest.respawn_near([at.x, at.y], None);
        log::info!("co-op: player {} goes back to the safe house at ({:.0}, {:.0})", id.0 + 1, at.x, at.y);
    }

    /// The career car of co-op player `id`: the current car for the lead, the guest's record for the guest. `None`
    /// for a guest with no career car, and for any player when no career is played.
    fn car_handle_of(&self, id: PlayerId) -> Option<u32> {
        if id == GUEST {
            return self.coop.as_ref()?.guest_handle;
        }
        if id != LEAD {
            return None;
        }
        self.career.handle.with(|career| career.settings.car_handle)
    }
}

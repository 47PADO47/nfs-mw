//! A member's own choices: ready, car and role. Each is a lobby action, rate-limited per member.

use blackbox_net_core::session::Phase;

use super::roster::peer;
use super::{Action, Lobby, LobbyError};
use crate::loadout::{Catalog, Loadout, validate};
use crate::message::{CancelReason, Mode, Role};
use crate::spectate::{SpectateReason, may_spectate};

impl Lobby {
    /// `SetReady` from `slot`. Only a racer or cop with a valid car, in the lobby. Un-readying during the start
    /// countdown cancels it; the last racer getting ready starts it when [`super::LobbyConfig::auto_start`] is on.
    pub fn set_ready(&mut self, slot: u8, ready: bool, now_ms: u64, tick: u32) -> Result<Vec<Action>, LobbyError> {
        self.lobby_action(slot, now_ms)?;
        if self.session.phase() != Phase::Lobby {
            return Err(LobbyError::WrongPhase);
        }
        let member = self.members.get_mut(&slot).ok_or(LobbyError::UnknownSlot)?;
        if !member.is_player() {
            return Err(LobbyError::Spectator);
        }
        if ready && member.loadout.is_none() {
            return Err(LobbyError::NoLoadout);
        }
        if member.ready == ready {
            return Ok(Vec::new());
        }
        member.ready = ready;
        let mut actions = vec![self.update_row(slot)];
        if !ready {
            actions.extend(self.cancel_countdown(CancelReason::NotReady));
            return Ok(actions);
        }
        if self.config.auto_start && self.can_start().is_ok() {
            actions.extend(self.begin_countdown(tick));
        }
        Ok(actions)
    }

    /// `SelectLoadout` from `slot`, in the lobby or on the results screen. The car is checked against `catalog` and
    /// the session's restriction (MP-31); a refusal keeps the old car. A new car un-readies the member.
    pub fn select_loadout(
        &mut self,
        slot: u8,
        loadout: Loadout,
        catalog: &impl Catalog,
        now_ms: u64,
    ) -> Result<Vec<Action>, LobbyError> {
        self.lobby_action(slot, now_ms)?;
        if !matches!(self.session.phase(), Phase::Lobby | Phase::Results) {
            return Err(LobbyError::WrongPhase);
        }
        validate(&loadout, catalog, &self.settings.allowed_cars).map_err(LobbyError::Loadout)?;
        let member = self.members.get_mut(&slot).ok_or(LobbyError::UnknownSlot)?;
        let was_ready = member.ready;
        member.loadout = Some(loadout);
        member.ready = false;
        let mut actions = vec![self.update_row(slot)];
        if was_ready {
            actions.extend(self.cancel_countdown(CancelReason::NotReady));
        }
        Ok(actions)
    }

    /// `SetRole` from `slot`. Watching is allowed any time there is a spectator place. Driving again, or switching
    /// between racer and cop, only in the lobby; outside it the member is marked to race next. Cops only in cops
    /// against racers; a spectate-only client never drives.
    pub fn set_role(&mut self, slot: u8, role: Role, now_ms: u64) -> Result<Vec<Action>, LobbyError> {
        self.lobby_action(slot, now_ms)?;
        let member = self.members.get(&slot).ok_or(LobbyError::UnknownSlot)?;
        if member.role == role {
            return Ok(Vec::new());
        }
        if role == Role::Spectator {
            return self.make_spectator(slot, SpectateReason::Chosen);
        }
        if member.spectate_only {
            return Err(LobbyError::SpectateOnly);
        }
        if role == Role::Cop && self.settings.mode != Mode::CopsVsRacers {
            return Err(LobbyError::RoleNotOffered);
        }
        let was_player = member.is_player();
        if self.session.phase() != Phase::Lobby {
            return self.want_role(slot, role, was_player);
        }
        if !was_player && self.racers() >= usize::from(self.settings.max_players) {
            return Err(LobbyError::Full);
        }
        if !was_player {
            self.session.join(peer(slot)).map_err(LobbyError::Session)?;
        }
        let member = self.members.get_mut(&slot).ok_or(LobbyError::UnknownSlot)?;
        member.role = role;
        member.wants = role;
        member.ready = false;
        Ok(vec![Action::RoleChanged { slot, role }, self.update_row(slot)])
    }

    /// Outside the lobby a spectator can only ask to drive next time; a driver cannot change sides mid-race.
    fn want_role(&mut self, slot: u8, role: Role, was_player: bool) -> Result<Vec<Action>, LobbyError> {
        if was_player {
            return Err(LobbyError::WrongPhase);
        }
        let member = self.members.get_mut(&slot).ok_or(LobbyError::UnknownSlot)?;
        member.wants = role;
        Ok(vec![self.update_row(slot)])
    }

    /// The per-member lobby action bucket.
    pub(super) fn lobby_action(&mut self, slot: u8, now_ms: u64) -> Result<(), LobbyError> {
        let member = self.members.get_mut(&slot).ok_or(LobbyError::UnknownSlot)?;
        if !member.rate.allow(now_ms) {
            return Err(LobbyError::RateLimited);
        }
        Ok(())
    }

    /// Whether a spectator place is free for a member choosing to watch.
    pub fn can_spectate(&self) -> bool {
        may_spectate(&self.settings, self.spectators(), SpectateReason::Chosen).is_ok()
    }
}

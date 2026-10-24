//! Settings changes, host and admin commands (MP-26), and carrying out a passed vote (MP-27).

use blackbox_net_core::session::Phase;

use super::{Action, Lobby, LobbyError};
use crate::chat::notice;
use crate::loadout::{Catalog, validate};
use crate::message::{
    AdminCommand, CancelReason, Kick, KickReason, LeaveReason, Message, Mode, SettingsPatch, VoteKind,
};
use crate::roles::{Command, Rank, may_target, permits};
use crate::settings::{Field, PatchContext, SettingsError, apply_patch, has_laps};

impl Lobby {
    /// `UpdateSettings` from `slot` (host or admin; a lobby action). Cars the new settings refuse are dropped and their
    /// members un-readied, which cancels a countdown. Everyone gets the whole lobby again.
    pub fn update_settings(
        &mut self,
        slot: u8,
        patch: &SettingsPatch,
        catalog: &impl Catalog,
        now_ms: u64,
    ) -> Result<Vec<Action>, LobbyError> {
        let rank = self.members.get(&slot).ok_or(LobbyError::UnknownSlot)?.rank;
        self.lobby_action(slot, now_ms)?;
        self.patch(rank, patch, catalog)
    }

    /// `HostCommand` from `slot`, checked against its rank and the target's.
    pub fn command(
        &mut self,
        slot: u8,
        command: AdminCommand,
        catalog: &impl Catalog,
        now_ms: u64,
        tick: u32,
    ) -> Result<Vec<Action>, LobbyError> {
        match command {
            AdminCommand::Kick { slot: target } => {
                self.require(slot, Command::Kick)?;
                self.check_target(slot, target)?;
                self.kick(target, KickReason::Kicked, 0)
            }
            AdminCommand::Ban { slot: target, minutes } => {
                self.require(slot, Command::Ban)?;
                self.check_target(slot, target)?;
                let player_id = self.members.get(&target).ok_or(LobbyError::UnknownSlot)?.player_id;
                self.bans.ban(player_id, minutes, now_ms);
                self.kick(target, KickReason::Banned, minutes)
            }
            AdminCommand::Mute { slot: target, muted } => {
                self.require(slot, Command::Mute)?;
                self.check_target(slot, target)?;
                let member = self.members.get_mut(&target).ok_or(LobbyError::UnknownSlot)?;
                member.muted = muted;
                let line = Message::SystemNotice(notice::muted(target, muted));
                Ok(vec![Action::Broadcast(line), self.update_row(target)])
            }
            AdminCommand::Restart => {
                self.require(slot, Command::Restart)?;
                self.restart(tick)
            }
            AdminCommand::Hold { on } => self.hold(slot, on),
            AdminCommand::ChangeMode { mode } => {
                self.require(slot, Command::ChangeMode)?;
                let rank = self.members.get(&slot).ok_or(LobbyError::UnknownSlot)?.rank;
                let patch = self.mode_patch(mode);
                self.patch(rank, &patch, catalog)
            }
        }
    }

    /// The host makes `target` an admin or a player again.
    pub fn set_admin(&mut self, slot: u8, target: u8, admin: bool) -> Result<Vec<Action>, LobbyError> {
        self.require(slot, Command::SetAdmin)?;
        let member = self.members.get_mut(&target).ok_or(LobbyError::UnknownSlot)?;
        if target == slot || member.rank == Rank::Host {
            return Err(LobbyError::BadTarget);
        }
        member.rank = match admin {
            true => Rank::Admin,
            false => Rank::Player,
        };
        Ok(vec![self.update_row(target)])
    }

    /// Carries out a passed vote: a kick (reason `Voted`), a restart, a mode or track change (with the host's rights:
    /// the electorate decided), or skipping the results.
    pub fn apply_vote(
        &mut self,
        kind: VoteKind,
        option: u32,
        catalog: &impl Catalog,
        tick: u32,
    ) -> Result<Vec<Action>, LobbyError> {
        match kind {
            VoteKind::Kick => {
                let target = u8::try_from(option).map_err(|_| LobbyError::UnknownSlot)?;
                self.kick(target, KickReason::Voted, 0)
            }
            VoteKind::Restart => self.restart(tick),
            VoteKind::ChangeMode => {
                let invalid = LobbyError::Settings(SettingsError::Invalid(Field::Mode));
                let mode = u8::try_from(option).ok().and_then(Mode::from_code).ok_or(invalid)?;
                let patch = self.mode_patch(mode);
                self.patch(Rank::Host, &patch, catalog)
            }
            VoteKind::ChangeTrack => {
                let patch = SettingsPatch { track: Some(option), ..SettingsPatch::default() };
                self.patch(Rank::Host, &patch, catalog)
            }
            VoteKind::SkipResults => {
                if self.session.phase() != Phase::Results {
                    return Err(LobbyError::WrongPhase);
                }
                self.back_to_lobby()
            }
        }
    }

    /// A moderation target: another member of a lower rank.
    fn check_target(&self, slot: u8, target: u8) -> Result<(), LobbyError> {
        let actor = self.members.get(&slot).ok_or(LobbyError::UnknownSlot)?.rank;
        let member = self.members.get(&target).ok_or(LobbyError::UnknownSlot)?;
        if target == slot || !may_target(actor, member.rank) {
            return Err(LobbyError::BadTarget);
        }
        Ok(())
    }

    /// Kicks `target`: the `Kick` message and close, the kick notice, then the removal.
    fn kick(&mut self, target: u8, reason: KickReason, ban_minutes: u32) -> Result<Vec<Action>, LobbyError> {
        if !self.members.contains_key(&target) {
            return Err(LobbyError::UnknownSlot);
        }
        let banned = reason == KickReason::Banned;
        let leave = match banned {
            true => LeaveReason::Banned,
            false => LeaveReason::Kicked,
        };
        let line = notice::kicked(target, banned.then_some(ban_minutes));
        let mut actions = vec![
            Action::Kick { slot: target, kick: Kick { reason, ban_minutes } },
            Action::Broadcast(Message::SystemNotice(line)),
        ];
        actions.extend(self.remove(target, leave)?);
        Ok(actions)
    }

    /// A patch that changes the mode, with one lap when a lap mode would otherwise have none.
    fn mode_patch(&self, mode: Mode) -> SettingsPatch {
        let laps = (has_laps(mode) && self.settings.laps == 0).then_some(1);
        SettingsPatch { mode: Some(mode), laps, ..SettingsPatch::default() }
    }

    /// Applies a patch for a sender of `rank` and checks every car again.
    fn patch(&mut self, rank: Rank, patch: &SettingsPatch, catalog: &impl Catalog) -> Result<Vec<Action>, LobbyError> {
        if !permits(rank, Command::ChangeSettings) {
            return Err(LobbyError::NotPermitted);
        }
        let context = PatchContext {
            rank,
            phase: self.wire_phase(),
            racers: self.racers(),
            spectators: self.spectators(),
            has_password: self.has_password,
        };
        let (next, changed) = apply_patch(&self.settings, patch, &context).map_err(LobbyError::Settings)?;
        if changed.is_empty() {
            return Ok(Vec::new());
        }
        self.settings = next;
        let mut unreadied = false;
        for member in self.members.values_mut() {
            let refused = member
                .loadout
                .as_ref()
                .is_some_and(|loadout| validate(loadout, catalog, &self.settings.allowed_cars).is_err());
            if !refused {
                continue;
            }
            unreadied |= member.ready;
            member.loadout = None;
            member.ready = false;
        }
        let mut actions = Vec::new();
        if unreadied {
            actions.extend(self.cancel_countdown(CancelReason::NotReady));
        }
        actions.push(Action::Broadcast(Message::LobbyState(self.state())));
        Ok(actions)
    }
}

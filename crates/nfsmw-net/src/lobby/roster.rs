//! The lobby's state, its read access, and members joining, leaving, dropping and becoming spectators.

use std::collections::BTreeMap;

use blackbox_net_core::session::{PeerId, Phase, Session};

use super::member::{Member, NewMember};
use super::{Action, LobbyConfig, LobbyError};
use crate::chat::notice;
use crate::drop::{LoadoutRow, SessionData};
use crate::join::Placement;
use crate::message::{
    CancelReason, LeaveReason, List, LobbyChange, LobbyDelta, LobbyState, MAX_SLOTS, Message, Role, SessionPhase,
    Settings,
};
use crate::roles::{Bans, Rank};
use crate::settings::{SettingsError, validate};
use crate::spectate::{SpectateReason, may_spectate};

/// The lobby and session state of one server.
#[derive(Debug, Clone)]
pub struct Lobby {
    pub(super) config: LobbyConfig,
    pub(super) settings: Settings,
    pub(super) session: Session,
    pub(super) members: BTreeMap<u8, Member>,
    /// The tick at which the start countdown ends and loading begins.
    pub(super) countdown: Option<u32>,
    /// The slot that holds everyone.
    pub(super) hold: Option<u8>,
    pub(super) has_password: bool,
    pub(super) bans: Bans,
}

impl Lobby {
    /// An empty lobby with `settings`, which must pass [`validate`]. `has_password` says whether the server holds a
    /// room password; `password_required` without one is [`SettingsError::NoPassword`].
    pub fn new(config: LobbyConfig, settings: Settings, has_password: bool) -> Result<Self, SettingsError> {
        validate(&settings)?;
        if settings.password_required && !has_password {
            return Err(SettingsError::NoPassword);
        }
        Ok(Self {
            config,
            settings,
            session: Session::new(config.session),
            members: BTreeMap::new(),
            countdown: None,
            hold: None,
            has_password,
            bans: Bans::new(),
        })
    }

    pub fn settings(&self) -> &Settings {
        &self.settings
    }

    /// The phase machine's phase.
    pub fn phase(&self) -> Phase {
        self.session.phase()
    }

    /// The phase as the wire names it.
    pub fn wire_phase(&self) -> SessionPhase {
        self.session.phase().into()
    }

    pub fn member(&self, slot: u8) -> Option<&Member> {
        self.members.get(&slot)
    }

    /// Every member in slot order.
    pub fn members(&self) -> impl Iterator<Item = &Member> {
        self.members.values()
    }

    /// Every member slot in order.
    pub fn slots(&self) -> Vec<u8> {
        self.members.keys().copied().collect()
    }

    /// Racers and cops, held slots included.
    pub fn racers(&self) -> usize {
        self.members.values().filter(|member| member.is_player()).count()
    }

    /// Spectators, counted apart from the racers (MP-24).
    pub fn spectators(&self) -> usize {
        self.members.len() - self.racers()
    }

    /// The host's slot, if a member is the host.
    pub fn host(&self) -> Option<u8> {
        self.members.values().find(|member| member.rank == Rank::Host).map(|member| member.slot)
    }

    /// Whether the server muted `slot`'s chat.
    pub fn is_muted(&self, slot: u8) -> bool {
        self.members.get(&slot).is_some_and(|member| member.muted)
    }

    /// The slots that may vote: racers and cops who are connected.
    pub fn voters(&self) -> Vec<u8> {
        self.members.values().filter(|member| member.is_player() && !member.away).map(|member| member.slot).collect()
    }

    /// Slots no vote may kick: the host.
    pub fn protected(&self) -> Vec<u8> {
        self.host().into_iter().collect()
    }

    /// The racers in slot order: the spectator camera's targets.
    pub fn racer_slots(&self) -> Vec<u8> {
        self.members.values().filter(|member| member.is_player()).map(|member| member.slot).collect()
    }

    pub fn bans(&self) -> &Bans {
        &self.bans
    }

    /// The tick the start countdown ends at, while it runs.
    pub fn countdown(&self) -> Option<u32> {
        self.countdown
    }

    /// Whether "everyone wait" is on.
    pub fn is_held(&self) -> bool {
        self.hold.is_some()
    }

    /// The whole lobby, for a joiner or after a resume.
    pub fn state(&self) -> LobbyState {
        let rows = self.members.values().map(|member| member.row(self.hold == Some(member.slot))).collect();
        LobbyState {
            players: List::new(rows).unwrap_or_default(),
            settings: self.settings.clone(),
            phase: self.wire_phase(),
            host_slot: self.host().unwrap_or(0),
        }
    }

    /// The late-join session data (MP-53): the lobby and every member's car.
    pub fn session_data(&self) -> SessionData {
        let rows = self
            .members
            .values()
            .filter_map(|member| member.loadout.clone().map(|loadout| LoadoutRow { slot: member.slot, loadout }))
            .collect();
        SessionData { lobby: self.state(), loadouts: List::new(rows).unwrap_or_default() }
    }

    /// Seats an admitted joiner in the lowest free slot. A racer joins the phase machine (in the lobby, or straight
    /// into a running free roam with a [`Action::Spawn`]); a spectator is checked against the spectator limit.
    pub fn add(&mut self, new: NewMember, now_ms: u64) -> Result<(u8, Vec<Action>), LobbyError> {
        let slot = (0..MAX_SLOTS as u8).find(|slot| !self.members.contains_key(slot)).ok_or(LobbyError::Full)?;
        let racer = new.placement == Placement::Racer;
        if racer && self.racers() >= usize::from(self.settings.max_players) {
            return Err(LobbyError::Full);
        }
        if let Placement::Spectator(reason) = new.placement {
            may_spectate(&self.settings, self.spectators(), reason).map_err(|_| LobbyError::Full)?;
        }
        if racer {
            self.session.join(peer(slot)).map_err(LobbyError::Session)?;
        }
        let member = Member::new(slot, new, now_ms);
        let mut actions = vec![
            Action::Broadcast(delta(LobbyChange::Add(member.row(false)))),
            Action::Joined { slot },
            Action::Broadcast(Message::SystemNotice(notice::joined(slot))),
        ];
        self.members.insert(slot, member);
        actions.push(Action::Send { to: slot, message: Message::LobbyState(self.state()) });
        if racer && self.session.phase() == Phase::Running {
            actions.push(Action::Spawn { slot });
        }
        Ok((slot, actions))
    }

    /// Removes a member for `reason`. The leave notice comes from [`crate::drop::on_drop`] and the kick notice from
    /// the kick command, so this sends none. A racer leaving cancels a running start countdown; the host leaving ends
    /// the session.
    pub fn remove(&mut self, slot: u8, reason: LeaveReason) -> Result<Vec<Action>, LobbyError> {
        let member = self.members.remove(&slot).ok_or(LobbyError::UnknownSlot)?;
        let mut actions = Vec::new();
        if member.is_player() {
            let _ = self.session.leave(peer(slot));
            actions.push(Action::Despawn { slot });
            actions.extend(self.cancel_countdown(CancelReason::PlayerLeft));
        }
        if self.hold == Some(slot) {
            self.hold = None;
            actions.push(Action::Hold { on: false });
        }
        actions.push(Action::Broadcast(delta(LobbyChange::Remove { slot })));
        actions.push(Action::Left { slot, reason });
        if member.rank == Rank::Host {
            actions.push(Action::EndSession);
        }
        Ok(actions)
    }

    /// Marks a member dropped (its slot held for a resume, [`super::FLAG_AWAY`]) or back.
    pub fn set_away(&mut self, slot: u8, away: bool) -> Result<Vec<Action>, LobbyError> {
        let member = self.members.get_mut(&slot).ok_or(LobbyError::UnknownSlot)?;
        member.away = away;
        member.ready &= !away;
        let mut actions = Vec::new();
        if away && member.is_player() {
            actions.extend(self.cancel_countdown(CancelReason::PlayerLeft));
        }
        actions.push(self.update_row(slot));
        Ok(actions)
    }

    /// Records a member's round-trip time for its row; it goes out with the row's next update.
    pub fn set_ping(&mut self, slot: u8, ping_ms: u16) {
        if let Some(member) = self.members.get_mut(&slot) {
            member.ping_ms = ping_ms;
        }
    }

    /// Makes a member a spectator: `Eliminated` (knockout, busted out; outside the spectator limit, races again next
    /// time) or `Chosen` (checked against the limit). A driver leaves the phase machine and its car is removed.
    pub fn make_spectator(&mut self, slot: u8, reason: SpectateReason) -> Result<Vec<Action>, LobbyError> {
        let spectators = self.spectators();
        let member = self.members.get(&slot).ok_or(LobbyError::UnknownSlot)?;
        if !member.is_player() {
            return Ok(Vec::new());
        }
        may_spectate(&self.settings, spectators, reason).map_err(|_| LobbyError::Full)?;
        let _ = self.session.leave(peer(slot));
        let member = self.members.get_mut(&slot).ok_or(LobbyError::UnknownSlot)?;
        member.wants = match reason {
            SpectateReason::Chosen => Role::Spectator,
            SpectateReason::WaitingForRace | SpectateReason::Eliminated => member.role,
        };
        member.role = Role::Spectator;
        member.ready = false;
        member.loading = false;
        let mut actions = vec![Action::Despawn { slot }, Action::RoleChanged { slot, role: Role::Spectator }];
        actions.extend(self.cancel_countdown(CancelReason::PlayerLeft));
        actions.push(self.update_row(slot));
        Ok(actions)
    }

    /// The `LobbyDelta` that sends `slot`'s row again.
    pub(super) fn update_row(&self, slot: u8) -> Action {
        let change = match self.members.get(&slot) {
            Some(member) => LobbyChange::Update(member.row(self.hold == Some(slot))),
            None => LobbyChange::Remove { slot },
        };
        Action::Broadcast(delta(change))
    }

    /// Stops a running start countdown, telling everyone why. Empty when none runs.
    pub(super) fn cancel_countdown(&mut self, reason: CancelReason) -> Vec<Action> {
        if self.countdown.take().is_none() {
            return Vec::new();
        }
        vec![Action::Broadcast(Message::CancelCountdown(crate::message::CancelCountdown { reason }))]
    }
}

/// The phase machine's id of a slot.
pub(super) fn peer(slot: u8) -> PeerId {
    PeerId(u16::from(slot))
}

/// A one-change `LobbyDelta`.
pub(super) fn delta(change: LobbyChange) -> Message {
    Message::LobbyDelta(LobbyDelta { changes: List::new(vec![change]).unwrap_or_default() })
}

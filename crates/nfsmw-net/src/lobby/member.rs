//! One lobby member and its roster row.

use super::rate::{LOBBY_RATE, RateLimit};
use super::{FLAG_ADMIN, FLAG_AWAY, FLAG_HOLD, FLAG_HOST, FLAG_LOADING, FLAG_MUTED, FLAG_WAITING};
use crate::join::Placement;
use crate::loadout::Loadout;
use crate::message::{LobbyPlayer, Name, Role};
use crate::roles::Rank;
use crate::spectate::SpectateReason;

/// A joiner the admission let in.
#[derive(Debug, Clone, PartialEq)]
pub struct NewMember {
    pub player_id: [u8; 16],
    pub name: Name,
    pub rank: Rank,
    pub placement: Placement,
    /// The client is spectate-only (`CAP_SPECTATE_ONLY`).
    pub spectate_only: bool,
    /// The car of the `Join` message, validated by the admission.
    pub loadout: Loadout,
}

/// A member of the session.
#[derive(Debug, Clone, PartialEq)]
pub struct Member {
    pub slot: u8,
    pub player_id: [u8; 16],
    pub name: Name,
    pub rank: Rank,
    pub role: Role,
    /// The role the member takes at the next lobby: a spectator waiting for a race wants to race.
    pub wants: Role,
    pub spectate_only: bool,
    pub ready: bool,
    /// The car, or `None` once a settings change made it invalid.
    pub loadout: Option<Loadout>,
    pub muted: bool,
    pub team: u8,
    pub ping_ms: u16,
    pub loading: bool,
    /// Dropped, slot held for a resume.
    pub away: bool,
    pub(super) rate: RateLimit,
}

impl Member {
    pub(super) fn new(slot: u8, new: NewMember, now_ms: u64) -> Self {
        let (role, wants) = match new.placement {
            Placement::Racer => (Role::Racer, Role::Racer),
            Placement::Spectator(SpectateReason::Chosen) => (Role::Spectator, Role::Spectator),
            Placement::Spectator(_) => (Role::Spectator, Role::Racer),
        };
        Self {
            slot,
            player_id: new.player_id,
            name: new.name,
            rank: new.rank,
            role,
            wants,
            spectate_only: new.spectate_only,
            ready: false,
            loadout: Some(new.loadout),
            muted: false,
            team: 0,
            ping_ms: 0,
            loading: false,
            away: false,
            rate: RateLimit::new(LOBBY_RATE, now_ms),
        }
    }

    /// Whether the member drives (a racer or a cop): a peer of the phase machine.
    pub fn is_player(&self) -> bool {
        self.role != Role::Spectator
    }

    /// The roster row. `holding` sets [`FLAG_HOLD`].
    pub fn row(&self, holding: bool) -> LobbyPlayer {
        let flags = [
            (self.rank == Rank::Host, FLAG_HOST),
            (self.rank == Rank::Admin, FLAG_ADMIN),
            (self.muted, FLAG_MUTED),
            (self.loading, FLAG_LOADING),
            (self.role == Role::Spectator && self.wants != Role::Spectator, FLAG_WAITING),
            (holding, FLAG_HOLD),
            (self.away, FLAG_AWAY),
        ]
        .iter()
        .filter(|(on, _)| *on)
        .fold(0, |flags, (_, bit)| flags | bit);
        LobbyPlayer {
            slot: self.slot,
            player_id: self.player_id,
            name: self.name.clone(),
            car_type: self.loadout.as_ref().map_or(0, |loadout| loadout.car_type),
            ready: self.ready,
            role: self.role,
            team: self.team,
            ping_ms: self.ping_ms,
            flags,
        }
    }
}

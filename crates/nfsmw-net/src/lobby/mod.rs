//! The lobby and session state machine (MP-21, MP-22, MP-24, MP-26, MP-30, MP-31, MP-54).
//!
//! A [`Lobby`] holds the members (one per slot of the 32-slot id space), the settings, the "everyone wait" hold, the
//! start countdown and the session's bans, and drives the generic phase machine of `blackbox_net_core::session`
//! (`Lobby`, `Loading`, `WaitLoads`, `Intro`, `Countdown { start_tick }`, `Running`, `Results`). Racers and cops are
//! the machine's peers (`PeerId(slot)`); spectators are members only, so a slow spectator never holds the load barrier.
//!
//! Every call takes the time from the caller (`now_ms`, the server `tick`), checks the sender, changes the state and
//! returns the [`Action`]s to carry out, or a [`LobbyError`] that changes nothing. [`LobbyError::is_violation`] marks
//! what a normal client never sends (a player's host command), which the caller counts as a strike.
//!
//! The flow of a race:
//!
//! 1. Members choose a car ([`Lobby::select_loadout`], checked by [`crate::loadout::validate`] against the session's
//!    restriction), a role, and get ready. Lobby actions are rate-limited per member ([`LOBBY_RATE`]).
//! 2. The host or an admin starts ([`Lobby::start`]) when every racer is ready, or the lobby starts by itself when
//!    [`LobbyConfig::auto_start`] is on. The start countdown (`StartCountdown`, [`LobbyConfig::countdown_seconds`])
//!    runs to a fixed tick; a racer who leaves or un-readies, the host, or a hold cancels it (`CancelCountdown`).
//! 3. At that tick every member gets `LoadWorld` with its grid slot; loaded racers report `LoadComplete`, the server
//!    reports its own load ([`Lobby::server_loaded`]). A racer still loading at the barrier's deadline is moved to the
//!    spectators and told `CancelCountdown(LoadTimeout)`.
//! 4. The machine fixes the race start tick and the lobby broadcasts `StartAt(start_tick)`.
//! 5. [`Lobby::finish`] goes to the results; [`Lobby::back_to_lobby`] returns, un-readies everyone and seats the
//!    spectators who waited for a race.
//!
//! Row flags of `LobbyPlayer.flags` (the contract with the client): [`FLAG_HOST`], [`FLAG_ADMIN`], [`FLAG_MUTED`],
//! [`FLAG_LOADING`], [`FLAG_WAITING`] (a spectator who races next), [`FLAG_HOLD`] (this member holds everyone),
//! [`FLAG_AWAY`] (dropped, slot held for a resume). `host_slot` of `LobbyState` is the host's slot, or 0 on a
//! dedicated server without a host (no row then has `FLAG_HOST`).

mod admin;
mod choices;
mod member;
mod rate;
mod roster;
mod start;

#[cfg(test)]
mod tests;

pub use member::{Member, NewMember};
pub use rate::{CHAT_RATE, JOIN_RATE, LOBBY_RATE, RateLimit, RateSpec, VOTE_START_RATE};
pub use roster::Lobby;

use blackbox_net_core::session::{SessionConfig, SessionError};

use crate::loadout::LoadoutReject;
use crate::message::{Kick, LeaveReason, Message, Role, SessionPhase};
use crate::settings::SettingsError;

/// Row flag: the member is the host.
pub const FLAG_HOST: u8 = 1 << 0;
/// Row flag: the member is an admin.
pub const FLAG_ADMIN: u8 = 1 << 1;
/// Row flag: the member's chat is muted by the server.
pub const FLAG_MUTED: u8 = 1 << 2;
/// Row flag: the member is loading the world.
pub const FLAG_LOADING: u8 = 1 << 3;
/// Row flag: a spectator who joins the next race.
pub const FLAG_WAITING: u8 = 1 << 4;
/// Row flag: the member holds everyone ("everyone wait", MP-54).
pub const FLAG_HOLD: u8 = 1 << 5;
/// Row flag: the member dropped and its slot is held for a resume.
pub const FLAG_AWAY: u8 = 1 << 6;

/// `LoadWorld.spawn_slot` of a spectator: no grid slot.
pub const NO_SPAWN_SLOT: u8 = u8::MAX;

/// The lobby's timing and start rules.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LobbyConfig {
    /// The phase machine's load deadline, intro and race countdown.
    pub session: SessionConfig,
    /// Seconds from the start to loading (`StartCountdown.seconds`). Default 5.
    pub countdown_seconds: u8,
    /// Start the countdown by itself once every racer is ready. Default off.
    pub auto_start: bool,
    /// Fewest racers to start. Default 1 (free roam alone is allowed).
    pub min_racers: u8,
    /// `LoadWorld.time_of_day`, minutes after midnight. Default noon.
    pub time_of_day: u16,
    /// `LoadWorld.weather`. Default 0.
    pub weather: u8,
}

impl Default for LobbyConfig {
    fn default() -> Self {
        Self {
            session: SessionConfig::default(),
            countdown_seconds: 5,
            auto_start: false,
            min_racers: 1,
            time_of_day: 12 * 60,
            weather: 0,
        }
    }
}

/// One thing the server does because of a lobby call, in the order given.
#[derive(Debug, Clone, PartialEq)]
pub enum Action {
    /// Send a reliable message to one member.
    Send { to: u8, message: Message },
    /// Send a reliable message to every member.
    Broadcast(Message),
    /// Send the `PlayerJoined` event (the caller adds the event head).
    Joined { slot: u8 },
    /// Send the `PlayerLeft` event.
    Left { slot: u8, reason: LeaveReason },
    /// Send the `RoleChanged` event.
    RoleChanged { slot: u8, role: Role },
    /// Send the `PhaseChange` event; `target_tick` is the race start for the countdown, else 0.
    Phase { phase: SessionPhase, target_tick: u32 },
    /// Send `kick` to the member, then close its connection.
    Kick { slot: u8, kick: Kick },
    /// Put the member's car into the running world (a join in progress; `Respawn` with cause `JoinInProgress`).
    Spawn { slot: u8 },
    /// Take the member's car out of the world, if it has one.
    Despawn { slot: u8 },
    /// "Everyone wait" turned on or off.
    Hold { on: bool },
    /// The host of a listen server left: the session ends (MP-52, no host migration in v1).
    EndSession,
}

/// Why a lobby call was refused. A refused call changes nothing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LobbyError {
    /// No member in that slot.
    UnknownSlot,
    /// The sender's rank may not do this (a spoofed host command).
    NotPermitted,
    /// A moderation command against oneself, the host, or an equal rank.
    BadTarget,
    /// Not in this phase.
    WrongPhase,
    /// Over the lobby action rate.
    RateLimited,
    /// No racer, spectator or slot place left.
    Full,
    /// Some racer is not ready.
    NotAllReady,
    /// Fewer racers than [`LobbyConfig::min_racers`].
    TooFewRacers,
    /// Ready without a valid car.
    NoLoadout,
    /// A spectator cannot get ready.
    Spectator,
    /// The car was refused.
    Loadout(LoadoutReject),
    /// The settings change was refused.
    Settings(SettingsError),
    /// Everyone is held.
    Held,
    /// A countdown is running already.
    CountdownRunning,
    /// No countdown to cancel.
    NoCountdown,
    /// The role is not offered in this mode (cops outside cops against racers).
    RoleNotOffered,
    /// A spectate-only client asked to drive.
    SpectateOnly,
    /// The phase machine refused the step.
    Session(SessionError),
}

impl LobbyError {
    /// Whether a normal client never causes this: count it as a strike (`security-gameplay.md` 5.2).
    pub fn is_violation(self) -> bool {
        matches!(self, Self::NotPermitted | Self::BadTarget | Self::Settings(SettingsError::NotPermitted))
    }
}

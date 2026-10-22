//! What the LAN screens, the session HUD and the `net.*` console commands may read and do: the [`NetFrontend`] trait.
//! The game's `NetSession` (`crate::net`) implements it over the net thread and a `ClientSession`; the tests use
//! [`super::mock::MockNetFrontend`]. Everything is in terms of `nfsmw-net` types, so the screens and the session agree
//! on the lobby, the settings and the events without a translation layer.
//!
//! The trait never blocks: a call that starts something (scan, host, connect) returns at once, and its progress shows in
//! [`NetFrontend::status`] and [`NetFrontend::take_events`] on later frames.

use std::net::SocketAddr;

use blackbox_net_core::discovery::{self, Reply};
use nfsmw_net::client_session::{EndReason, LobbyView, SessionEvent};
use nfsmw_net::loadout::Loadout;
use nfsmw_net::message::{AdminCommand, LimitError, Mode, RaceResults, Role, Settings, SettingsPatch, VoteKind};
use nfsmw_net::netsim::NetSim;
use nfsmw_net::netstats::NetLine;

/// Where the player is with the network.
#[derive(Debug, Clone, PartialEq)]
pub enum NetStatus {
    /// No session and no scan.
    Idle,
    /// A discovery scan is running; [`NetFrontend::servers`] fills as answers come in.
    Scanning,
    /// Connecting or being admitted (the handshake, the password exchange, the seat).
    Connecting,
    /// Seated in the lobby (also between races, and while the world loads for the next one).
    InLobby,
    /// Driving in the session's world.
    Playing,
    /// Watching the session's world (a spectator seat).
    Spectating,
    /// The session ended or could not start; the screens show why and go back to the browser.
    Disconnected(Disconnect),
}

impl NetStatus {
    /// Whether a seat is held now (lobby, world or spectating).
    pub fn is_seated(&self) -> bool {
        matches!(self, Self::InLobby | Self::Playing | Self::Spectating)
    }
}

/// Why a session is over.
#[derive(Debug, Clone, PartialEq)]
pub enum Disconnect {
    /// The client session ended, with its reason.
    Session(EndReason),
    /// Hosting or connecting failed before a session existed (a port in use, a bad address).
    Failed(String),
}

/// One LAN server a discovery scan found.
#[derive(Debug, Clone, PartialEq)]
pub struct ServerEntry {
    /// The game port's address: the reply's source address with the reply's game port.
    pub addr: SocketAddr,
    pub name: String,
    /// The mode, or `None` for a code this build does not know.
    pub mode: Option<Mode>,
    pub players: u8,
    pub max_players: u8,
    /// The server speaks this build's protocol major version.
    pub compatible: bool,
    /// The room needs a password.
    pub password: bool,
    /// A race is running now.
    pub in_progress: bool,
    /// The time from the scan's request to this reply, milliseconds.
    pub ping_ms: Option<u16>,
}

impl ServerEntry {
    /// The entry of `reply`, which came from `from`, for a client of protocol major `our_major`.
    pub fn from_reply(from: SocketAddr, reply: &Reply, our_major: u8, ping_ms: Option<u16>) -> Self {
        Self {
            addr: SocketAddr::new(from.ip(), reply.game_port),
            name: reply.name.clone(),
            mode: Mode::from_code(reply.mode),
            players: reply.players,
            max_players: reply.max_players,
            compatible: reply.proto_major == our_major,
            password: reply.flags & discovery::FLAG_PASSWORD != 0,
            in_progress: reply.flags & discovery::FLAG_IN_PROGRESS != 0,
            ping_ms,
        }
    }
}

/// What the Host screen asks for: the session's settings (name, mode, track, laps, traffic, cops, player limit), the
/// room password, the port, and the host's own name and car.
#[derive(Debug, Clone, PartialEq)]
pub struct HostOptions {
    /// Checked by `nfsmw_net::settings::validate` before it is sent.
    pub settings: Settings,
    /// The room password; `settings.password_required` is set exactly when this is `Some`.
    pub password: Option<String>,
    /// The game port (discovery answers on the next one).
    pub port: u16,
    /// The host's player name.
    pub player_name: String,
    pub loadout: Loadout,
}

/// What `net.lead` prints: the estimated server tick and the input lead.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct LeadInfo {
    /// `S_est`, once the clock is synced.
    pub server_tick: Option<f64>,
    /// How many ticks the own car runs ahead of the server, once prediction runs.
    pub lead_ticks: Option<u32>,
    /// The best round trip measured, milliseconds.
    pub rtt_ms: Option<f64>,
}

/// The network as a screen sees it. Commands that need a seat return `false` when there is none (or the link refused
/// the message), as `ClientSession`'s do.
pub trait NetFrontend {
    /// Where the player is now.
    fn status(&self) -> NetStatus;

    /// The servers the last scan found, in the order they answered.
    fn servers(&self) -> &[ServerEntry];

    /// Starts (or restarts) a discovery scan; the list is emptied first.
    fn scan(&mut self);

    /// Starts a listen server with `options` and seats the host in its lobby.
    fn host(&mut self, options: HostOptions) -> Result<(), String>;

    /// Connects to the server at `addr`, with the room password if one was typed.
    fn connect(&mut self, addr: SocketAddr, password: Option<&str>) -> Result<(), String>;

    /// Leaves the session (a host also closes its server). The status goes to `Idle`.
    fn leave(&mut self);

    /// The lobby mirror while a session exists.
    fn lobby(&self) -> Option<&LobbyView>;

    /// The events since the last call, oldest first: the front end's own queue, drained once a frame by the LAN system
    /// (lobby, feed, votes, results). The session acts on its events itself (admission, load, start, gameplay) and keeps a
    /// copy here for the screens, so draining it never takes an event from the world.
    fn take_events(&mut self) -> Vec<SessionEvent>;

    /// The last race result record.
    fn last_results(&self) -> Option<&RaceResults>;

    fn set_ready(&mut self, ready: bool) -> bool;

    /// Asks for another car (MP-30).
    fn choose_loadout(&mut self, loadout: Loadout) -> bool;

    /// Asks for a role: racer or spectator (MP-41).
    fn set_role(&mut self, role: Role) -> bool;

    /// Host or admin: changes the settings (MP-25).
    fn update_settings(&mut self, patch: SettingsPatch) -> bool;

    /// Host or admin: starts the countdown (MP-22). A listen host runs it on its own server.
    fn start_race(&mut self) -> bool;

    /// Votes for (or starts a vote on) `option` of `kind` (MP-27).
    fn vote(&mut self, kind: VoteKind, option: u32) -> bool;

    /// Host or admin: kick, ban, server mute, restart, change mode (MP-26, MP-42).
    fn admin(&mut self, command: AdminCommand) -> bool;

    /// Says `text` to everybody (MP-40).
    fn say(&mut self, text: &str) -> Result<bool, LimitError>;

    /// Sends preset line `id` (MP-44).
    fn quick_message(&mut self, id: u16) -> bool;

    /// A spectator's camera follows the next racer (`forward`) or the previous one.
    fn follow(&mut self, forward: bool);

    /// The slot the spectator camera follows, `None` for a free camera or a racer.
    fn follow_slot(&self) -> Option<u8>;

    /// The `net` overlay line (MP-82), while a session exists.
    fn stats_line(&mut self) -> Option<NetLine>;

    /// The clock and the input lead (`net.lead`).
    fn lead(&self) -> LeadInfo;

    /// Wraps the session's link in the bad-network simulator from now on (`net.sim`, MP-83).
    fn set_net_sim(&mut self, sim: NetSim) -> Result<(), String>;
}

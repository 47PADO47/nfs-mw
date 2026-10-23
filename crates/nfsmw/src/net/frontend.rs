//! `NetFrontend` for `NetSession`: what the LAN screens, the session HUD and the `net.*` console commands see of the
//! game's network (`frontend/lan/backend.rs`). Commands go to the client session; the lobby, the result record and the
//! server list are this frame's copies (`session.rs`), so the screens borrow them without the client's `RefCell`.

use std::net::SocketAddr;
use std::time::Duration;

use nfsmw_net::client_session::{LobbyView, SessionEvent, Status};
use nfsmw_net::loadout::Loadout;
use nfsmw_net::message::{AdminCommand, LimitError, RaceResults, Role, SessionPhase, SettingsPatch, VoteKind};
use nfsmw_net::netsim::NetSim;
use nfsmw_net::netstats::NetLine;

use super::client::{GameSession, PlayerSetup};
use super::options::ServeOptions;
use super::session::{Mode, NetSession};
use crate::frontend::lan::backend::{HostOptions, ServerEntry};
use crate::frontend::lan::{Disconnect, LeadInfo, NetFrontend, NetStatus};
use crate::scenes::world::net::catalog::CarCatalog;

/// The default game port; discovery answers on the next one.
const DEFAULT_PORT: u16 = 9750;
/// How long a scan from the browser listens.
const SCAN_FOR: Duration = Duration::from_secs(2);

impl NetSession {
    /// Runs `f` on the session, when there is a client that is not busy.
    fn with_session<T>(&self, f: impl FnOnce(&mut GameSession) -> T) -> Option<T> {
        let client = self.client.as_ref()?;
        let mut client = client.try_borrow_mut().ok()?;
        Some(f(client.session_mut()))
    }

    /// The car folder of `loadout` in this install, or the player's car when the install has none of that type.
    fn folder_of(&self, loadout: &Loadout) -> String {
        let catalog = CarCatalog::new(self.cars());
        catalog.folder(loadout.car_type).map_or_else(|| self.player.car.clone(), str::to_owned)
    }
}

/// Where a joined session is, from its phase and the player's role.
fn seated_status(session: &GameSession) -> NetStatus {
    if session.is_spectator() {
        return NetStatus::Spectating;
    }
    match session.lobby().phase {
        SessionPhase::Lobby | SessionPhase::Results => NetStatus::InLobby,
        SessionPhase::Loading
        | SessionPhase::WaitLoads
        | SessionPhase::Intro
        | SessionPhase::Countdown
        | SessionPhase::Running => NetStatus::Playing,
    }
}

impl NetFrontend for NetSession {
    fn status(&self) -> NetStatus {
        let client = self.client.as_ref().and_then(|client| client.try_borrow().ok());
        let Some(client) = client else {
            if let Some(error) = &self.error {
                return NetStatus::Disconnected(Disconnect::Failed(error.clone()));
            }
            return match (self.mode, self.scan.is_some()) {
                (Mode::Host, _) => NetStatus::Connecting,
                (_, true) => NetStatus::Scanning,
                (_, false) => NetStatus::Idle,
            };
        };
        match client.session().status() {
            Status::Connecting | Status::Admitting | Status::Reconnecting { .. } => NetStatus::Connecting,
            Status::Ended(reason) => NetStatus::Disconnected(Disconnect::Session(reason.clone())),
            Status::Joined => seated_status(client.session()),
        }
    }

    fn servers(&self) -> &[ServerEntry] {
        &self.servers
    }

    fn scan(&mut self) {
        if let Err(error) = self.start_scan(DEFAULT_PORT, SCAN_FOR) {
            self.error = Some(format!("{error:#}"));
        }
    }

    fn host(&mut self, options: HostOptions) -> Result<(), String> {
        if options.password.is_some() {
            return Err("room passwords are not supported yet: the server cannot check one".into());
        }
        let car = self.folder_of(&options.loadout);
        self.player = PlayerSetup { name: options.player_name.clone(), car: car.clone() };
        let name = options.settings.name.as_str().to_owned();
        let serve = ServeOptions {
            port: Some(options.port),
            name: Some(name),
            car,
            settings: Some(options.settings),
            ..ServeOptions::default()
        };
        NetSession::host(self, serve).map_err(|error| format!("{error:#}"))
    }

    fn connect(&mut self, addr: SocketAddr, password: Option<&str>) -> Result<(), String> {
        self.join(addr, password).map_err(|error| format!("{error:#}"))
    }

    fn leave(&mut self) {
        NetSession::leave(self);
        self.error = None;
    }

    fn lobby(&self) -> Option<&LobbyView> {
        self.lobby.as_ref()
    }

    fn take_events(&mut self) -> Vec<SessionEvent> {
        self.refresh();
        self.take_frontend_events()
    }

    fn last_results(&self) -> Option<&RaceResults> {
        self.results.as_ref()
    }

    fn set_ready(&mut self, ready: bool) -> bool {
        self.with_session(|session| session.set_ready(ready)).unwrap_or(false)
    }

    fn choose_loadout(&mut self, loadout: Loadout) -> bool {
        self.player.car = self.folder_of(&loadout);
        self.with_session(|session| session.choose_loadout(loadout)).unwrap_or(false)
    }

    fn set_role(&mut self, role: Role) -> bool {
        self.with_session(|session| session.set_role(role)).unwrap_or(false)
    }

    fn update_settings(&mut self, patch: SettingsPatch) -> bool {
        self.with_session(|session| session.update_settings(patch)).unwrap_or(false)
    }

    /// The lobby starts by itself once every racer is ready (`auto_start`), so the host starts it by saying ready; the
    /// client session has no message of its own for an early start.
    fn start_race(&mut self) -> bool {
        self.with_session(|session| session.lobby().is_host() && session.set_ready(true)).unwrap_or(false)
    }

    fn vote(&mut self, kind: VoteKind, option: u32) -> bool {
        self.with_session(|session| session.vote(kind, option)).unwrap_or(false)
    }

    fn admin(&mut self, command: AdminCommand) -> bool {
        self.with_session(|session| session.host_command(command)).unwrap_or(false)
    }

    fn say(&mut self, text: &str) -> Result<bool, LimitError> {
        self.with_session(|session| session.say(text)).unwrap_or(Ok(false))
    }

    fn quick_message(&mut self, id: u16) -> bool {
        self.with_session(|session| session.quick_message(id)).unwrap_or(false)
    }

    fn follow(&mut self, forward: bool) {
        self.with_session(|session| match forward {
            true => session.follow_next(),
            false => session.follow_previous(),
        });
    }

    /// A player's car is the entity with its slot's number (`world-host.md` section 4), so the followed entity is the
    /// slot.
    fn follow_slot(&self) -> Option<u8> {
        let client = self.client.as_ref()?.try_borrow().ok()?;
        let session = client.session();
        if !session.is_spectator() {
            return None;
        }
        u8::try_from(session.follow_target()?).ok()
    }

    fn stats_line(&mut self) -> Option<NetLine> {
        self.with_session(|session| session.net_line())
    }

    fn lead(&self) -> LeadInfo {
        let Some(client) = self.client.as_ref().and_then(|client| client.try_borrow().ok()) else {
            return LeadInfo::default();
        };
        let session = client.session();
        LeadInfo {
            server_tick: session.server_tick(),
            lead_ticks: session.lead_ticks(),
            rtt_ms: session.net_stats().rtt_ms(),
        }
    }

    /// The game's link is the net thread's UDP endpoint, which has no simulator to wrap; only `NetSim::OFF` is taken.
    fn set_net_sim(&mut self, sim: NetSim) -> Result<(), String> {
        match sim == NetSim::OFF {
            true => Ok(()),
            false => Err("the bad-network simulator is not wired into the game's endpoint yet".into()),
        }
    }

    fn as_any_mut(&mut self) -> Option<&mut dyn std::any::Any> {
        Some(self)
    }
}

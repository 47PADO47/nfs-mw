//! A scripted [`NetFrontend`] for tests: the test sets the status, the servers and the lobby, pushes events, and reads
//! back every call the screens made ([`Call`]). Nothing is sent anywhere.

use std::net::SocketAddr;

use nfsmw_net::client_session::{LobbyView, SessionEvent};
use nfsmw_net::loadout::Loadout;
use nfsmw_net::message::{
    AdminCommand, ChatText, LimitError, LobbyPlayer, Name, RaceResults, Role, Settings, SettingsPatch, VoteKind,
};
use nfsmw_net::netsim::NetSim;
use nfsmw_net::netstats::NetLine;

use super::backend::{HostOptions, LeadInfo, NetFrontend, NetStatus, ServerEntry};

/// A call a screen made, in order.
#[derive(Debug, Clone, PartialEq)]
pub enum Call {
    Scan,
    Host(Box<HostOptions>),
    Connect(SocketAddr, Option<String>),
    Leave,
    Ready(bool),
    Loadout(Loadout),
    Role(Role),
    Settings(Box<SettingsPatch>),
    Start,
    Vote(VoteKind, u32),
    Admin(AdminCommand),
    Say(String),
    Whisper(u8, String),
    Quick(u16),
    Follow(bool),
    NetSim(NetSim),
}

/// The mock. Every field is public so a test can set the state it needs.
#[derive(Debug, Default)]
pub struct MockNetFrontend {
    pub status: Option<NetStatus>,
    pub servers: Vec<ServerEntry>,
    pub lobby: Option<LobbyView>,
    pub events: Vec<SessionEvent>,
    pub results: Option<RaceResults>,
    pub follow: Option<u8>,
    pub line: Option<NetLine>,
    pub lead: LeadInfo,
    pub calls: Vec<Call>,
    /// `host` and `connect` fail with this text when set.
    pub fail: Option<String>,
}

impl MockNetFrontend {
    /// A mock seated in a lobby of `players` (slot 0 is the host) with this client in `my_slot`.
    pub fn in_lobby(players: &[&str], my_slot: u8) -> Self {
        let mut lobby = LobbyView::new(64);
        for (slot, name) in players.iter().enumerate() {
            lobby.upsert(player(slot as u8, name));
        }
        lobby.my_slot = Some(my_slot);
        lobby.settings = Some(Settings::lan(nfsmw_net::message::ServerName::new("Test room").unwrap_or_default()));
        Self { status: Some(NetStatus::InLobby), lobby: Some(lobby), ..Self::default() }
    }

    /// The calls since the last time, emptied.
    pub fn take_calls(&mut self) -> Vec<Call> {
        std::mem::take(&mut self.calls)
    }

    fn seated(&self) -> bool {
        self.status().is_seated()
    }
}

/// A roster row named `name` in `slot`, ready, a racer, with ping 12.
pub fn player(slot: u8, name: &str) -> LobbyPlayer {
    LobbyPlayer {
        slot,
        player_id: [slot; 16],
        name: Name::new(name).unwrap_or_default(),
        car_type: 0,
        ready: false,
        role: Role::Racer,
        team: 0,
        ping_ms: 12,
        flags: 0,
    }
}

impl NetFrontend for MockNetFrontend {
    fn status(&self) -> NetStatus {
        self.status.clone().unwrap_or(NetStatus::Idle)
    }

    fn servers(&self) -> &[ServerEntry] {
        &self.servers
    }

    fn scan(&mut self) {
        self.calls.push(Call::Scan);
        self.status = Some(NetStatus::Scanning);
    }

    fn host(&mut self, options: HostOptions) -> Result<(), String> {
        if let Some(fail) = &self.fail {
            return Err(fail.clone());
        }
        self.calls.push(Call::Host(Box::new(options)));
        self.status = Some(NetStatus::Connecting);
        Ok(())
    }

    fn connect(&mut self, addr: SocketAddr, password: Option<&str>) -> Result<(), String> {
        if let Some(fail) = &self.fail {
            return Err(fail.clone());
        }
        self.calls.push(Call::Connect(addr, password.map(str::to_owned)));
        self.status = Some(NetStatus::Connecting);
        Ok(())
    }

    fn leave(&mut self) {
        self.calls.push(Call::Leave);
        self.status = Some(NetStatus::Idle);
        self.lobby = None;
    }

    fn lobby(&self) -> Option<&LobbyView> {
        self.lobby.as_ref()
    }

    fn take_events(&mut self) -> Vec<SessionEvent> {
        std::mem::take(&mut self.events)
    }

    fn last_results(&self) -> Option<&RaceResults> {
        self.results.as_ref()
    }

    fn set_ready(&mut self, ready: bool) -> bool {
        self.calls.push(Call::Ready(ready));
        self.seated()
    }

    fn choose_loadout(&mut self, loadout: Loadout) -> bool {
        self.calls.push(Call::Loadout(loadout));
        self.seated()
    }

    fn set_role(&mut self, role: Role) -> bool {
        self.calls.push(Call::Role(role));
        self.seated()
    }

    fn update_settings(&mut self, patch: SettingsPatch) -> bool {
        self.calls.push(Call::Settings(Box::new(patch)));
        self.seated()
    }

    fn start_race(&mut self) -> bool {
        self.calls.push(Call::Start);
        self.seated()
    }

    fn vote(&mut self, kind: VoteKind, option: u32) -> bool {
        self.calls.push(Call::Vote(kind, option));
        self.seated()
    }

    fn admin(&mut self, command: AdminCommand) -> bool {
        self.calls.push(Call::Admin(command));
        self.seated()
    }

    fn say(&mut self, text: &str) -> Result<bool, LimitError> {
        let text = ChatText::new(text)?;
        self.calls.push(Call::Say(text.as_str().to_owned()));
        Ok(self.seated())
    }

    fn whisper(&mut self, slot: u8, text: &str) -> Result<bool, LimitError> {
        let text = ChatText::new(text)?;
        self.calls.push(Call::Whisper(slot, text.as_str().to_owned()));
        Ok(self.seated())
    }

    fn quick_message(&mut self, id: u16) -> bool {
        self.calls.push(Call::Quick(id));
        self.seated()
    }

    fn follow(&mut self, forward: bool) {
        self.calls.push(Call::Follow(forward));
    }

    fn follow_slot(&self) -> Option<u8> {
        self.follow
    }

    fn stats_line(&mut self) -> Option<NetLine> {
        self.line
    }

    fn lead(&self) -> LeadInfo {
        self.lead
    }

    fn set_net_sim(&mut self, sim: NetSim) -> Result<(), String> {
        self.calls.push(Call::NetSim(sim));
        Ok(())
    }
}

//! The [`NetFrontend`] of a run with no network wired in: the menu opens and says why nothing can start.

use std::net::SocketAddr;

use nfsmw_net::client_session::{LobbyView, SessionEvent};
use nfsmw_net::loadout::Loadout;
use nfsmw_net::message::{AdminCommand, LimitError, RaceResults, Role, SettingsPatch, VoteKind};
use nfsmw_net::netsim::NetSim;
use nfsmw_net::netstats::NetLine;

use super::backend::{Disconnect, HostOptions, LeadInfo, NetFrontend, NetStatus, ServerEntry};

/// What the menu says when host or join is tried.
pub const NO_NETWORK: &str = "LAN play is not available in this run";

/// No network: every command is refused.
#[derive(Debug, Default)]
pub struct Offline {
    failed: bool,
}

impl NetFrontend for Offline {
    fn status(&self) -> NetStatus {
        match self.failed {
            true => NetStatus::Disconnected(Disconnect::Failed(NO_NETWORK.into())),
            false => NetStatus::Idle,
        }
    }

    fn servers(&self) -> &[ServerEntry] {
        &[]
    }

    fn scan(&mut self) {}

    fn host(&mut self, _options: HostOptions) -> Result<(), String> {
        self.failed = true;
        Err(NO_NETWORK.into())
    }

    fn connect(&mut self, _addr: SocketAddr, _password: Option<&str>) -> Result<(), String> {
        self.failed = true;
        Err(NO_NETWORK.into())
    }

    fn leave(&mut self) {
        self.failed = false;
    }

    fn lobby(&self) -> Option<&LobbyView> {
        None
    }

    fn take_events(&mut self) -> Vec<SessionEvent> {
        Vec::new()
    }

    fn last_results(&self) -> Option<&RaceResults> {
        None
    }

    fn set_ready(&mut self, _ready: bool) -> bool {
        false
    }

    fn choose_loadout(&mut self, _loadout: Loadout) -> bool {
        false
    }

    fn set_role(&mut self, _role: Role) -> bool {
        false
    }

    fn update_settings(&mut self, _patch: SettingsPatch) -> bool {
        false
    }

    fn start_race(&mut self) -> bool {
        false
    }

    fn vote(&mut self, _kind: VoteKind, _option: u32) -> bool {
        false
    }

    fn admin(&mut self, _command: AdminCommand) -> bool {
        false
    }

    fn say(&mut self, _text: &str) -> Result<bool, LimitError> {
        Ok(false)
    }

    fn whisper(&mut self, _slot: u8, _text: &str) -> Result<bool, LimitError> {
        Ok(false)
    }

    fn quick_message(&mut self, _id: u16) -> bool {
        false
    }

    fn follow(&mut self, _forward: bool) {}

    fn follow_slot(&self) -> Option<u8> {
        None
    }

    fn stats_line(&mut self) -> Option<NetLine> {
        None
    }

    fn lead(&self) -> LeadInfo {
        LeadInfo::default()
    }

    fn set_net_sim(&mut self, _sim: NetSim) -> Result<(), String> {
        Err(NO_NETWORK.into())
    }
}

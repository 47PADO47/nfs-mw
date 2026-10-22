//! A pretend session for working on the LAN screens without a second machine or the network wired in: with
//! `NFSMW_LAN_DEMO=1` and no game network, the menu runs on [`DemoNet`]. A scan finds three rooms, hosting or joining seats
//! the player with two others, chat echoes, the start counts down five seconds, the "race" lasts five more and ends with a
//! result record. It sends nothing anywhere. Offscreen screenshots of the pages use it.

use std::net::SocketAddr;

use nfsmw_net::client_session::{ChatLine, LobbyView, SessionEvent};
use nfsmw_net::loadout::Loadout;
use nfsmw_net::message::{
    AdminCommand, ChatChannel, EventHead, LimitError, List, LobbyPlayer, Mode, Name, NoticeCode, RaceResults,
    ResultRow, Role, ServerName, SessionPhase, Settings, SettingsPatch, StartCountdown, SystemNotice, VoteKind,
    VoteOption, VoteState,
};
use nfsmw_net::netsim::NetSim;
use nfsmw_net::netstats::NetLine;

use super::backend::{Disconnect, HostOptions, LeadInfo, NetFrontend, NetStatus, ServerEntry};
use super::car::car_type;

/// The environment variable that turns the demo on.
pub const ENV: &str = "NFSMW_LAN_DEMO";
/// Frames of the countdown and of the race (the menu asks once a frame, about 60 a second).
const COUNTDOWN_FRAMES: u32 = 300;
const RACE_FRAMES: u32 = 300;

/// Whether the environment asks for the demo.
pub fn wanted() -> bool {
    std::env::var(ENV).is_ok_and(|value| matches!(value.trim(), "1" | "on" | "true" | "yes"))
}

/// The pretend session.
#[derive(Debug)]
pub struct DemoNet {
    status: NetStatus,
    servers: Vec<ServerEntry>,
    lobby: Option<LobbyView>,
    events: Vec<SessionEvent>,
    results: Option<RaceResults>,
    frame: u32,
    race_ends: Option<u32>,
}

impl Default for DemoNet {
    fn default() -> Self {
        Self {
            status: NetStatus::Idle,
            servers: Vec::new(),
            lobby: None,
            events: Vec::new(),
            results: None,
            frame: 0,
            race_ends: None,
        }
    }
}

fn row(slot: u8, name: &str, car: &str, ready: bool, flags: u8) -> LobbyPlayer {
    LobbyPlayer {
        slot,
        player_id: [slot + 1; 16],
        name: Name::new(name).unwrap_or_default(),
        car_type: car_type(car),
        ready,
        role: Role::Racer,
        team: 0,
        ping_ms: 2 + u16::from(slot) * 3,
        flags,
    }
}

fn notice(code: NoticeCode, slot: u32) -> SessionEvent {
    SessionEvent::Notice(SystemNotice { code, args: List::new(vec![slot]).unwrap_or_default() })
}

impl DemoNet {
    /// Seats the player in a room of `settings`: `me` in `my_slot`, the host in slot 0.
    fn seat(&mut self, settings: Settings, me: &str, my_slot: u8) {
        let mut lobby = LobbyView::new(64);
        let host = match my_slot {
            0 => me,
            _ => "Rhino",
        };
        lobby.upsert(row(0, host, "BMWM3GTR", false, nfsmw_net::lobby::FLAG_HOST));
        lobby.upsert(row(1, "Sully", "SL65", true, 0));
        let third = match my_slot {
            2 => me,
            _ => "Ming",
        };
        lobby.upsert(row(2, third, "CAMARO", false, 0));
        lobby.my_slot = Some(my_slot);
        lobby.settings = Some(settings);
        self.lobby = Some(lobby);
        self.status = NetStatus::InLobby;
        self.events.push(SessionEvent::Admitted { slot: my_slot, entity: u16::from(my_slot), resumed: false });
        self.events.push(notice(NoticeCode::Join, 1));
        self.events.push(SessionEvent::Chat(ChatLine {
            from_slot: 1,
            channel: ChatChannel::All,
            text: "hi! ready when you are".into(),
            tick: 0,
        }));
    }

    fn me(&mut self) -> Option<&mut LobbyPlayer> {
        let lobby = self.lobby.as_mut()?;
        let slot = lobby.my_slot?;
        lobby.players.iter_mut().find(|row| row.slot == slot)
    }

    fn seated(&self) -> bool {
        self.status.is_seated()
    }

    /// The countdown and the race move on by one frame.
    fn advance(&mut self) {
        self.frame += 1;
        let Some(lobby) = self.lobby.as_mut() else { return };
        if let Some(countdown) = &lobby.countdown
            && self.frame >= countdown.start_tick
        {
            lobby.countdown = None;
            lobby.phase = SessionPhase::Running;
            self.status = NetStatus::Playing;
            self.race_ends = Some(self.frame + RACE_FRAMES);
        }
        if self.race_ends.is_some_and(|end| self.frame >= end) {
            self.race_ends = None;
            lobby.phase = SessionPhase::Lobby;
            lobby.players.iter_mut().for_each(|row| row.ready = false);
            let finish = |slot: u8, place: u8, time_ms: u32| ResultRow {
                slot,
                entity_id: u16::from(slot),
                place,
                time_ms,
                best_lap_ms: time_ms / 3,
                flags: 0,
            };
            let rows = vec![finish(1, 1, 151_250), finish(0, 2, 152_900), finish(2, 3, 158_010)];
            let results = RaceResults { head: EventHead::default(), rows: List::new(rows).unwrap_or_default() };
            self.results = Some(results.clone());
            self.events.push(SessionEvent::Results(results));
            self.status = NetStatus::InLobby;
        }
    }
}

impl NetFrontend for DemoNet {
    fn status(&self) -> NetStatus {
        self.status.clone()
    }

    fn servers(&self) -> &[ServerEntry] {
        &self.servers
    }

    fn scan(&mut self) {
        let entry = |last: u8, name: &str, mode, players, password, compatible| ServerEntry {
            addr: SocketAddr::from(([192, 168, 1, last], 9750)),
            name: name.into(),
            mode: Some(mode),
            players,
            max_players: 8,
            compatible,
            password,
            in_progress: false,
            ping_ms: Some(u16::from(last % 7) + 1),
        };
        self.servers = vec![
            entry(20, "Rhino's game", Mode::FreeRoam, 3, false, true),
            entry(31, "Friday circuit", Mode::Circuit, 5, true, true),
            entry(44, "Old build", Mode::Sprint, 2, false, false),
        ];
    }

    fn host(&mut self, options: HostOptions) -> Result<(), String> {
        let name = options.player_name.clone();
        self.seat(options.settings, &name, 0);
        Ok(())
    }

    fn connect(&mut self, addr: SocketAddr, password: Option<&str>) -> Result<(), String> {
        let found = self.servers.iter().find(|entry| entry.addr == addr).cloned();
        if found.as_ref().is_some_and(|entry| entry.password) && password != Some("demo") {
            let reason = match password {
                None => nfsmw_net::client_session::EndReason::PasswordRequired,
                Some(_) => nfsmw_net::client_session::EndReason::PasswordRejected,
            };
            self.status = NetStatus::Disconnected(Disconnect::Session(reason));
            return Ok(());
        }
        let room = found.map_or_else(|| addr.to_string(), |entry| entry.name);
        let settings = Settings::lan(ServerName::new(&room).unwrap_or_default());
        self.seat(settings, "Player", 2);
        Ok(())
    }

    fn leave(&mut self) {
        *self = Self { servers: std::mem::take(&mut self.servers), ..Self::default() };
    }

    fn lobby(&self) -> Option<&LobbyView> {
        self.lobby.as_ref()
    }

    fn take_events(&mut self) -> Vec<SessionEvent> {
        self.advance();
        std::mem::take(&mut self.events)
    }

    fn last_results(&self) -> Option<&RaceResults> {
        self.results.as_ref()
    }

    fn set_ready(&mut self, ready: bool) -> bool {
        self.me().map(|me| me.ready = ready).is_some()
    }

    fn choose_loadout(&mut self, loadout: Loadout) -> bool {
        self.me().map(|me| me.car_type = loadout.car_type).is_some()
    }

    fn set_role(&mut self, role: Role) -> bool {
        self.me().map(|me| me.role = role).is_some()
    }

    fn update_settings(&mut self, patch: SettingsPatch) -> bool {
        let Some(settings) = self.lobby.as_mut().and_then(|lobby| lobby.settings.as_mut()) else { return false };
        *settings = settings.apply(&patch);
        true
    }

    fn start_race(&mut self) -> bool {
        let start_tick = self.frame + COUNTDOWN_FRAMES;
        let Some(lobby) = self.lobby.as_mut() else { return false };
        lobby.countdown = Some(StartCountdown { start_tick, seconds: 5 });
        lobby.phase = SessionPhase::Countdown;
        true
    }

    fn vote(&mut self, kind: VoteKind, option: u32) -> bool {
        let ends_tick = self.frame + 1800;
        let Some(lobby) = self.lobby.as_mut() else { return false };
        let options = List::new(vec![VoteOption { id: option, count: 1 }]).unwrap_or_default();
        lobby.vote = Some(VoteState { kind, options, ends_tick });
        true
    }

    fn admin(&mut self, command: AdminCommand) -> bool {
        let Some(lobby) = self.lobby.as_mut() else { return false };
        if let AdminCommand::Kick { slot } | AdminCommand::Ban { slot, .. } = command {
            lobby.remove(slot);
            self.events.push(notice(NoticeCode::Kick, u32::from(slot)));
        }
        if let AdminCommand::Mute { slot, muted } = command
            && let Some(row) = lobby.players.iter_mut().find(|row| row.slot == slot)
        {
            match muted {
                true => row.flags |= nfsmw_net::lobby::FLAG_MUTED,
                false => row.flags &= !nfsmw_net::lobby::FLAG_MUTED,
            }
        }
        true
    }

    fn say(&mut self, text: &str) -> Result<bool, LimitError> {
        let text = nfsmw_net::message::ChatText::new(text)?;
        let Some(slot) = self.lobby.as_ref().and_then(|lobby| lobby.my_slot) else { return Ok(false) };
        let line =
            ChatLine { from_slot: slot, channel: ChatChannel::All, text: text.as_str().to_owned(), tick: self.frame };
        self.events.push(SessionEvent::Chat(line));
        Ok(true)
    }

    fn quick_message(&mut self, _id: u16) -> bool {
        self.seated()
    }

    fn follow(&mut self, _forward: bool) {}

    fn follow_slot(&self) -> Option<u8> {
        matches!(self.status, NetStatus::Spectating).then_some(1)
    }

    fn stats_line(&mut self) -> Option<NetLine> {
        self.seated().then_some(NetLine {
            rtt_ms: Some(3.0),
            jitter_ms: 0.4,
            loss: 0.002,
            snapshot_age_ms: Some(48),
            corrections_per_min: 2,
            snaps_per_min: 0,
            bytes_in_per_s: 14_200.0,
            bytes_out_per_s: 2_100.0,
        })
    }

    fn lead(&self) -> LeadInfo {
        LeadInfo { server_tick: Some(f64::from(self.frame)), lead_ticks: Some(3), rtt_ms: Some(3.0) }
    }

    fn set_net_sim(&mut self, _sim: NetSim) -> Result<(), String> {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hosting_then_starting_runs_a_countdown_a_race_and_results() {
        let mut net = DemoNet::default();
        let settings = Settings::lan(ServerName::new("Room").unwrap());
        let options = HostOptions {
            settings,
            password: None,
            port: 9750,
            player_name: "Pietro".into(),
            loadout: Loadout::stock(0),
        };
        net.host(options).unwrap();
        assert_eq!(net.status(), NetStatus::InLobby);
        assert_eq!(net.lobby().unwrap().me().unwrap().name.as_str(), "Pietro");
        assert!(net.start_race());
        for _ in 0..COUNTDOWN_FRAMES {
            net.take_events();
        }
        assert_eq!(net.status(), NetStatus::Playing);
        let mut results = None;
        for _ in 0..RACE_FRAMES {
            results = results.or(net.take_events().into_iter().find(|e| matches!(e, SessionEvent::Results(_))));
        }
        assert!(results.is_some());
        assert_eq!(net.status(), NetStatus::InLobby);
    }

    #[test]
    fn a_locked_room_wants_the_demo_password() {
        let mut net = DemoNet::default();
        net.scan();
        let locked = net.servers()[1].addr;
        net.connect(locked, None).unwrap();
        assert!(matches!(net.status(), NetStatus::Disconnected(_)));
        net.connect(locked, Some("demo")).unwrap();
        assert_eq!(net.status(), NetStatus::InLobby);
    }
}

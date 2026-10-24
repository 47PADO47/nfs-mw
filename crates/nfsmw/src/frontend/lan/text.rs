//! The words the LAN screens, the session HUD and the console share: modes, the status, why a session ended, a server's
//! line in the browser, a system notice and a race time. The original has no strings for LAN play: the words are the
//! rewrite's catalogue (`i18n/<locale>/lan.ftl`), and the game's own label wherever it has the word ([`label`]).

use blackbox_net_core::endpoint::DisconnectReason;
use blackbox_net_core::secure::HandshakeError;
use blackbox_net_core::secure::packet::PacketError;
use nfsmw_net::client_session::{EndReason, LobbyView};
use nfsmw_net::message::{KickReason, Mode, NoticeCode, SessionPhase, SystemNotice};
use nfsmw_net::version::RejectReason;

use super::backend::{Disconnect, NetStatus, ServerEntry};
use crate::i18n::fmt::ordinal;
use crate::i18n::{self, Args, t};

/// The game labels the LAN screens show: the original's own words, so every installed language has them.
pub mod label {
    /// `COMMON_BACK`.
    pub const BACK: u32 = 0x8CD5_67B8;
    /// `COMMON_CANCEL`.
    pub const CANCEL: u32 = 0x1A29_4DAD;
    /// `COMMON_CONTINUE`.
    pub const CONTINUE: u32 = 0x8098_A54C;
    /// `COMMON_NO`.
    pub const NO: u32 = 0x417B_25E4;
    /// `COMMON_OFF`.
    pub const OFF: u32 = 0x70DF_E5C2;
    /// `COMMON_SELECT`.
    pub const SELECT: u32 = 0x3FC3_2947;
    /// `COMMON_YES`.
    pub const YES: u32 = 0x70E0_1038;
    /// `HUD_KMH`, the unit the HUD's speed shows in.
    pub const KMH: u32 = 0x8569_A25F;
    /// `OL_GAME_ROOM_READY`.
    pub const READY: u32 = 0xFC00_D963;
    /// `OL_JOIN`.
    pub const JOIN: u32 = 0x8D20_0029;
    /// `OL_PASSWORD`.
    pub const PASSWORD: u32 = 0x8BC0_B80C;
    /// `OL_PLAYER_STATUS_LOADING`.
    pub const LOADING: u32 = 0xAEB6_FD26;
    /// `OL_PLAYER_STATUS_WAITING`.
    pub const WAITING: u32 = 0xDD68_EABB;
    /// `QR_OPT_LAPS`, the quick race's laps option.
    pub const LAPS: u32 = 0x4849_4E83;
    /// `UI_CHAT`.
    pub const CHAT: u32 = 0x53F9_FD5C;
    /// `UI_ONLINE_LOBBY`.
    pub const LOBBY: u32 = 0x4AB6_A4F8;
    /// `RACETYPES_CIRCUIT`.
    pub const CIRCUIT: u32 = 0x034F_A2C1;
    /// `RACETYPES_SPRINT`.
    pub const SPRINT: u32 = 0xB94F_D70E;
    /// `RACETYPES_DRAG`.
    pub const DRAG: u32 = 0x6F54_7E4C;
    /// `RACETYPES_SPEEDTRAP`.
    pub const SPEEDTRAP: u32 = 0xEE1E_DC76;
    /// `RACETYPES_KNOCKOUT`.
    pub const KNOCKOUT: u32 = 0x4930_F5FC;
}

/// The text of game label `hash` in the current language (the original's default-string error text when none has it).
pub fn game(hash: u32) -> String {
    i18n::game_or_error(hash)
}

/// The modes the Host screen offers, in its order. Drag and cops vs racers are refused by the lobby (ADR 0011).
pub const HOST_MODES: [Mode; 7] =
    [Mode::FreeRoam, Mode::Circuit, Mode::Sprint, Mode::Speedtrap, Mode::Knockout, Mode::PursuitCoop, Mode::Tag];

/// A mode's name: the game's race type where it has one.
pub fn mode_name(mode: Mode) -> String {
    let id = match mode {
        Mode::Circuit => return game(label::CIRCUIT),
        Mode::Sprint => return game(label::SPRINT),
        Mode::Drag => return game(label::DRAG),
        Mode::Speedtrap => return game(label::SPEEDTRAP),
        Mode::Knockout => return game(label::KNOCKOUT),
        Mode::Custom(id) => return t!("lan-mode-custom", id = id),
        Mode::FreeRoam => "lan-mode-free-roam",
        Mode::PursuitCoop => "lan-mode-pursuit-coop",
        Mode::CopsVsRacers => "lan-mode-cops-vs-racers",
        Mode::Tag => "lan-mode-tag",
    };
    i18n::text(id, &Args::new())
}

/// A session phase as the lobby's status line names it.
pub fn phase_name(phase: SessionPhase) -> String {
    let id = match phase {
        SessionPhase::Lobby => return game(label::LOBBY),
        SessionPhase::Loading => return game(label::LOADING),
        SessionPhase::WaitLoads => return game(label::WAITING),
        SessionPhase::Intro => "lan-phase-intro",
        SessionPhase::Countdown => "lan-phase-countdown",
        SessionPhase::Running => "lan-phase-running",
        SessionPhase::Results => "lan-phase-results",
    };
    i18n::text(id, &Args::new())
}

/// The status as one line.
pub fn status_text(status: &NetStatus) -> String {
    let id = match status {
        NetStatus::Disconnected(why) => return t!("lan-status-disconnected", why = disconnect_text(why)),
        NetStatus::Idle => "lan-status-idle",
        NetStatus::Scanning => "lan-status-scanning",
        NetStatus::Connecting => "lan-status-connecting",
        NetStatus::InLobby => "lan-status-in-lobby",
        NetStatus::Playing => "lan-status-playing",
        NetStatus::Spectating => "lan-status-spectating",
    };
    i18n::text(id, &Args::new())
}

/// Why a session is over, as a sentence fragment.
pub fn disconnect_text(why: &Disconnect) -> String {
    match why {
        // The system's own words for a port in use or a bad address: not the rewrite's to translate.
        Disconnect::Failed(text) => text.clone(),
        Disconnect::Session(reason) => end_text(reason),
    }
}

/// Why a client session ended.
pub fn end_text(reason: &EndReason) -> String {
    let id = match reason {
        EndReason::Rejected(rejected) => return t!("lan-end-rejected", why = reject_text(rejected.reason)),
        EndReason::Kicked(kick) => return kick_text(kick.reason, kick.ban_minutes),
        EndReason::Link(link) => return t!("lan-end-link", why = link_text(link)),
        EndReason::Left => "lan-end-left",
        EndReason::ResumeExpired => "lan-end-resume-expired",
        EndReason::PasswordRequired => "lan-end-password-required",
        EndReason::PasswordRejected => "lan-end-password-rejected",
        EndReason::SchemaMismatch => "lan-end-schema-mismatch",
        EndReason::Protocol => "lan-end-protocol",
    };
    i18n::text(id, &Args::new())
}

fn reject_text(reason: RejectReason) -> String {
    let id = match reason {
        RejectReason::ServerFull => "lan-reject-server-full",
        RejectReason::ProtocolMismatch => "lan-reject-protocol-mismatch",
        RejectReason::DataMismatch => "lan-reject-data-mismatch",
        RejectReason::ModMismatch => "lan-reject-mod-mismatch",
        RejectReason::Banned => "lan-reject-banned",
        RejectReason::InProgress => "lan-reject-in-progress",
        RejectReason::LoadoutRejected => "lan-reject-loadout-rejected",
        RejectReason::RateLimited => "lan-reject-rate-limited",
        RejectReason::ServerShuttingDown => "lan-reject-server-shutting-down",
        RejectReason::SimBuildMismatch => "lan-reject-sim-build-mismatch",
    };
    i18n::text(id, &Args::new())
}

/// Why the connection closed (the link's own reason, not the session's).
fn link_text(link: &DisconnectReason) -> String {
    let id = match link {
        DisconnectReason::Remote(code) => return t!("lan-link-remote", code = *code),
        DisconnectReason::Local(code) => return t!("lan-link-local", code = *code),
        DisconnectReason::Handshake(error) => return t!("lan-link-handshake", why = handshake_text(error)),
        DisconnectReason::Timeout => "lan-link-timeout",
        DisconnectReason::Overflow => "lan-link-overflow",
        DisconnectReason::Shutdown => "lan-link-shutdown",
    };
    i18n::text(id, &Args::new())
}

fn handshake_text(error: &HandshakeError) -> String {
    let id = match error {
        HandshakeError::Packet(packet) => return t!("lan-handshake-packet", why = packet_text(*packet)),
        HandshakeError::Rejected(code) if !(1..=3).contains(code) => {
            return t!("lan-handshake-rejected", code = *code);
        }
        HandshakeError::Noise => "lan-handshake-noise",
        HandshakeError::Random => "lan-handshake-random",
        HandshakeError::Config => "lan-handshake-config",
        HandshakeError::NoAnswer => "lan-handshake-no-answer",
        HandshakeError::Rejected(1) => "lan-handshake-full",
        HandshakeError::Rejected(2) => "lan-handshake-protocol",
        HandshakeError::Rejected(_) => "lan-handshake-shutting-down",
    };
    i18n::text(id, &Args::new())
}

fn packet_text(packet: PacketError) -> String {
    let id = match packet {
        PacketError::BadLength => "lan-packet-bad-length",
        PacketError::BadType => "lan-packet-bad-type",
        PacketError::BadVersion => "lan-packet-bad-version",
        PacketError::BadPadding => "lan-packet-bad-padding",
        PacketError::BadField => "lan-packet-bad-field",
    };
    i18n::text(id, &Args::new())
}

fn kick_text(reason: KickReason, ban_minutes: u32) -> String {
    let id = match reason {
        KickReason::Banned => return t!("lan-kick-banned", minutes = ban_minutes),
        KickReason::Kicked => "lan-kick-kicked",
        KickReason::Violations => "lan-kick-violations",
        KickReason::Idle => "lan-kick-idle",
        KickReason::Voted => "lan-kick-voted",
    };
    i18n::text(id, &Args::new())
}

/// A server's line in the browser: name, mode, players, then the flags.
pub fn server_line(entry: &ServerEntry) -> (String, String) {
    let mode = entry.mode.map_or_else(|| t!("lan-server-unknown-mode"), mode_name);
    let mut value = t!("lan-server-players", mode = mode, players = entry.players, max = entry.max_players);
    if let Some(ping) = entry.ping_ms {
        value.push_str(&format!("  {}", ping_text(u32::from(ping))));
    }
    if entry.password {
        value.push_str(&format!("  {}", t!("lan-server-locked")));
    }
    if entry.in_progress {
        value.push_str(&format!("  {}", t!("lan-server-racing")));
    }
    if !entry.compatible {
        value.push_str(&format!("  {}", t!("lan-server-other-version")));
    }
    (entry.name.clone(), value)
}

/// A round trip: `4 ms`.
pub fn ping_text(ms: u32) -> String {
    t!("lan-ping", ms = ms)
}

/// The name of `slot` in the lobby, or `slot N` when the roster has no row for it.
pub fn slot_name(lobby: Option<&LobbyView>, slot: u32) -> String {
    let row = u8::try_from(slot).ok().and_then(|slot| lobby?.player(slot));
    row.map_or_else(|| t!("lan-slot", slot = slot), |row| row.name.as_str().to_owned())
}

/// A system notice in the reader's words, naming players from the lobby.
pub fn notice_text(notice: &SystemNotice, lobby: Option<&LobbyView>) -> String {
    let arg = |i: usize| notice.args.get(i).copied();
    let who = || arg(0).map_or_else(|| t!("lan-someone"), |slot| slot_name(lobby, slot));
    match notice.code {
        NoticeCode::Join => t!("lan-notice-join", who = who()),
        NoticeCode::Leave => t!("lan-notice-leave", who = who()),
        NoticeCode::Kick => t!("lan-notice-kick", who = who()),
        NoticeCode::Mute if arg(1) == Some(0) => t!("lan-notice-unmute", who = who()),
        NoticeCode::Mute => t!("lan-notice-mute", who = who()),
        NoticeCode::Vote => match arg(1) {
            Some(passed) if passed != 0 => t!("lan-notice-vote-passed"),
            Some(_) => t!("lan-notice-vote-failed"),
            None => t!("lan-notice-vote-started"),
        },
        NoticeCode::Result => match arg(1) {
            Some(place) => t!("lan-notice-finished-place", who = who(), place = ordinal(place)),
            None => t!("lan-notice-finished", who = who()),
        },
    }
}

/// `1 vote`, `3 votes`.
pub fn votes(count: u8) -> String {
    t!("lan-votes", n = count)
}

/// A race time as `m:ss.mmm`; 0 reads as `-`. The format is the same in every language.
pub fn race_time(ms: u32) -> String {
    if ms == 0 {
        return "-".into();
    }
    format!("{}:{:02}.{:03}", ms / 60_000, ms / 1000 % 60, ms % 1000)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::frontend::lan::NetFrontend;
    use crate::frontend::lan::mock::MockNetFrontend;
    use nfsmw_net::message::{Kick, List};

    fn notice(code: NoticeCode, args: &[u32]) -> SystemNotice {
        SystemNotice { code, args: List::new(args.to_vec()).unwrap() }
    }

    #[test]
    fn notices_name_the_players_of_the_lobby() {
        let net = MockNetFrontend::in_lobby(&["Host", "Ann"], 0);
        let lobby = net.lobby();
        assert_eq!(notice_text(&notice(NoticeCode::Join, &[1]), lobby), "Ann joined");
        assert_eq!(notice_text(&notice(NoticeCode::Leave, &[7]), lobby), "slot 7 left");
        assert_eq!(notice_text(&notice(NoticeCode::Kick, &[1]), lobby), "Ann was kicked");
        assert_eq!(notice_text(&notice(NoticeCode::Mute, &[1, 0]), lobby), "Ann can chat again");
        assert_eq!(notice_text(&notice(NoticeCode::Vote, &[0, 1]), lobby), "the vote passed");
        assert_eq!(notice_text(&notice(NoticeCode::Result, &[0, 2]), lobby), "Host finished 2nd");
        assert_eq!(notice_text(&notice(NoticeCode::Join, &[]), None), "someone joined");
    }

    #[test]
    fn votes_and_times() {
        assert_eq!((votes(1), votes(3)), ("1 vote".to_owned(), "3 votes".to_owned()));
        assert_eq!(race_time(83_456), "1:23.456");
        assert_eq!(race_time(0), "-");
    }

    #[test]
    fn the_end_of_a_session_says_why() {
        assert_eq!(end_text(&EndReason::PasswordRejected), "wrong password");
        let kicked = EndReason::Kicked(Kick { reason: KickReason::Banned, ban_minutes: 10 });
        assert_eq!(end_text(&kicked), "you were banned for 10 min");
        let status = NetStatus::Disconnected(Disconnect::Failed("port 9750 is in use".into()));
        assert_eq!(status_text(&status), "disconnected: port 9750 is in use");
        let link = EndReason::Link(DisconnectReason::Handshake(HandshakeError::Rejected(1)));
        assert_eq!(end_text(&link), "the connection ended: handshake failed: host full");
    }

    #[test]
    fn a_server_line_shows_mode_players_and_flags() {
        let entry = ServerEntry {
            addr: "192.168.1.5:9750".parse().unwrap(),
            name: "Pietro's game".into(),
            mode: Some(Mode::Circuit),
            players: 3,
            max_players: 8,
            compatible: false,
            password: true,
            in_progress: true,
            ping_ms: Some(4),
        };
        let value = format!("{}  3/8  4 ms  locked  racing  other version", mode_name(Mode::Circuit));
        assert_eq!(server_line(&entry), ("Pietro's game".into(), value));
    }
}

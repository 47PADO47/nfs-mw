//! The words the LAN screens, the session HUD and the console share: modes, the status, why a session ended, a server's
//! line in the browser, a system notice and a race time. Plain English: the original has no strings for LAN play.

use nfsmw_net::client_session::{EndReason, LobbyView};
use nfsmw_net::message::{KickReason, Mode, NoticeCode, SystemNotice};
use nfsmw_net::version::RejectReason;

use super::backend::{Disconnect, NetStatus, ServerEntry};

/// The modes the Host screen offers, in its order. Drag and cops vs racers are refused by the lobby (ADR 0011).
pub const HOST_MODES: [Mode; 7] =
    [Mode::FreeRoam, Mode::Circuit, Mode::Sprint, Mode::Speedtrap, Mode::Knockout, Mode::PursuitCoop, Mode::Tag];

/// A mode's name.
pub fn mode_name(mode: Mode) -> String {
    let name = match mode {
        Mode::FreeRoam => "Free Roam",
        Mode::Circuit => "Circuit",
        Mode::Sprint => "Sprint",
        Mode::Drag => "Drag",
        Mode::Speedtrap => "Speedtrap",
        Mode::Knockout => "Lap Knockout",
        Mode::PursuitCoop => "Pursuit Co-op",
        Mode::CopsVsRacers => "Cops vs Racers",
        Mode::Tag => "Tag",
        Mode::Custom(id) => return format!("Mod mode {id}"),
    };
    name.to_owned()
}

/// The status as one line.
pub fn status_text(status: &NetStatus) -> String {
    match status {
        NetStatus::Idle => "not in a session".into(),
        NetStatus::Scanning => "looking for LAN games".into(),
        NetStatus::Connecting => "connecting".into(),
        NetStatus::InLobby => "in the lobby".into(),
        NetStatus::Playing => "playing".into(),
        NetStatus::Spectating => "spectating".into(),
        NetStatus::Disconnected(why) => format!("disconnected: {}", disconnect_text(why)),
    }
}

/// Why a session is over, as a sentence fragment.
pub fn disconnect_text(why: &Disconnect) -> String {
    match why {
        Disconnect::Failed(text) => text.clone(),
        Disconnect::Session(reason) => end_text(reason),
    }
}

/// Why a client session ended.
pub fn end_text(reason: &EndReason) -> String {
    match reason {
        EndReason::Left => "you left".into(),
        EndReason::Rejected(rejected) => format!("the server refused the seat ({})", reject_text(rejected.reason)),
        EndReason::Kicked(kick) => kick_text(kick.reason, kick.ban_minutes),
        EndReason::Link(link) => format!("the connection ended: {link}"),
        EndReason::ResumeExpired => "the connection was lost and could not be resumed".into(),
        EndReason::PasswordRequired => "the game needs a password".into(),
        EndReason::PasswordRejected => "wrong password".into(),
        EndReason::SchemaMismatch => "the server runs another game build".into(),
        EndReason::Protocol => "the server broke the protocol".into(),
    }
}

fn reject_text(reason: RejectReason) -> String {
    format!("{reason:?}").to_lowercase()
}

fn kick_text(reason: KickReason, ban_minutes: u32) -> String {
    match reason {
        KickReason::Kicked => "you were kicked".into(),
        KickReason::Banned => format!("you were banned for {ban_minutes} min"),
        KickReason::Violations => "removed for invalid messages".into(),
        KickReason::Idle => "removed for being idle".into(),
        KickReason::Voted => "voted out".into(),
    }
}

/// A server's line in the browser: name, mode, players, then the flags.
pub fn server_line(entry: &ServerEntry) -> (String, String) {
    let mode = entry.mode.map_or_else(|| "Unknown mode".to_owned(), mode_name);
    let mut value = format!("{mode}  {}/{}", entry.players, entry.max_players);
    if let Some(ping) = entry.ping_ms {
        value.push_str(&format!("  {ping} ms"));
    }
    if entry.password {
        value.push_str("  locked");
    }
    if entry.in_progress {
        value.push_str("  racing");
    }
    if !entry.compatible {
        value.push_str("  other version");
    }
    (entry.name.clone(), value)
}

/// The name of `slot` in the lobby, or `slot N` when the roster has no row for it.
pub fn slot_name(lobby: Option<&LobbyView>, slot: u32) -> String {
    let row = u8::try_from(slot).ok().and_then(|slot| lobby?.player(slot));
    row.map_or_else(|| format!("slot {slot}"), |row| row.name.as_str().to_owned())
}

/// A system notice in the reader's words, naming players from the lobby.
pub fn notice_text(notice: &SystemNotice, lobby: Option<&LobbyView>) -> String {
    let arg = |i: usize| notice.args.get(i).copied();
    let who = || arg(0).map_or_else(|| "someone".to_owned(), |slot| slot_name(lobby, slot));
    match notice.code {
        NoticeCode::Join => format!("{} joined", who()),
        NoticeCode::Leave => format!("{} left", who()),
        NoticeCode::Kick => format!("{} was kicked", who()),
        NoticeCode::Mute if arg(1) == Some(0) => format!("{} can chat again", who()),
        NoticeCode::Mute => format!("{} was muted", who()),
        NoticeCode::Vote => match arg(1) {
            Some(passed) if passed != 0 => "the vote passed".into(),
            Some(_) => "the vote failed".into(),
            None => "a vote started".into(),
        },
        NoticeCode::Result => match arg(1) {
            Some(place) => format!("{} finished {}", who(), ordinal(place)),
            None => format!("{} finished", who()),
        },
    }
}

/// `1st`, `2nd`, `3rd`, `4th`, `11th`, `21st`.
pub fn ordinal(n: u32) -> String {
    let suffix = match (n % 10, n % 100) {
        (_, 11..=13) => "th",
        (1, _) => "st",
        (2, _) => "nd",
        (3, _) => "rd",
        _ => "th",
    };
    format!("{n}{suffix}")
}

/// `1 vote`, `3 votes`.
pub fn votes(count: u8) -> String {
    match count {
        1 => "1 vote".into(),
        n => format!("{n} votes"),
    }
}

/// A race time as `m:ss.mmm`; 0 reads as `-`.
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
    fn ordinals_and_times() {
        let words: Vec<String> = [1, 2, 3, 4, 11, 12, 13, 21, 102].into_iter().map(ordinal).collect();
        assert_eq!(words, ["1st", "2nd", "3rd", "4th", "11th", "12th", "13th", "21st", "102nd"]);
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
        assert_eq!(
            server_line(&entry),
            ("Pietro's game".into(), "Circuit  3/8  4 ms  locked  racing  other version".into())
        );
    }
}

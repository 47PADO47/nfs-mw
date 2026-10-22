//! The session's feed (MP-40): chat lines and system notices (join, leave, kick, mute, vote, result) in the order they
//! came, with their age so the HUD can fade them. Chat from a player muted on this machine (MP-42, `LocalMutes`) is left
//! out; the server's mute stops it before it gets here.

use std::collections::VecDeque;

use nfsmw_net::client_session::{ChatLine, LobbyView, SessionEvent};
use nfsmw_net::message::ChatChannel;
use nfsmw_net::roles::LocalMutes;

use super::text::{end_text, notice_text, ordinal, slot_name};

/// Lines kept.
pub const KEEP: usize = 64;

/// What a line is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FeedKind {
    Chat,
    Notice,
}

#[derive(Clone, Debug, PartialEq)]
pub struct FeedLine {
    pub kind: FeedKind,
    pub text: String,
    /// Seconds since it came.
    pub age: f32,
}

/// The feed.
#[derive(Clone, Debug, Default)]
pub struct Feed {
    lines: VecDeque<FeedLine>,
}

impl Feed {
    pub fn push(&mut self, kind: FeedKind, text: String) {
        if self.lines.len() >= KEEP {
            self.lines.pop_front();
        }
        self.lines.push_back(FeedLine { kind, text, age: 0.0 });
    }

    /// Ages every line by `dt` seconds.
    pub fn tick(&mut self, dt: f32) {
        for line in &mut self.lines {
            line.age += dt;
        }
    }

    /// The newest `count` lines, oldest first.
    pub fn last(&self, count: usize) -> impl Iterator<Item = &FeedLine> {
        self.lines.iter().skip(self.lines.len().saturating_sub(count))
    }

    /// The lines younger than `seconds`, at most `count`, oldest first.
    pub fn recent(&self, seconds: f32, count: usize) -> Vec<&FeedLine> {
        let young: Vec<&FeedLine> = self.lines.iter().filter(|line| line.age < seconds).collect();
        young[young.len().saturating_sub(count)..].to_vec()
    }

    /// Adds the lines `event` makes, naming players from `lobby` and leaving out chat from `mutes`.
    pub fn take(&mut self, event: &SessionEvent, lobby: Option<&LobbyView>, mutes: &LocalMutes) {
        match event {
            SessionEvent::Chat(line) if shows(line, lobby, mutes) => self.push(FeedKind::Chat, chat_text(line, lobby)),
            SessionEvent::Notice(notice) => self.push(FeedKind::Notice, notice_text(notice, lobby)),
            SessionEvent::Ended(reason) => self.push(FeedKind::Notice, format!("Session over: {}", end_text(reason))),
            SessionEvent::ConnectionLost => self.push(FeedKind::Notice, "Connection lost, trying to resume".into()),
            SessionEvent::Admitted { resumed: true, .. } => self.push(FeedKind::Notice, "Connection resumed".into()),
            SessionEvent::Results(results) => {
                let winner = results.rows.iter().find(|row| row.place == 1);
                if let Some(row) = winner {
                    let name = slot_name(lobby, u32::from(row.slot));
                    self.push(FeedKind::Notice, format!("{name} won ({} of {})", ordinal(1), results.rows.len()));
                }
            }
            _ => {}
        }
    }
}

fn shows(line: &ChatLine, lobby: Option<&LobbyView>, mutes: &LocalMutes) -> bool {
    let Some(row) = lobby.and_then(|lobby| lobby.player(line.from_slot)) else {
        return true;
    };
    mutes.shows(row.player_id)
}

/// `Name: text`, or `Name (team): text` and `Name (whisper): text`.
pub fn chat_text(line: &ChatLine, lobby: Option<&LobbyView>) -> String {
    let name = slot_name(lobby, u32::from(line.from_slot));
    match line.channel {
        ChatChannel::All => format!("{name}: {}", line.text),
        ChatChannel::Team => format!("{name} (team): {}", line.text),
        ChatChannel::Whisper => format!("{name} (whisper): {}", line.text),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::frontend::lan::NetFrontend;
    use crate::frontend::lan::mock::MockNetFrontend;
    use nfsmw_net::client_session::EndReason;
    use nfsmw_net::message::{List, NoticeCode, SystemNotice};

    fn chat(slot: u8, text: &str) -> SessionEvent {
        SessionEvent::Chat(ChatLine { from_slot: slot, channel: ChatChannel::All, text: text.into(), tick: 0 })
    }

    #[test]
    fn chat_and_notices_name_players_and_muted_chat_is_left_out() {
        let net = MockNetFrontend::in_lobby(&["Host", "Ann", "Bob"], 0);
        let lobby = net.lobby();
        let mut mutes = LocalMutes::new();
        mutes.set([2; 16], true);
        let mut feed = Feed::default();
        feed.take(&chat(1, "hi"), lobby, &mutes);
        feed.take(&chat(2, "spam"), lobby, &mutes);
        let joined = SystemNotice { code: NoticeCode::Join, args: List::new(vec![2]).unwrap() };
        feed.take(&SessionEvent::Notice(joined), lobby, &mutes);
        feed.take(&SessionEvent::Ended(EndReason::Left), lobby, &mutes);
        feed.take(&SessionEvent::LobbyChanged, lobby, &mutes);
        let lines: Vec<&str> = feed.last(10).map(|line| line.text.as_str()).collect();
        assert_eq!(lines, ["Ann: hi", "Bob joined", "Session over: you left"]);
    }

    #[test]
    fn lines_age_and_old_ones_drop_off_the_recent_view() {
        let mut feed = Feed::default();
        feed.push(FeedKind::Notice, "old".into());
        feed.tick(9.0);
        feed.push(FeedKind::Chat, "new".into());
        feed.tick(1.0);
        let recent: Vec<&str> = feed.recent(8.0, 5).into_iter().map(|line| line.text.as_str()).collect();
        assert_eq!(recent, ["new"]);
        for n in 0..KEEP + 5 {
            feed.push(FeedKind::Chat, n.to_string());
        }
        assert_eq!(feed.last(1000).count(), KEEP);
        assert_eq!(feed.recent(1.0, 2).len(), 2);
    }
}

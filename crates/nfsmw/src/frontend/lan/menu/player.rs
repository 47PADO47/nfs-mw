//! The menu of one player of the lobby (MP-26, MP-27, MP-42): mute for me (local), a kick vote, and for the host or an
//! admin kick, server mute and a ten-minute ban.

use nfsmw_net::client_session::LobbyView;
use nfsmw_net::lobby::FLAG_MUTED;
use nfsmw_net::message::{AdminCommand, VoteKind};
use nfsmw_net::roles::LocalMutes;

use super::cx::Cx;
use super::lobby::may_manage;
use super::rows::{Cursor, Row};

/// The ban the menu offers, in minutes.
pub const BAN_MINUTES: u32 = 10;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Item {
    LocalMute,
    VoteKick,
    Kick,
    ServerMute,
    Ban,
    Back,
}

/// The player menu's state: whose it is and the selection.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct PlayerMenu {
    pub slot: u8,
    pub cursor: Cursor,
}

fn items(lobby: &LobbyView) -> Vec<Item> {
    let mut items = vec![Item::LocalMute, Item::VoteKick];
    if may_manage(lobby) {
        items.extend([Item::Kick, Item::ServerMute, Item::Ban]);
    }
    items.push(Item::Back);
    items
}

impl PlayerMenu {
    pub fn new(slot: u8) -> Self {
        Self { slot, cursor: Cursor::default() }
    }

    pub fn rows(&self, lobby: &LobbyView, mutes: &LocalMutes) -> Vec<Row> {
        let Some(row) = lobby.player(self.slot) else {
            return vec![Row::action("Back", true)];
        };
        let local = !mutes.shows(row.player_id);
        let server = row.flags & FLAG_MUTED != 0;
        items(lobby)
            .into_iter()
            .map(|item| match item {
                Item::LocalMute if local => Row::action("Unmute for me", true),
                Item::LocalMute => Row::action("Mute for me", true),
                Item::VoteKick => Row::action("Vote to kick", lobby.vote.is_none()),
                Item::Kick => Row::action("Kick", true),
                Item::ServerMute if server => Row::action("Unmute in session", true),
                Item::ServerMute => Row::action("Mute in session", true),
                Item::Ban => Row::action(format!("Ban for {BAN_MINUTES} minutes"), true),
                Item::Back => Row::action("Back", true),
            })
            .collect()
    }

    pub fn up(&mut self, lobby: &LobbyView) {
        self.cursor.up(items(lobby).len());
    }

    pub fn down(&mut self, lobby: &LobbyView) {
        self.cursor.down(items(lobby).len());
    }

    /// Accept: does the selected thing. `true` when the menu should close.
    pub fn accept(&mut self, cx: &mut Cx, mutes: &mut LocalMutes) -> bool {
        let Some(lobby) = cx.net.lobby().cloned() else { return true };
        let Some(row) = lobby.player(self.slot).cloned() else { return true };
        let items = items(&lobby);
        let slot = self.slot;
        match items[self.cursor.clamped(items.len())] {
            Item::LocalMute => {
                let muted = !mutes.shows(row.player_id);
                mutes.set(row.player_id, !muted);
                false
            }
            Item::VoteKick if lobby.vote.is_none() => cx.net.vote(VoteKind::Kick, u32::from(slot)),
            Item::VoteKick => false,
            Item::Kick => cx.net.admin(AdminCommand::Kick { slot }),
            Item::ServerMute => cx.net.admin(AdminCommand::Mute { slot, muted: row.flags & FLAG_MUTED == 0 }),
            Item::Ban => cx.net.admin(AdminCommand::Ban { slot, minutes: BAN_MINUTES }),
            Item::Back => true,
        }
    }
}

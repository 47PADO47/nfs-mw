//! The Lobby page (MP-21, 22, 27, 40, 41): the roster with car, ready state, host and admin badges and ping; ready, car,
//! role, chat and quick lines; the running vote and a restart vote; for the host or an admin the session settings and the
//! start; leave. The countdown shows in the subtitle.

use nfsmw_net::client_session::LobbyView;
use nfsmw_net::lobby::{FLAG_ADMIN, FLAG_HOST, FLAG_MUTED};
use nfsmw_net::message::{LobbyPlayer, Role, SessionPhase, VoteKind, VoteState};

use super::cx::{Cx, MenuOut};
use super::field::{Field, Typed};
use super::rows::{Cursor, Row, RowKind};
use crate::frontend::lan::car::car_name;
use crate::frontend::lan::quick::{QUICK_LINES, quick_line};
use crate::frontend::lan::text::{mode_name, slot_name, votes};

/// Where Accept on the lobby goes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Go {
    Stay,
    /// The player menu of a slot.
    Player(u8),
    Settings,
    /// The session was left.
    Left,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Item {
    Player(u8),
    Ready,
    Car,
    Role,
    Chat,
    Quick,
    VoteFor(u32),
    VoteRestart,
    Settings,
    Start,
    Leave,
}

/// The lobby page's own state: the selection, the chat line being typed and the quick line chosen.
#[derive(Clone, Debug, PartialEq)]
pub struct LobbyPage {
    pub cursor: Cursor,
    pub chat: Field,
    pub quick: u16,
}

impl Default for LobbyPage {
    fn default() -> Self {
        Self { cursor: Cursor::default(), chat: Field::new("", 200), quick: 0 }
    }
}

/// Whether this client may change settings and start: the host or an admin.
pub fn may_manage(lobby: &LobbyView) -> bool {
    lobby.is_host() || lobby.me().is_some_and(|me| me.flags & FLAG_ADMIN != 0)
}

/// Whether every racer is ready.
pub fn all_ready(lobby: &LobbyView) -> bool {
    lobby.players.iter().filter(|row| row.role != Role::Spectator).all(|row| row.ready)
}

fn items(lobby: &LobbyView) -> Vec<Item> {
    let mut items: Vec<Item> = lobby.players.iter().map(|row| Item::Player(row.slot)).collect();
    if lobby.my_role() != Role::Spectator {
        items.push(Item::Ready);
    }
    items.extend([Item::Car, Item::Role, Item::Chat, Item::Quick]);
    match &lobby.vote {
        Some(vote) => items.extend(vote.options.iter().map(|option| Item::VoteFor(option.id))),
        None => items.push(Item::VoteRestart),
    }
    if may_manage(lobby) {
        items.extend([Item::Settings, Item::Start]);
    }
    items.push(Item::Leave);
    items
}

impl LobbyPage {
    fn item(&self, lobby: &LobbyView) -> Option<Item> {
        let items = items(lobby);
        items.get(self.cursor.clamped(items.len())).copied()
    }

    /// Whether the chat row has the keys.
    pub fn typing(&self) -> bool {
        self.chat.editing
    }

    pub fn rows(&self, lobby: &LobbyView, cars: &[String], my_car: &str) -> Vec<Row> {
        items(lobby).into_iter().map(|item| self.row(item, lobby, cars, my_car)).collect()
    }

    fn row(&self, item: Item, lobby: &LobbyView, cars: &[String], my_car: &str) -> Row {
        let me = lobby.me();
        match item {
            Item::Player(slot) => match lobby.player(slot) {
                Some(row) => player_row(row, lobby, cars, my_car),
                None => Row::info(format!("slot {slot}"), ""),
            },
            Item::Ready => Row::choice("Ready", yes_no(me.is_some_and(|me| me.ready)), true),
            Item::Car => Row { label: "Car".into(), value: my_car.into(), enabled: true, kind: RowKind::Action },
            Item::Role => Row::choice("Role", role_name(lobby.my_role()), true),
            Item::Chat if self.chat.editing => Row::text("Say", self.chat.shown(), true),
            Item::Chat => Row::text("Say", "", false),
            Item::Quick => Row::choice("Quick message", quick_line(self.quick).unwrap_or_default(), true),
            Item::VoteFor(option) => {
                let vote = lobby.vote.as_ref();
                let count = vote.and_then(|vote| vote.options.iter().find(|o| o.id == option)).map_or(0, |o| o.count);
                let label = vote.map_or_else(String::new, |vote| vote_label(vote, option, lobby));
                Row { label, value: votes(count), enabled: true, kind: RowKind::Action }
            }
            Item::VoteRestart => Row::action("Vote to restart", lobby.phase != SessionPhase::Lobby),
            Item::Settings => Row::action("Session settings", true),
            Item::Start => Row::action("Start race", lobby.phase == SessionPhase::Lobby && all_ready(lobby)),
            Item::Leave => Row::action("Leave", true),
        }
    }

    pub fn up(&mut self, lobby: &LobbyView) {
        self.cursor.up(items(lobby).len());
    }

    pub fn down(&mut self, lobby: &LobbyView) {
        self.cursor.down(items(lobby).len());
    }

    pub fn take(&mut self, key: Typed) {
        self.chat.take(key);
    }

    /// Back while typing ends the typing; `false` when there was none (the page then asks to leave).
    pub fn back(&mut self) -> bool {
        let was = self.chat.editing;
        self.chat.editing = false;
        was
    }

    /// Left or right on the selected row.
    pub fn step(&mut self, cx: &mut Cx, forward: bool) {
        let Some(lobby) = cx.net.lobby().cloned() else { return };
        match self.item(&lobby) {
            Some(Item::Ready) => {
                let ready = lobby.me().is_some_and(|me| me.ready);
                cx.net.set_ready(!ready);
            }
            Some(Item::Role) => {
                cx.net.set_role(other_role(lobby.my_role()));
            }
            Some(Item::Quick) => {
                let len = QUICK_LINES.len() as u16;
                self.quick = match forward {
                    true => (self.quick + 1) % len,
                    false => (self.quick + len - 1) % len,
                };
            }
            _ => {}
        }
    }

    /// Accept on the selected row.
    pub fn accept(&mut self, cx: &mut Cx) -> Go {
        let Some(lobby) = cx.net.lobby().cloned() else { return Go::Stay };
        let Some(item) = self.item(&lobby) else { return Go::Stay };
        match item {
            Item::Player(slot) if Some(slot) != lobby.my_slot => return Go::Player(slot),
            Item::Player(_) => {}
            Item::Ready | Item::Role => self.step(cx, true),
            Item::Car => cx.out.push(MenuOut::CarSelect),
            Item::Chat => self.send_chat(cx),
            Item::Quick => {
                cx.net.quick_message(self.quick);
            }
            Item::VoteFor(option) => {
                let Some(vote) = &lobby.vote else { return Go::Stay };
                cx.net.vote(vote.kind, option);
            }
            Item::VoteRestart if lobby.phase != SessionPhase::Lobby => {
                cx.net.vote(VoteKind::Restart, 0);
            }
            Item::VoteRestart => {}
            Item::Settings => return Go::Settings,
            Item::Start if lobby.phase == SessionPhase::Lobby && all_ready(&lobby) => {
                cx.net.start_race();
            }
            Item::Start => {}
            Item::Leave => {
                cx.net.leave();
                return Go::Left;
            }
        }
        Go::Stay
    }

    /// Accept on the chat row: starts typing, or sends the line and stops.
    fn send_chat(&mut self, cx: &mut Cx) {
        if !self.chat.editing {
            self.chat.editing = true;
            return;
        }
        self.chat.editing = false;
        let line = self.chat.value().to_owned();
        self.chat.text.clear();
        if line.is_empty() {
            return;
        }
        if let Err(e) = cx.net.say(&line) {
            log::info!("LAN chat line refused: {e:?}");
        }
    }
}

fn player_row(row: &LobbyPlayer, lobby: &LobbyView, cars: &[String], my_car: &str) -> Row {
    let mut label = row.name.as_str().to_owned();
    if row.flags & FLAG_HOST != 0 || row.slot == lobby.host_slot {
        label.push_str("  [host]");
    }
    if row.flags & FLAG_ADMIN != 0 {
        label.push_str("  [admin]");
    }
    if row.flags & FLAG_MUTED != 0 {
        label.push_str("  [muted]");
    }
    if Some(row.slot) == lobby.my_slot {
        label.push_str("  (you)");
    }
    let state = match (row.role, row.ready) {
        (Role::Spectator, _) => "watching",
        (_, true) => "ready",
        (_, false) => "not ready",
    };
    let car = car_name(row.car_type, cars.iter().map(String::as_str).chain([my_car]));
    Row::info(label, format!("{car}  {state}  {} ms", row.ping_ms))
}

fn vote_label(vote: &VoteState, option: u32, lobby: &LobbyView) -> String {
    match vote.kind {
        VoteKind::Kick => format!("Vote: kick {}", slot_name(Some(lobby), option)),
        VoteKind::Restart => "Vote: restart".into(),
        VoteKind::ChangeMode => {
            let mode = u8::try_from(option).ok().and_then(nfsmw_net::message::Mode::from_code);
            format!("Vote: mode {}", mode.map_or_else(|| option.to_string(), mode_name))
        }
        VoteKind::ChangeTrack => format!("Vote: track {option}"),
        VoteKind::SkipResults => "Vote: skip results".into(),
    }
}

fn yes_no(on: bool) -> &'static str {
    match on {
        true => "Yes",
        false => "No",
    }
}

fn role_name(role: Role) -> &'static str {
    match role {
        Role::Racer => "Racer",
        Role::Cop => "Cop",
        Role::Spectator => "Spectator",
    }
}

fn other_role(role: Role) -> Role {
    match role {
        Role::Spectator => Role::Racer,
        _ => Role::Spectator,
    }
}

/// The countdown line: `Race starts in 3` while one runs at `server_tick`.
pub fn countdown_text(lobby: &LobbyView, server_tick: Option<f64>) -> Option<String> {
    let countdown = lobby.countdown.as_ref()?;
    let Some(now) = server_tick else {
        return Some(format!("Race starts in {}", countdown.seconds));
    };
    let left = ((f64::from(countdown.start_tick) - now) / 60.0).ceil().max(0.0);
    Some(format!("Race starts in {left:.0}"))
}

#[cfg(test)]
#[path = "lobby_tests.rs"]
mod tests;

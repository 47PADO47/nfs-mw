//! Each page as a [`PageView`]: the title, the status line, the rows, the side column and the footer.

use nfsmw_net::client_session::LobbyView;

use super::cx::Cx;
use super::lobby::countdown_text;
use super::rows::{PageView, Row};
use super::{LanMenu, Page};
use crate::frontend::lan::results;
use crate::frontend::lan::text::{mode_name, status_text};

/// The chat lines the lobby's side column shows.
const SIDE_LINES: usize = 12;

const MENU_HINTS: [(&str, &str); 3] = [("Enter", "Select"), ("Esc", "Back"), ("Arrows", "Move")];
const FORM_HINTS: [(&str, &str); 3] = [("Enter", "Select / type"), ("Left/Right", "Change"), ("Esc", "Back")];
const LOBBY_HINTS: [(&str, &str); 3] = [("Enter", "Select / say"), ("Left/Right", "Change"), ("Esc", "Stop typing")];

impl LanMenu {
    /// The page as it is drawn now.
    pub fn view(&self, cx: &Cx) -> PageView {
        let mut view = match self.page {
            Page::Home => self.home_view(cx),
            Page::Host => PageView {
                title: "Host a LAN Game".into(),
                subtitle: format!("Port {}  as {}", cx.settings.net.port, cx.settings.net.name),
                rows: self.host.rows(),
                selected: self.host.cursor.clamped(self.host.rows().len()),
                hints: FORM_HINTS.to_vec(),
                ..PageView::default()
            },
            Page::Browser => PageView {
                title: "Join a LAN Game".into(),
                subtitle: browser_subtitle(cx),
                rows: self.browser.rows(cx),
                selected: self.browser.selected(cx),
                hints: MENU_HINTS.to_vec(),
                ..PageView::default()
            },
            Page::Lobby | Page::Settings | Page::Player => self.session_view(cx),
            Page::Results => self.results_view(cx),
        };
        view.error = self.error.clone();
        view
    }

    fn home_view(&self, cx: &Cx) -> PageView {
        PageView {
            title: "LAN Game".into(),
            subtitle: format!("Playing as {}", cx.settings.net.name),
            rows: vec![Row::action("Host a Game", true), Row::action("Join a Game", true), Row::action("Back", true)],
            selected: self.home.clamped(3),
            hints: MENU_HINTS.to_vec(),
            ..PageView::default()
        }
    }

    fn session_view(&self, cx: &Cx) -> PageView {
        let Some(lobby) = cx.net.lobby() else {
            return PageView { title: "Lobby".into(), subtitle: status_text(&cx.net.status()), ..PageView::default() };
        };
        let (title, rows, selected, hints) = match self.page {
            Page::Settings if self.settings_form.is_some() => {
                let form = self.settings_form.as_ref().map(|form| (form.rows(), form.cursor)).unwrap_or_default();
                ("Session Settings".to_owned(), form.0.clone(), form.1.clamped(form.0.len()), FORM_HINTS.to_vec())
            }
            Page::Player => {
                let rows = self.player.rows(lobby, &self.mutes);
                let name = lobby.player(self.player.slot).map_or_else(String::new, |row| row.name.as_str().to_owned());
                let at = self.player.cursor.clamped(rows.len());
                (name, rows, at, MENU_HINTS.to_vec())
            }
            _ => {
                let rows = self.lobby.rows(lobby, cx.cars, &self.car);
                let at = self.lobby.cursor.clamped(rows.len());
                (room_title(lobby), rows, at, LOBBY_HINTS.to_vec())
            }
        };
        PageView {
            title,
            subtitle: lobby_subtitle(lobby, cx),
            rows,
            selected,
            side_title: "Chat".into(),
            side: self.feed.last(SIDE_LINES).map(|line| line.text.clone()).collect(),
            hints,
            ..PageView::default()
        }
    }

    fn results_view(&self, cx: &Cx) -> PageView {
        let mut rows = self.results.as_ref().map(|r| results::rows(r, cx.net.lobby())).unwrap_or_default();
        rows.push(Row::action("Continue", true));
        PageView {
            title: "Race Results".into(),
            subtitle: cx.net.lobby().map(room_title).unwrap_or_default(),
            selected: self.results_cursor.clamped(rows.len()),
            rows,
            hints: vec![("Enter", "Continue")],
            ..PageView::default()
        }
    }
}

fn room_title(lobby: &LobbyView) -> String {
    lobby.settings.as_ref().map_or_else(|| "Lobby".to_owned(), |settings| settings.name.as_str().to_owned())
}

/// The lobby's status line: the mode and the track, the head count, and the countdown or the phase.
fn lobby_subtitle(lobby: &LobbyView, cx: &Cx) -> String {
    let mode = lobby.settings.as_ref().map_or_else(String::new, |settings| mode_name(settings.mode));
    let max = lobby.settings.as_ref().map_or(0, |settings| settings.max_players);
    let mut line = format!("{mode}  {}/{max} players", lobby.players.len());
    match countdown_text(lobby, cx.net.lead().server_tick) {
        Some(countdown) => line.push_str(&format!("  {countdown}")),
        None => line.push_str(&format!("  {:?}", lobby.phase)),
    }
    line
}

fn browser_subtitle(cx: &Cx) -> String {
    let found = cx.net.servers().len();
    let status = status_text(&cx.net.status());
    match found {
        0 => format!("No games found yet  ({status})"),
        1 => format!("1 game found  ({status})"),
        n => format!("{n} games found  ({status})"),
    }
}

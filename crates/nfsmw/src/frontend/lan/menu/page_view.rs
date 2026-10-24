//! Each page as a [`PageView`]: the title, the status line, the rows, the side column and the footer.

use nfsmw_net::client_session::LobbyView;

use super::cx::Cx;
use super::lobby::countdown_text;
use super::rows::{HintDoes, HintKey, PageView, Row};
use super::{LanMenu, Page};
use crate::frontend::lan::results;
use crate::frontend::lan::text::{game, label, mode_name, phase_name, status_text};
use crate::i18n::t;

/// The chat lines the lobby's side column shows.
const SIDE_LINES: usize = 12;

const MENU_HINTS: [(HintKey, HintDoes); 3] =
    [(HintKey::Enter, HintDoes::Select), (HintKey::Esc, HintDoes::Back), (HintKey::Arrows, HintDoes::Move)];
const FORM_HINTS: [(HintKey, HintDoes); 3] =
    [(HintKey::Enter, HintDoes::SelectOrType), (HintKey::LeftRight, HintDoes::Change), (HintKey::Esc, HintDoes::Back)];
const LOBBY_HINTS: [(HintKey, HintDoes); 3] = [
    (HintKey::Enter, HintDoes::SelectOrSay),
    (HintKey::LeftRight, HintDoes::Change),
    (HintKey::Esc, HintDoes::StopTyping),
];

impl LanMenu {
    /// The page as it is drawn now.
    pub fn view(&self, cx: &Cx) -> PageView {
        let mut view = match self.page {
            Page::Home => self.home_view(cx),
            Page::Host => PageView {
                title: t!("lan-title-host"),
                subtitle: t!("lan-host-subtitle", port = cx.settings.net.port, name = cx.settings.net.name.as_str()),
                rows: self.host.rows(),
                selected: self.host.cursor.clamped(self.host.rows().len()),
                hints: FORM_HINTS.to_vec(),
                ..PageView::default()
            },
            Page::Browser => PageView {
                title: t!("lan-title-join"),
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
            title: t!("lan-title-home"),
            subtitle: t!("lan-home-subtitle", name = cx.settings.net.name.as_str()),
            rows: vec![
                Row::action(t!("lan-home-host"), true),
                Row::action(t!("lan-home-join"), true),
                match self.name.editing {
                    true => Row::text(t!("lan-home-name"), self.name.shown(), true),
                    false => Row::text(t!("lan-home-name"), cx.settings.net.name.as_str(), false),
                },
                Row::action(game(label::BACK), true),
            ],
            selected: self.home.clamped(super::HOME_ROWS),
            hints: MENU_HINTS.to_vec(),
            ..PageView::default()
        }
    }

    fn session_view(&self, cx: &Cx) -> PageView {
        let Some(lobby) = cx.net.lobby() else {
            return PageView {
                title: game(label::LOBBY),
                subtitle: status_text(&cx.net.status()),
                ..PageView::default()
            };
        };
        let (title, rows, selected, hints) = match self.page {
            Page::Settings if self.settings_form.is_some() => {
                let form = self.settings_form.as_ref().map(|form| (form.rows(), form.cursor)).unwrap_or_default();
                (t!("lan-title-settings"), form.0.clone(), form.1.clamped(form.0.len()), FORM_HINTS.to_vec())
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
            side_title: game(label::CHAT),
            side: self.feed.last(SIDE_LINES).map(|line| line.text.clone()).collect(),
            hints,
            ..PageView::default()
        }
    }

    fn results_view(&self, cx: &Cx) -> PageView {
        let mut rows = self.results.as_ref().map(|r| results::rows(r, cx.net.lobby())).unwrap_or_default();
        rows.push(Row::action(game(label::CONTINUE), true));
        PageView {
            title: t!("lan-title-results"),
            subtitle: cx.net.lobby().map(room_title).unwrap_or_default(),
            selected: self.results_cursor.clamped(rows.len()),
            rows,
            hints: vec![(HintKey::Enter, HintDoes::Continue)],
            ..PageView::default()
        }
    }
}

/// The room's name (the player's own words), or the game's word for a lobby before the settings came.
fn room_title(lobby: &LobbyView) -> String {
    lobby.settings.as_ref().map_or_else(|| game(label::LOBBY), |settings| settings.name.as_str().to_owned())
}

/// The lobby's status line: the mode and the track, the head count, and the countdown or the phase.
fn lobby_subtitle(lobby: &LobbyView, cx: &Cx) -> String {
    let mode = lobby.settings.as_ref().map_or_else(String::new, |settings| mode_name(settings.mode));
    let max = lobby.settings.as_ref().map_or(0, |settings| settings.max_players);
    let mut line = t!("lan-lobby-players", mode = mode, players = lobby.players.len(), max = max);
    match countdown_text(lobby, cx.net.lead().server_tick) {
        Some(countdown) => line.push_str(&format!("  {countdown}")),
        None => line.push_str(&format!("  {}", phase_name(lobby.phase))),
    }
    line
}

fn browser_subtitle(cx: &Cx) -> String {
    t!("lan-browser-found", n = cx.net.servers().len(), status = status_text(&cx.net.status()))
}

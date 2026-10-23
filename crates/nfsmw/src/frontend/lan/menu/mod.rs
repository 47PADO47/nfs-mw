//! The LAN menu: Home (host, join), the Host form, the Join browser with its password prompt, the Lobby with its player
//! menu and session settings, and the results of the last race. Pad-driven like the FEng menus (a `--ui-script` drives
//! it), with clicks on rows; drawn by `view.rs`. The original has no LAN screens to run, so this is the egui panel layer
//! (`docs/specs/multiplayer/frontend.md`).
//!
//! - `cx.rs`: the keys, the context of a key or a frame, and what the menu asks of the front end.
//! - `rows.rs`, `field.rs`: the rows and text fields the pages are made of.
//! - `host_form.rs`, `browser.rs`, `lobby.rs`, `player.rs`: the pages.
//! - `page_view.rs`: each page as a [`rows::PageView`].
//! - `follow.rs`: the page follows the session (seated, ended, results).

mod browser;
mod cx;
mod field;
mod follow;
mod host_form;
mod lobby;
mod page_view;
mod player;
pub mod rows;

pub use cx::{Cx, LanKey, MenuOut};
pub use field::Typed;
pub use host_form::{TrackChoice, city};
pub use lobby::countdown_text;

use nfsmw_net::message::RaceResults;
use nfsmw_net::roles::LocalMutes;

use super::NetStatus;
use super::feed::Feed;
use super::host_options::{default_room, host_options};
use browser::{Browser, BrowserGo};
use host_form::{FormOut, HostForm};
use lobby::{Go, LobbyPage};
use player::PlayerMenu;
use rows::Cursor;

/// The pages.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Page {
    Home,
    Host,
    Browser,
    Lobby,
    Settings,
    Player,
    Results,
}

/// The LAN menu's state.
#[derive(Debug)]
pub struct LanMenu {
    /// The menu has the screen (opened from the main menu's LAN icon or by a session).
    pub open: bool,
    page: Page,
    home: Cursor,
    host: HostForm,
    browser: Browser,
    lobby: LobbyPage,
    settings_form: Option<HostForm>,
    player: PlayerMenu,
    /// The last race's results, shown on the results page and kept for the HUD.
    pub results: Option<RaceResults>,
    results_cursor: Cursor,
    /// The chat and notices of the session, also drawn by the session HUD.
    pub feed: Feed,
    /// The players muted on this machine (MP-42).
    pub mutes: LocalMutes,
    error: Option<String>,
    /// The car this player drives in a session (a folder name).
    pub car: String,
    tracks: Vec<TrackChoice>,
    last_status: NetStatus,
}

impl Default for LanMenu {
    fn default() -> Self {
        Self {
            open: false,
            page: Page::Home,
            home: Cursor::default(),
            host: HostForm::new("", vec![city()]),
            browser: Browser::default(),
            lobby: LobbyPage::default(),
            settings_form: None,
            player: PlayerMenu::default(),
            results: None,
            results_cursor: Cursor::default(),
            feed: Feed::default(),
            mutes: LocalMutes::new(),
            error: None,
            car: crate::scenes::world::DEFAULT_CAR.to_owned(),
            tracks: vec![city()],
            last_status: NetStatus::Idle,
        }
    }
}

impl LanMenu {
    #[cfg(test)]
    pub fn page(&self) -> Page {
        self.page
    }

    /// The tracks the Host form offers (the whole city and the races of the track list).
    pub fn set_tracks(&mut self, tracks: Vec<TrackChoice>) {
        self.tracks = tracks;
    }

    /// Opens the menu: the lobby when a seat is held, else Home.
    pub fn open(&mut self, status: &NetStatus) {
        self.open = true;
        self.error = None;
        self.page = match status.is_seated() {
            true => Page::Lobby,
            false => Page::Home,
        };
    }

    /// Whether the menu is on screen now: open, and not hidden behind the session's world (the results show over it).
    pub fn visible(&self, status: &NetStatus) -> bool {
        if !self.open {
            return false;
        }
        self.page == Page::Results || !matches!(status, NetStatus::Playing | NetStatus::Spectating)
    }

    /// Whether a text row has the keys: the letters are text then, not menu buttons.
    pub fn typing(&self) -> bool {
        match self.page {
            Page::Host => self.host.typing(),
            Page::Browser => self.browser.typing(),
            Page::Lobby => self.lobby.typing(),
            Page::Settings => self.settings_form.as_ref().is_some_and(HostForm::typing),
            Page::Player => self.player.typing(),
            _ => false,
        }
    }

    /// The car select chose `folder`: it is this player's car from now on, and the lobby's if a seat is held.
    pub fn car_chosen(&mut self, folder: &str, cx: &mut Cx) {
        self.car = folder.to_owned();
        if cx.net.status().is_seated() {
            cx.net.choose_loadout(nfsmw_net::loadout::Loadout::stock(super::car::car_type(folder)));
        }
    }

    /// One key.
    pub fn key(&mut self, key: LanKey, cx: &mut Cx) {
        if let LanKey::Typed(typed) = key {
            return self.typed(typed);
        }
        if let LanKey::Click(row) = key {
            self.select(row, cx);
            return self.key(LanKey::Accept, cx);
        }
        if let LanKey::Step(row, forward) = key {
            self.select(row, cx);
            let way = match forward {
                true => LanKey::Right,
                false => LanKey::Left,
            };
            return self.key(way, cx);
        }
        self.error = None;
        match self.page {
            Page::Home => self.home_key(key, cx),
            Page::Host => self.host_key(key, cx),
            Page::Browser => self.browser_key(key, cx),
            Page::Lobby => self.lobby_key(key, cx),
            Page::Settings => self.settings_key(key, cx),
            Page::Player => self.player_key(key, cx),
            Page::Results => self.results_key(key, cx),
        }
    }

    fn typed(&mut self, typed: Typed) {
        match self.page {
            Page::Host => self.host.take(typed),
            Page::Browser => self.browser.take(typed),
            Page::Lobby => self.lobby.take(typed),
            Page::Settings => self.settings_form.iter_mut().for_each(|form| form.take(typed)),
            Page::Player => self.player.take(typed),
            _ => {}
        }
    }

    /// Moves the selection to `row` by pressing Down from the first row, as the keys would.
    fn select(&mut self, row: usize, cx: &mut Cx) {
        let len = self.view(cx).rows.len();
        if row >= len {
            return;
        }
        for _ in 0..len {
            if self.view(cx).selected == row {
                return;
            }
            self.key(LanKey::Down, cx);
        }
    }

    fn home_key(&mut self, key: LanKey, cx: &mut Cx) {
        match key {
            LanKey::Up => self.home.up(3),
            LanKey::Down => self.home.down(3),
            LanKey::Back => self.close(cx),
            LanKey::Accept => match self.home.clamped(3) {
                0 => {
                    let room = default_room(cx.settings.net.name.as_str());
                    self.host = HostForm::new(&room, self.tracks.clone());
                    self.page = Page::Host;
                }
                1 => {
                    self.browser = Browser::default();
                    cx.net.scan();
                    self.page = Page::Browser;
                }
                _ => self.close(cx),
            },
            _ => {}
        }
    }

    fn close(&mut self, cx: &mut Cx) {
        self.open = false;
        cx.out.push(MenuOut::Close);
    }

    fn host_key(&mut self, key: LanKey, cx: &mut Cx) {
        let out = match key {
            LanKey::Up => return self.host.up(),
            LanKey::Down => return self.host.down(),
            LanKey::Left => return self.host.step(false),
            LanKey::Right => return self.host.step(true),
            LanKey::Accept => self.host.accept(),
            LanKey::Back => self.host.back(),
            _ => None,
        };
        match out {
            Some(FormOut::Back) => self.page = Page::Home,
            Some(FormOut::Submit) => self.submit_host(cx),
            None => {}
        }
    }

    fn submit_host(&mut self, cx: &mut Cx) {
        let base = match super::host_options::default_settings(self.host.room.value()) {
            Ok(base) => base,
            Err(e) => return self.error = Some(e),
        };
        let settings = self.host.settings(&base);
        let options = match host_options(&cx.settings.net, settings, self.host.password.value(), &self.car) {
            Ok(options) => options,
            Err(e) => return self.error = Some(e),
        };
        if let Err(e) = cx.net.host(options) {
            self.error = Some(e);
        }
    }

    fn browser_key(&mut self, key: LanKey, cx: &mut Cx) {
        let go = match key {
            LanKey::Up => return self.browser.up(cx),
            LanKey::Down => return self.browser.down(cx),
            LanKey::Accept => self.browser.accept(cx),
            LanKey::Back => self.browser.back(),
            _ => BrowserGo::Stay,
        };
        match go {
            BrowserGo::Back => self.page = Page::Home,
            BrowserGo::Error(e) => self.error = Some(e),
            BrowserGo::Stay => {}
        }
    }

    fn lobby_key(&mut self, key: LanKey, cx: &mut Cx) {
        let Some(lobby) = cx.net.lobby().cloned() else { return };
        let go = match key {
            LanKey::Up => return self.lobby.up(&lobby),
            LanKey::Down => return self.lobby.down(&lobby),
            LanKey::Left => return self.lobby.step(cx, false),
            LanKey::Right => return self.lobby.step(cx, true),
            LanKey::Accept => self.lobby.accept(cx),
            // Back in the lobby only ends typing: leaving is the Leave row, so a stray Back keeps the seat.
            LanKey::Back => {
                self.lobby.back();
                Go::Stay
            }
            _ => Go::Stay,
        };
        match go {
            Go::Player(slot) => {
                self.player = PlayerMenu::new(slot);
                self.page = Page::Player;
            }
            Go::Settings => {
                let settings =
                    lobby.settings.clone().unwrap_or_else(|| nfsmw_net::message::Settings::lan(Default::default()));
                self.settings_form = Some(HostForm::from_settings(&settings, self.tracks.clone()));
                self.page = Page::Settings;
            }
            Go::Left => self.page = Page::Home,
            Go::Stay => {}
        }
    }

    fn settings_key(&mut self, key: LanKey, cx: &mut Cx) {
        let Some(form) = self.settings_form.as_mut() else {
            self.page = Page::Lobby;
            return;
        };
        let out = match key {
            LanKey::Up => return form.up(),
            LanKey::Down => return form.down(),
            LanKey::Left => return form.step(false),
            LanKey::Right => return form.step(true),
            LanKey::Accept => form.accept(),
            LanKey::Back => form.back(),
            _ => None,
        };
        match out {
            Some(FormOut::Back) => self.page = Page::Lobby,
            Some(FormOut::Submit) => {
                let old = cx.net.lobby().and_then(|lobby| lobby.settings.clone());
                if let Some(old) = old {
                    cx.net.update_settings(form.patch(&old));
                }
                self.page = Page::Lobby;
            }
            None => {}
        }
    }

    fn player_key(&mut self, key: LanKey, cx: &mut Cx) {
        let Some(lobby) = cx.net.lobby().cloned() else {
            self.page = Page::Lobby;
            return;
        };
        match key {
            LanKey::Up => self.player.up(&lobby),
            LanKey::Down => self.player.down(&lobby),
            // Back while typing a whisper only ends the typing.
            LanKey::Back if self.player.back() => {}
            LanKey::Back => self.page = Page::Lobby,
            LanKey::Accept if self.player.accept(cx, &mut self.mutes) => self.page = Page::Lobby,
            _ => {}
        }
    }

    fn results_key(&mut self, key: LanKey, cx: &mut Cx) {
        let len = self.results.as_ref().map_or(0, |results| results.rows.len()) + 1;
        match key {
            LanKey::Up => self.results_cursor.up(len),
            LanKey::Down => self.results_cursor.down(len),
            LanKey::Accept | LanKey::Back => {
                self.page = match cx.net.status().is_seated() {
                    true => Page::Lobby,
                    false => Page::Home,
                };
            }
            _ => {}
        }
    }
}

#[cfg(test)]
#[path = "tests.rs"]
mod tests;

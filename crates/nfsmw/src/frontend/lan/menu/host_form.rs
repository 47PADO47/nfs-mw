//! The Host page and the lobby's Settings page (MP-25): room name, mode, track, laps, traffic density, police cap, player
//! limit and password over the LAN defaults of `nfsmw_net::settings`. The same form builds a host's options and, for a
//! host or admin in the lobby, a `SettingsPatch` of what changed.

use nfsmw_net::message::{
    MAX_COP_LEVEL, MAX_LAPS, MAX_TRAFFIC_DENSITY, Mode, PLAYERS_RANGE, ServerName, Settings, SettingsPatch,
};
use nfsmw_net::settings::{Field as SettingsField, has_laps};

use super::field::{Field, Typed};
use super::rows::{Cursor, Row, cycle, nudge};
use crate::frontend::lan::host_options::field_name;
use crate::frontend::lan::text::{HOST_MODES, game, label, mode_name};
use crate::i18n::fmt::percent;
use crate::i18n::t;

/// One track a session can run: its id on the wire (0 is the whole city), its title and the mode it is for (`None`: the
/// free modes).
#[derive(Clone, Debug, PartialEq)]
pub struct TrackChoice {
    pub id: u32,
    pub title: String,
    pub mode: Option<Mode>,
}

/// The whole city, the track of the free modes (free roam, pursuit co-op, tag).
pub fn city() -> TrackChoice {
    TrackChoice { id: 0, title: t!("lan-track-city"), mode: None }
}

/// The rows of the form, in order.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Item {
    Room,
    Mode,
    Track,
    Laps,
    Traffic,
    Cops,
    Players,
    Password,
    Submit,
    Back,
}

const ITEMS: [Item; 10] = [
    Item::Room,
    Item::Mode,
    Item::Track,
    Item::Laps,
    Item::Traffic,
    Item::Cops,
    Item::Players,
    Item::Password,
    Item::Submit,
    Item::Back,
];

/// What Accept on the form asks for.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FormOut {
    Submit,
    Back,
}

/// The form's values.
#[derive(Clone, Debug, PartialEq)]
pub struct HostForm {
    pub room: Field,
    pub mode: Mode,
    pub track: u32,
    pub laps: u8,
    pub traffic: u8,
    pub cops: u8,
    pub players: u8,
    pub password: Field,
    /// Every track; the form offers the ones of the mode.
    pub tracks: Vec<TrackChoice>,
    pub cursor: Cursor,
    /// The lobby's Settings page: Apply instead of Start Hosting, and no password row.
    pub editing_session: bool,
}

impl HostForm {
    /// A new room called `room` with the LAN defaults.
    pub fn new(room: &str, tracks: Vec<TrackChoice>) -> Self {
        let defaults = Settings::lan(ServerName::new(room).unwrap_or_default());
        let mut form = Self::from_settings(&defaults, tracks);
        form.room = Field::new(room, 32);
        form.editing_session = false;
        form
    }

    /// The form of a session's `settings` (the lobby's Settings page).
    pub fn from_settings(settings: &Settings, tracks: Vec<TrackChoice>) -> Self {
        Self {
            room: Field::new(settings.name.as_str(), 32),
            mode: settings.mode,
            track: settings.track,
            laps: settings.laps.max(1),
            traffic: settings.traffic_density,
            cops: settings.cop_level_cap,
            players: settings.max_players,
            password: Field::secret(24),
            tracks,
            cursor: Cursor::default(),
            editing_session: true,
        }
    }

    fn items(&self) -> impl Iterator<Item = Item> + '_ {
        ITEMS.into_iter().filter(|item| !(self.editing_session && *item == Item::Password))
    }

    fn item(&self) -> Item {
        let items: Vec<Item> = self.items().collect();
        items[self.cursor.clamped(items.len())]
    }

    /// The tracks of the current mode.
    pub fn mode_tracks(&self) -> Vec<&TrackChoice> {
        let found: Vec<&TrackChoice> = self.tracks.iter().filter(|track| track.mode == track_mode(self.mode)).collect();
        found
    }

    fn track_title(&self) -> String {
        let tracks = self.mode_tracks();
        let found = tracks.iter().find(|track| track.id == self.track);
        found.map_or_else(|| t!("lan-none"), |track| track.title.clone())
    }

    /// Whether a text row has the keys.
    pub fn typing(&self) -> bool {
        self.room.editing || self.password.editing
    }

    /// The rows as the page shows them.
    pub fn rows(&self) -> Vec<Row> {
        self.items().map(|item| self.row(item)).collect()
    }

    fn row(&self, item: Item) -> Row {
        match item {
            Item::Room => Row::text(field_name(SettingsField::Name), self.room.shown(), self.room.editing),
            Item::Mode => Row::choice(field_name(SettingsField::Mode), mode_name(self.mode), true),
            Item::Track => {
                Row::choice(field_name(SettingsField::Track), self.track_title(), self.mode_tracks().len() > 1)
            }
            Item::Laps => Row::choice(game(label::LAPS), self.laps.to_string(), has_laps(self.mode)),
            Item::Traffic => {
                Row::choice(field_name(SettingsField::TrafficDensity), percent(i64::from(self.traffic)), true)
            }
            Item::Cops => Row::choice(field_name(SettingsField::CopLevelCap), cops_text(self.cops), true),
            Item::Players => Row::choice(field_name(SettingsField::MaxPlayers), self.players.to_string(), true),
            Item::Password if self.password.text.is_empty() && !self.password.editing => {
                Row::text(game(label::PASSWORD), t!("lan-none"), false)
            }
            Item::Password => Row::text(game(label::PASSWORD), self.password.shown(), self.password.editing),
            Item::Submit if self.editing_session => Row::action(t!("lan-host-apply"), true),
            Item::Submit => {
                Row::action(t!("lan-host-start"), !self.room.value().is_empty() && !self.mode_tracks().is_empty())
            }
            Item::Back => Row::action(game(label::BACK), true),
        }
    }

    pub fn up(&mut self) {
        let len = self.items().count();
        self.cursor.up(len);
    }

    pub fn down(&mut self) {
        let len = self.items().count();
        self.cursor.down(len);
    }

    /// Left (`forward` false) or right on the selected row.
    pub fn step(&mut self, forward: bool) {
        let sign = match forward {
            true => 1,
            false => -1,
        };
        match self.item() {
            Item::Mode => {
                self.mode = cycle(&HOST_MODES, self.mode, forward);
                self.fit_track();
            }
            Item::Track => {
                let ids: Vec<u32> = self.mode_tracks().iter().map(|track| track.id).collect();
                self.track = cycle(&ids, self.track, forward);
            }
            Item::Laps if has_laps(self.mode) => self.laps = nudge(self.laps, sign, 1, MAX_LAPS),
            Item::Traffic => self.traffic = nudge(self.traffic, 10 * sign, 0, MAX_TRAFFIC_DENSITY),
            Item::Cops => self.cops = nudge(self.cops, sign, 0, MAX_COP_LEVEL),
            Item::Players => self.players = nudge(self.players, sign, *PLAYERS_RANGE.start(), *PLAYERS_RANGE.end()),
            _ => {}
        }
    }

    /// The track stays one of the mode's: the first when it is not.
    fn fit_track(&mut self) {
        let ids: Vec<u32> = self.mode_tracks().iter().map(|track| track.id).collect();
        if ids.contains(&self.track) {
            return;
        }
        self.track = ids.first().copied().unwrap_or(0);
    }

    /// Accept on the selected row: a text row starts or ends typing; Submit and Back answer.
    pub fn accept(&mut self) -> Option<FormOut> {
        match self.item() {
            Item::Room => self.room.editing = !self.room.editing,
            Item::Password => self.password.editing = !self.password.editing,
            Item::Submit if self.row(Item::Submit).enabled => return Some(FormOut::Submit),
            Item::Back => return Some(FormOut::Back),
            _ => {}
        }
        None
    }

    /// Back: ends typing, or leaves the form.
    pub fn back(&mut self) -> Option<FormOut> {
        if self.typing() {
            self.room.editing = false;
            self.password.editing = false;
            return None;
        }
        Some(FormOut::Back)
    }

    pub fn take(&mut self, key: Typed) {
        self.room.take(key);
        self.password.take(key);
    }

    /// `base` with the form's values. Police off turns pursuits off; laps are 0 for a mode without laps.
    pub fn settings(&self, base: &Settings) -> Settings {
        Settings {
            name: ServerName::new(self.room.value()).unwrap_or_else(|_| base.name.clone()),
            mode: self.mode,
            track: self.track,
            laps: match has_laps(self.mode) {
                true => self.laps,
                false => 0,
            },
            traffic_density: self.traffic,
            cop_level_cap: self.cops,
            pursuits: self.cops > 0,
            max_players: self.players,
            ..base.clone()
        }
    }

    /// What changed against `old`, as a host's patch.
    pub fn patch(&self, old: &Settings) -> SettingsPatch {
        let new = self.settings(old);
        let changed = |a: u8, b: u8| (a != b).then_some(b);
        SettingsPatch {
            name: (new.name != old.name).then(|| new.name.clone()),
            mode: (new.mode != old.mode).then_some(new.mode),
            track: (new.track != old.track).then_some(new.track),
            laps: changed(old.laps, new.laps),
            traffic_density: changed(old.traffic_density, new.traffic_density),
            cop_level_cap: changed(old.cop_level_cap, new.cop_level_cap),
            pursuits: (new.pursuits != old.pursuits).then_some(new.pursuits),
            max_players: changed(old.max_players, new.max_players),
            ..SettingsPatch::default()
        }
    }
}

/// The mode a track list is kept under: the race modes have their own tracks, the free modes share the city.
fn track_mode(mode: Mode) -> Option<Mode> {
    match mode {
        Mode::Circuit | Mode::Sprint | Mode::Speedtrap | Mode::Knockout | Mode::Drag => Some(mode),
        _ => None,
    }
}

fn cops_text(cap: u8) -> String {
    match cap {
        0 => game(label::OFF),
        heat => t!("lan-host-heat", heat = heat),
    }
}

#[cfg(test)]
#[path = "host_form_tests.rs"]
mod tests;

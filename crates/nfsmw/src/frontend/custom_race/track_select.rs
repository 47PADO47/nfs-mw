//! Custom Race, step two: the track list (`Track_Select.fng`, the original's `UIQRTrackSelect`). Left and right step through the
//! tracks of the kind of race and the region the player filtered; the shoulder buttons (and the arrows beside the region)
//! change the region. A track the career has not opened shows what unlocks it, and Accept is refused. Accept on an open
//! track plays the package's leave script and opens the options. Spec: `docs/specs/custom-race.md` section 3.

use std::sync::Arc;

use blackbox_feng::ids::{PAD_ACCEPT, PAD_BACK, PAD_LEFT, PAD_LTRIGGER, PAD_RIGHT, PAD_RTRIGGER};
use blackbox_text::label_hash;
use glam::Vec2;

use super::mode::Mode;
use super::records;
use super::split;
use super::state;
use super::tracks::{self, Listed, Standing};
use crate::career::CareerHandle;
use crate::frontend::challenge_series::{Crop, Maps, Zoom};
use crate::frontend::ids::{self, screen};
use crate::frontend::logic::{Args, Command, Cx, Footer, ScreenLogic, Sub};
use crate::frontend::quick_play::pool::PROLOGUE_STAGE;
use crate::frontend::quick_play::text;
use crate::frontend::quick_race::{self, Board, Track};
use crate::settings::SpeedUnits;
use crate::ui::Text;

/// The objects of the package, by name hash.
const MAP: u32 = 0x0F36_5871;
const TRACK_NAME: [u32; 2] = [0x5E7B_09C9, 0x0DFB_7A2E];
const EVENT_ID: u32 = 0xBBF9_70CD;
const MODE_NAME: u32 = 0xB515_4998;
const LENGTH: u32 = 0xB515_4999;
const RECORD: u32 = 0xB515_499C;
const RECORD_LABEL: u32 = 0x2846_2C64;
const MODE_ICON: u32 = 0x0800_7B4C;
const REGION: [u32; 2] = [0x7800_8599, 0x4510_987F];
const NUMBER: u32 = 0x6F25_A248;
const NUMBER_OF: u32 = 0xB203_7BDC;
/// The package's own "9 / 9" counter, which the game never writes; it would draw over the real one.
const STATIC_COUNTER: [u32; 3] = [0x5B80_5EE5, 0x2E48_1779, 0xF658_6F97];
const LOCKED_PANEL: u32 = 0x6B67_D70B;
const LOCKED_TEXT: u32 = 0x6821_5623;
/// The arrows beside the track name and beside the region, which a click presses.
const TRACK_ARROWS: [u32; 2] = [0x29B7_FF24, 0x6126_AB37];
const REGION_ARROWS: [u32; 2] = [0x091C_4A50, 0x2D14_5BE3];

/// `Unlock this track by beating %s, Blacklist member %d.`, and `--:--.--` for a list with no track.
const UNLOCK_LABEL: u32 = 0xBD56_3BE5;
const NO_RECORD: u32 = 0x472A_A00A;
const KMH_TO_MPH: f32 = 0.621_371;
/// The units of a speed record: kilometres and miles per hour.
const KPH_LABEL: u32 = 0x8569_A25F;
const MPH_LABEL: u32 = 0x8569_AB44;

pub struct TrackSelect {
    board: Option<Arc<Board>>,
    /// The tracks of the mode and region, and the one shown.
    list: Vec<Listed>,
    at: usize,
    shown: Option<usize>,
    maps: Maps,
    zoom: Zoom,
    /// Accept was pressed: the leave script runs and `EXIT_COMPLETE` opens the options.
    leaving: bool,
}

impl Default for TrackSelect {
    fn default() -> Self {
        Self {
            board: None,
            list: Vec::new(),
            at: 0,
            shown: None,
            maps: Maps::default(),
            zoom: Zoom::at(Crop::WHOLE),
            leaving: false,
        }
    }
}

impl TrackSelect {
    fn standing() -> Standing {
        CareerHandle::shared()
            .with_loaded(|career| Standing::of(&career.blacklist))
            .unwrap_or_else(|| Standing::at(PROLOGUE_STAGE))
    }

    /// Takes the board once it was read and makes the list.
    fn take_board(&mut self, cx: &mut Cx) {
        if self.board.is_some() {
            return;
        }
        let Some(board) = quick_race::board() else { return };
        let mode = state::mode();
        self.maps
            .start(board.tracks.iter().filter(|t| t.race_type == mode.race_type()).map(|t| t.map_texture).collect());
        self.board = Some(board);
        self.rebuild(cx, state::with(|s| s.event().map(str::to_owned)));
    }

    /// Makes the list again for the mode and region, on the track named `keep` when it is still in it.
    fn rebuild(&mut self, cx: &mut Cx, keep: Option<String>) {
        let Some(board) = self.board.clone() else { return };
        let (mode, filter) = state::with(|s| (s.mode, s.filter));
        self.list = tracks::list(&board, mode, filter, &Self::standing(), tracks::unlock_all());
        self.at = keep
            .and_then(|name| self.list.iter().position(|listed| board.tracks[listed.index].name == name))
            .unwrap_or(0);
        self.shown = None;
        self.refresh(cx);
    }

    fn current(&self) -> Option<(&Track, Listed)> {
        let listed = *self.list.get(self.at)?;
        Some((&self.board.as_ref()?.tracks[listed.index], listed))
    }

    fn step(&mut self, cx: &mut Cx, by: isize) {
        if self.list.len() < 2 {
            return;
        }
        self.at = (self.at as isize + by).rem_euclid(self.list.len() as isize) as usize;
        self.refresh(cx);
    }

    fn step_region(&mut self, cx: &mut Cx, forward: bool) {
        let keep = self.current().map(|(track, _)| track.name.clone());
        state::with(|s| s.filter = s.filter.step(forward));
        self.rebuild(cx, keep);
    }

    /// Writes everything the shown track decides (`RefreshHeader`).
    fn refresh(&mut self, cx: &mut Cx) {
        let (mode, filter) = state::with(|s| (s.mode, s.filter));
        cx.label_group(ids::TITLE_GROUP, mode.track_title());
        for name in REGION {
            if let Some(o) = cx.object(name) {
                cx.label(o, filter.label());
            }
        }
        for hidden in STATIC_COUNTER {
            cx.hide(hidden, true);
        }
        cx.hide(EVENT_ID, true);
        if let Some(o) = cx.object(MODE_NAME) {
            cx.label(o, mode.name());
        }
        if let Some(o) = cx.object(MODE_ICON) {
            cx.rt.set_texture(o, mode.icon());
        }
        let number = if self.list.is_empty() { 0 } else { self.at + 1 };
        cx.text_object(NUMBER, &number.to_string());
        cx.text_object(NUMBER_OF, &self.list.len().to_string());
        let Some((track, listed)) = self.current().map(|(track, listed)| (track.clone(), listed)) else {
            return self.show_empty(cx);
        };
        self.show_track(cx, mode, &track, listed.locked);
    }

    /// No track in the list: dashes, no lock panel and no Accept.
    fn show_empty(&mut self, cx: &mut Cx) {
        for name in TRACK_NAME {
            cx.text_object(name, "");
        }
        cx.text_object(LENGTH, "--");
        let none = Text::Game(NO_RECORD).resolve(cx.assets).unwrap_or_default();
        cx.text_object(RECORD, &none);
        cx.hide(LOCKED_PANEL, true);
        self.shown = None;
    }

    fn show_track(&mut self, cx: &mut Cx, mode: Mode, track: &Track, locked: bool) {
        cx.hide(LOCKED_PANEL, !locked);
        if locked {
            let sentence = unlock_sentence(cx, track);
            cx.text_object(LOCKED_TEXT, &sentence);
        }
        let name = Text::Game(label_hash(&text::track_label_key(&track.event))).resolve(cx.assets);
        for object in TRACK_NAME {
            cx.text_object(object, name.as_deref().unwrap_or(&track.event));
        }
        let metric = cx.settings.speed_units == SpeedUnits::Metric;
        let unit = Text::Game(text::unit_label(metric)).resolve(cx.assets).unwrap_or_default();
        cx.text_object(LENGTH, &text::length_text(track.brief_length(), metric, &unit));
        let record = record_text(cx, mode, metric, &track.event);
        cx.text_object(RECORD, &record);
        if let Some(label) = mode.record_label()
            && let Some(o) = cx.object(RECORD_LABEL)
        {
            cx.label(o, label);
        }
        if self.shown != Some(self.at) {
            self.shown = Some(self.at);
            self.zoom.go_to(track.crop);
        }
        self.place_map(cx, track);
    }

    fn place_map(&self, cx: &mut Cx, track: &Track) {
        let Some(o) = cx.object(MAP) else { return };
        let loaded = cx.assets.texture(track.map_texture).is_some();
        cx.rt.set_hidden(o, !loaded);
        if !loaded {
            return;
        }
        cx.rt.set_texture(o, track.map_texture);
        cx.rt.set_uv(o, self.zoom.now().uv());
    }

    fn accept(&mut self, cx: &mut Cx) {
        let Some((track, listed)) = self.current().map(|(track, listed)| (track.name.clone(), listed)) else { return };
        if listed.locked {
            return;
        }
        state::with(|s| s.set_event(Some(track)));
        self.leaving = true;
        cx.script(ids::EVENT_HANDLER, ids::SCRIPT_FORWARD);
    }

    /// The leave is over: the options open, or in split screen a drag, sprint or speed trap race goes straight to Press Start
    /// for player 1 (the race has one opponent and no cops, and no row left to set) **[decomp]**.
    fn after_accept(&mut self, cx: &mut Cx) {
        let straight = state::with(|s| s.split.on && split::skips_options(s.mode));
        if !straight {
            return self.leave_to(cx, screen::TRACK_OPTIONS, Args::default());
        }
        state::with(|s| s.split.clear_seats());
        self.leave_to(cx, screen::PRESS_START, Args::default());
    }

    fn leave_to(&mut self, cx: &mut Cx, screen: &str, args: Args) {
        self.maps.release(cx.assets);
        cx.send(Command::Switch(screen.into(), args));
    }
}

/// `Unlock this track by beating <alias>, Blacklist member <bin>.`, `None`-safe: the sentence of a track with no bin is empty.
fn unlock_sentence(cx: &Cx, track: &Track) -> String {
    let Some(bin) = track.bin else { return String::new() };
    let template = Text::Game(UNLOCK_LABEL).resolve(cx.assets).unwrap_or_default();
    let alias = Text::Game(label_hash(&text::alias_key(bin))).resolve(cx.assets).unwrap_or_default();
    text::fill_sentence(&template, &alias, bin)
}

/// The record line: the best time, or for the speedtrap the best speed in the speed unit; an event never raced reads `0.00` or
/// `0 MPH`/`0 KPH` **[verified]**.
fn record_text(cx: &Cx, mode: Mode, metric: bool, event: &str) -> String {
    let best = records::best(event);
    match mode {
        Mode::SpeedTrap => {}
        Mode::Circuit | Mode::Knockout => return records::time_text(best.lap),
        _ => return records::time_text(best.time),
    }
    let unit = Text::Game(if metric { KPH_LABEL } else { MPH_LABEL }).resolve(cx.assets).unwrap_or_default();
    let speed = best.speed.unwrap_or(0.0) * if metric { 1.0 } else { KMH_TO_MPH };
    format!("{speed:.0} {unit}")
}

impl ScreenLogic for TrackSelect {
    fn start(&mut self, cx: &mut Cx) {
        self.take_board(cx);
        self.refresh(cx);
    }

    fn resume(&mut self, cx: &mut Cx) {
        self.refresh(cx);
    }

    fn tick(&mut self, cx: &mut Cx, dt: f32) {
        // The track list is read in the background, so the first frames may have none.
        if self.board.is_none() {
            return self.take_board(cx);
        }
        self.maps.poll(cx.assets);
        self.zoom.tick(dt);
        let Some(track) = self.current().map(|(track, _)| track.clone()) else { return };
        self.place_map(cx, &track);
    }

    fn click(&mut self, cx: &mut Cx, point: Vec2) -> bool {
        if self.leaving {
            return false;
        }
        let find = |cx: &Cx, names: [u32; 2]| names.map(|name| cx.object(name));
        let (tracks, regions) = (find(cx, TRACK_ARROWS), find(cx, REGION_ARROWS));
        let all: Vec<_> = tracks.iter().chain(regions.iter()).flatten().copied().collect();
        let Some(hit) = cx.rt.object_at(&all, point) else { return false };
        let before = (self.at, state::with(|s| s.filter));
        match (tracks.iter().position(|o| *o == Some(hit)), regions.iter().position(|o| *o == Some(hit))) {
            (Some(side), _) => self.step(cx, if side == 0 { -1 } else { 1 }),
            (_, Some(side)) => self.step_region(cx, side == 1),
            _ => return false,
        }
        if before != (self.at, state::with(|s| s.filter)) {
            crate::audio::menu::request(PAD_RIGHT);
        }
        true
    }

    fn message(&mut self, cx: &mut Cx, message: u32) {
        match message {
            PAD_LEFT if !self.leaving => self.step(cx, -1),
            PAD_RIGHT if !self.leaving => self.step(cx, 1),
            PAD_LTRIGGER if !self.leaving => self.step_region(cx, false),
            PAD_RTRIGGER if !self.leaving => self.step_region(cx, true),
            PAD_ACCEPT if !self.leaving => self.accept(cx),
            PAD_BACK if !self.leaving => {
                state::with(|s| s.set_event(None));
                self.leave_to(cx, screen::MAIN_MENU_SUB, Args { sub: Sub::CustomRace, ..Args::default() });
            }
            ids::EXIT_COMPLETE if self.leaving => self.after_accept(cx),
            _ => {}
        }
    }

    fn footer(&self) -> Footer {
        match self.current() {
            Some((_, listed)) if !listed.locked => Footer::BackAccept,
            _ => Footer::BackOnly,
        }
    }
}

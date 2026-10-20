//! Quick Race, step one: the track list (`UIQRTrackSelect`, `Track_Select.fng`). Left and right step through the
//! races of the install (`quick_race.rs`); the track's name, kind and length go into the package's strings. Accept plays
//! the package's leave script and then opens the track's options; back returns to the main menu
//! (docs/specs/frontend-menus.md, section 5).

use blackbox_feng::ids::{PAD_ACCEPT, PAD_BACK, PAD_LEFT, PAD_RIGHT};

use super::ids::{self, screen};
use super::logic::{Args, Command, Cx, ScreenLogic};
use super::quick_race::{self, Board, Track};

/// The group that holds the track's name (two strings: the text and its shadow), by name hash.
const TRACK_NAME_GROUP: u32 = 0x1120_38E6;
/// The strings under the map: the race kind (`Circuit - K.O.`) and the length (`10KM`), by name hash.
const KIND_STRING: u32 = 0xB515_4998;
const LENGTH_STRING: u32 = 0xB515_4999;

#[derive(Default)]
pub struct TrackSelect {
    /// Accept was pressed: the leave script runs and `EXIT_COMPLETE` opens the options.
    leaving: bool,
    /// The track list was shown at least once (it is read in the background, so the first frames may have none).
    shown: bool,
}

impl ScreenLogic for TrackSelect {
    fn start(&mut self, cx: &mut Cx) {
        self.refresh(cx);
    }

    fn resume(&mut self, cx: &mut Cx) {
        self.refresh(cx);
    }

    fn tick(&mut self, cx: &mut Cx, _dt: f32) {
        if !self.shown {
            self.refresh(cx);
        }
    }

    fn message(&mut self, cx: &mut Cx, message: u32) {
        match message {
            PAD_LEFT if !self.leaving => self.step(cx, -1),
            PAD_RIGHT if !self.leaving => self.step(cx, 1),
            PAD_ACCEPT if !self.leaving => self.leave(cx),
            PAD_BACK if !self.leaving => cx.send(Command::Switch(screen::MAIN_MENU.into(), Args::default())),
            ids::EXIT_COMPLETE => self.open_options(cx),
            _ => {}
        }
    }
}

impl TrackSelect {
    /// Plays the event handler's forward script, which sends `EXIT_COMPLETE` when the leave animation is done.
    fn leave(&mut self, cx: &mut Cx) {
        self.leaving = true;
        cx.script(ids::EVENT_HANDLER, ids::SCRIPT_FORWARD);
    }

    fn open_options(&self, cx: &mut Cx) {
        if !self.leaving {
            return;
        }
        cx.send(Command::Switch(screen::TRACK_OPTIONS.into(), Args::default()));
    }

    /// Moves to the next or previous track and shows it.
    fn step(&mut self, cx: &mut Cx, by: i32) {
        let Some(board) = quick_race::board() else { return };
        quick_race::step(&board, by);
        self.refresh(cx);
    }

    /// Writes the picked track into the package's strings.
    fn refresh(&mut self, cx: &mut Cx) {
        let Some(board) = quick_race::board() else { return };
        let Some(track) = quick_race::picked(&board) else { return };
        show(cx, track, &board);
        self.shown = true;
    }
}

fn show(cx: &mut Cx, track: &Track, _board: &Board) {
    cx.text_group(TRACK_NAME_GROUP, &track.title);
    cx.text_object(KIND_STRING, track.kind);
    cx.text_object(LENGTH_STRING, &track.length_text());
}

//! Quick Race, step two: the track's options (`UIQRTrackOptions`, `Track_Options.fng`). Back returns to the track
//! list. Accept plays the package's own leave animation and starts the race on the track that was picked: the world
//! opens behind the loading screen and the race begins on the grid (`quick_race.rs`,
//! docs/specs/frontend-menus.md, section 5). The option rows (laps, opponents, difficulty, traffic) are not bound yet:
//! the race runs with its own stored values.

use blackbox_feng::ids::{PAD_ACCEPT, PAD_BACK};

use super::ids::screen;
use super::logic::{Args, Command, Cx, ScreenLogic};
use super::quick_race;

pub struct TrackOptions;

impl ScreenLogic for TrackOptions {
    fn message(&mut self, cx: &mut Cx, message: u32) {
        match message {
            PAD_BACK => cx.send(Command::Switch(screen::TRACK_SELECT.into(), Args::default())),
            PAD_ACCEPT => start(cx),
            _ => {}
        }
    }
}

/// Asks the front end to start the picked track. Nothing happens while the track list is not read yet.
fn start(cx: &mut Cx) {
    let Some(board) = quick_race::board() else { return };
    let Some(track) = quick_race::picked(&board) else { return };
    cx.send(Command::StartQuickRace(track.name.clone()));
}

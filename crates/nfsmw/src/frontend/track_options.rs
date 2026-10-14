//! Quick Race, step two: the track's options (`UIQRTrackOptions`, `Track_Options.fng`). Back returns to the track
//! list. Accept plays the package's own leave animation and sends the game `PAD_ACCEPT`, but the race does not start
//! yet (there is no race engine), so this logic answers it with no command (docs/specs/frontend-menus.md, section 5).

use blackbox_feng::ids::PAD_BACK;

use super::ids::screen;
use super::logic::{Args, Command, Cx, ScreenLogic};

pub struct TrackOptions;

impl ScreenLogic for TrackOptions {
    fn message(&mut self, cx: &mut Cx, message: u32) {
        if message == PAD_BACK {
            cx.send(Command::Switch(screen::TRACK_SELECT.into(), Args::default()));
        }
    }
}

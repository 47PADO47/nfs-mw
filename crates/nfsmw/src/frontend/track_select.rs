//! Quick Race, step one: the track list (`UIQRTrackSelect`, `Track_Select.fng`). Accept plays the package's leave
//! script and then opens the track's options; back returns to the main menu. The list is not populated yet (no
//! track source is read), so left and right do nothing (docs/specs/frontend-menus.md, section 5).

use blackbox_feng::ids::{PAD_ACCEPT, PAD_BACK};

use super::ids::{self, screen};
use super::logic::{Args, Command, Cx, ScreenLogic};

#[derive(Default)]
pub struct TrackSelect {
    /// Accept was pressed: the leave script runs and `EXIT_COMPLETE` opens the options.
    leaving: bool,
}

impl ScreenLogic for TrackSelect {
    fn message(&mut self, cx: &mut Cx, message: u32) {
        match message {
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
}

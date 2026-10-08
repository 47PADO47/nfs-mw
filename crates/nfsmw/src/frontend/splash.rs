//! The title screen (docs/specs/frontend-menus.md, section 4): the package fades its text in and tells the game
//! (`INIT_COMPLETE`) after about five seconds; from then on accept or start goes on to the main menu.

use blackbox_feng::ids::{PAD_ACCEPT, PAD_START};

use super::ids;
use super::logic::{Command, Cx, ScreenLogic};

/// Objects the original hides or relabels at start-up.
const HD_GROUP: u32 = 0x534C_C377;
const START_CLICK: u32 = 0x13CF_446D;
const MOUSE_CLICK: u32 = 0x8C0B_D743;
/// A console licence line (`Licensed by ...`) that has no meaning on the PC.
const LICENCE_LINE: u32 = 0x4B98_C4B9;
const ESRB_ICON: u32 = 0x43D4_1F73;
/// The group that shows the prompt, and its label: `Press START`.
const PROMPT: u32 = 0xC4DF_3FF2;
const PRESS_START: u32 = 0x9B58_0A55;

#[derive(Default)]
pub struct Splash {
    allow_continue: bool,
}

impl ScreenLogic for Splash {
    fn start(&mut self, cx: &mut Cx) {
        for hidden in [HD_GROUP, START_CLICK, MOUSE_CLICK, LICENCE_LINE, ESRB_ICON] {
            cx.hide(hidden, true);
        }
        cx.label_group(PROMPT, PRESS_START);
    }

    fn message(&mut self, cx: &mut Cx, message: u32) {
        match message {
            ids::INIT_COMPLETE => self.allow_continue = true,
            PAD_ACCEPT | PAD_START if self.allow_continue => cx.send(Command::NextBootStep),
            _ => {}
        }
    }
}

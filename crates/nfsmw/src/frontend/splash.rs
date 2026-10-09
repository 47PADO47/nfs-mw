//! The title screen (docs/specs/frontend-menus.md, section 4): the package fades its text in, and `Press START`
//! comes up after a short wait. From then on accept or start goes on to the main menu.

use blackbox_feng::ids::{PAD_ACCEPT, PAD_START};

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
/// `CURRENT_GEN_WIDESCREEN`: the game sends it to the widescreen package, which shows its wide art on it.
const WIDESCREEN: u32 = 0xCB83_5EE3;
/// Seconds from the title screen appearing until the prompt shows and accept starts the game.
const PROMPT_DELAY: f32 = 1.0;

pub struct Splash {
    /// Seconds left before the prompt shows.
    wait: f32,
    allow_continue: bool,
}

impl Default for Splash {
    fn default() -> Self {
        Self { wait: PROMPT_DELAY, allow_continue: false }
    }
}

impl ScreenLogic for Splash {
    fn start(&mut self, cx: &mut Cx) {
        for hidden in [HD_GROUP, START_CLICK, MOUSE_CLICK, LICENCE_LINE, ESRB_ICON, PROMPT] {
            cx.hide(hidden, true);
        }
        cx.label_group(PROMPT, PRESS_START);
        if cx.name.to_ascii_lowercase().starts_with("ws_") {
            cx.rt.post_to_package(cx.package, WIDESCREEN);
        }
    }

    fn tick(&mut self, cx: &mut Cx, dt: f32) {
        if self.allow_continue {
            return;
        }
        self.wait -= dt;
        if self.wait > 0.0 {
            return;
        }
        cx.hide(PROMPT, false);
        self.allow_continue = true;
    }

    fn message(&mut self, cx: &mut Cx, message: u32) {
        match message {
            PAD_ACCEPT | PAD_START if self.allow_continue => cx.send(Command::NextBootStep),
            _ => {}
        }
    }
}

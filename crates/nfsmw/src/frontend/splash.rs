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
/// `CURRENT_GEN_WIDESCREEN`: the game sends it to the widescreen package, which shows its wide art on it.
const WIDESCREEN: u32 = 0xCB83_5EE3;

#[derive(Default)]
pub struct Splash {
    allow_continue: bool,
}

impl ScreenLogic for Splash {
    fn sound_trigger(
        &mut self,
        _: &blackbox_feng::Runtime,
        _: blackbox_feng::PackageId,
        _: u32,
        cue: Option<i32>,
    ) -> Option<i32> {
        cue.filter(|_| self.allow_continue)
    }

    fn start(&mut self, cx: &mut Cx) {
        for hidden in [HD_GROUP, START_CLICK, MOUSE_CLICK, LICENCE_LINE, ESRB_ICON] {
            cx.hide(hidden, true);
        }
        cx.label_group(PROMPT, PRESS_START);
        if cx.name.to_ascii_lowercase().starts_with("ws_") {
            cx.rt.post_to_package(cx.package, WIDESCREEN);
        }
    }

    fn message(&mut self, cx: &mut Cx, message: u32) {
        match message {
            ids::INIT_COMPLETE => self.allow_continue = true,
            PAD_ACCEPT | PAD_START if self.allow_continue => cx.send(Command::NextBootStep),
            _ => {}
        }
    }
}

#[cfg(test)]
#[test]
fn splash_cues_wait_for_ready() {
    let runtime = blackbox_feng::Runtime::new();
    let package = blackbox_feng::PackageId(0);
    let mut splash = Splash::default();
    assert_eq!(splash.sound_trigger(&runtime, package, PAD_ACCEPT, Some(4)), None);
    splash.allow_continue = true;
    assert_eq!(splash.sound_trigger(&runtime, package, PAD_ACCEPT, Some(4)), Some(4));
}

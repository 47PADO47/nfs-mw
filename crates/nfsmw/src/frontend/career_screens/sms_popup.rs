//! The text message that opens over the game when the phone delivers it (`docs/specs/story-flow-2.md` section 3.7,
//! `docs/specs/career-phone.md` section 3.6). The screen is `SMS_Message.fng`, the inbox's message screen; this logic
//! shows the message and closes it with Back or the focused Done button, which resumes the game. Replay and delete do
//! nothing here: the voice is not played yet, and delete is an action of the inbox.

use blackbox_feng::ids::PAD_BACK;
use blackbox_feng::runtime::pad;

use super::sms::show_message;
use crate::frontend::ids::INPUT_ENABLE;
use crate::frontend::logic::{Command, Cx, ScreenLogic};

/// The Done button's group in `SMS_Message.fng` (`nfsmw dump-screen SMS_Message.fng`).
const DONE: u32 = 0x000F_A4AA;

/// Whether a message from the package closes the popup: Back does.
pub const fn closes_on(message: u32) -> bool {
    message == PAD_BACK
}

/// Whether a pad press closes the popup: Accept while Done has the focus.
pub const fn closes_on_press(bits: u32, focus: Option<u32>) -> bool {
    bits & pad::ACCEPT != 0 && matches!(focus, Some(DONE))
}

/// The message screen over the frozen game, showing message `id`.
pub struct SmsPopup {
    id: u32,
}

impl SmsPopup {
    /// The popup for message `id`.
    pub fn new(id: u32) -> Self {
        Self { id }
    }
}

impl ScreenLogic for SmsPopup {
    fn start(&mut self, cx: &mut Cx) {
        show_message(cx, self.id);
        cx.rt.post_to_package(cx.package, INPUT_ENABLE);
    }

    fn message(&mut self, cx: &mut Cx, message: u32) {
        if closes_on(message) {
            cx.send(Command::Resume);
        }
    }

    fn pressed(&mut self, cx: &mut Cx, button: u32) {
        if button == DONE {
            cx.send(Command::Resume);
        }
    }

    fn pad_pressed(&mut self, cx: &mut Cx, bits: u32) {
        let focus = cx.rt.focus(cx.package);
        if closes_on_press(bits, focus) {
            cx.send(Command::Resume);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn back_closes_and_the_replay_and_delete_buttons_do_not() {
        assert!(closes_on(PAD_BACK));
        assert!(!closes_on(0xC519_BFC3), "button 4 replays the voice");
        assert!(!closes_on(0xC519_BFC4), "button 5 asks to delete");
    }

    #[test]
    fn accept_closes_only_on_the_done_button() {
        assert!(closes_on_press(pad::ACCEPT, Some(DONE)));
        assert!(!closes_on_press(pad::ACCEPT, Some(0x000F_A584)), "Re-Play has the focus");
        assert!(!closes_on_press(pad::ACCEPT, None));
        assert!(!closes_on_press(pad::BACK, Some(DONE)), "Back is the message, not the press");
    }
}

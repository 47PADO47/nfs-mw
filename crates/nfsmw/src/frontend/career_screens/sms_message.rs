//! One text message (`SMS_Message.fng`), opened from the inbox ([`MessageLogic`]) or over the game
//! (the delivery of a message while driving, `flow_sms.rs`): the sender, the subject and the body, read from the language table.
//!
//! The package gives the body ten string objects, one per line, 20 units apart and each at most 620 wide
//! (`nfsmw dump-screen SMS_Message.fng`). The body is wrapped into lines with the installed font and written top to
//! bottom; a body longer than ten lines scrolls one line per press of the package's up and down arrows (or the wheel).

use blackbox_feng::ids::{PAD_DOWN, PAD_UP};
use nfsmw_career::phone::{from_label_for, is_story, label_for, subject_label_for};

use super::sms::{
    DELETE, REPLAY, delete_question, leave, leave_command, lookup, open_message, opened, replay_voice, set_text,
};
use crate::frontend::ids::{EXIT_COMPLETE, INIT_COMPLETE};
use crate::frontend::logic::{Args, Command, Cx, Footer, ScreenLogic};
use crate::frontend::text_wrap::wrap;

/// The body's lines, top to bottom.
pub const BODY_LINES: [u32; 10] = [
    0x000F_4BA5,
    0x000F_A694,
    0x000F_A697,
    0x000F_A69A,
    0x000F_A69D,
    0x000F_A6A0,
    0x000F_A6A3,
    0x000F_A6A6,
    0x000F_A6AE,
    0x000F_A6B1,
];
/// The sender's and the subject's values, on the rows of the `From:` and `Subject:` captions.
const FROM: u32 = 0x000F_9301;
const SUBJECT: u32 = 0x000E_5A97;
/// The scroll bar's thumb and track, and the package's scripts that show and hide them (`SHOW`, `HIDE`).
const SCROLL_BAR: [u32; 2] = [0x3FBB_7A4E, 0x688A_1551];
const SHOW: u32 = 0x001C_A7C0;
const HIDE: u32 = 0x0016_A259;

/// The first line shown after scrolling `delta` lines (negative is up), kept so that the ten lines stay full.
pub fn scrolled_line(first: usize, delta: i32, lines: usize) -> usize {
    let last = lines.saturating_sub(BODY_LINES.len());
    (first as i64 + i64::from(delta)).clamp(0, last as i64) as usize
}

/// What one message screen shows: the message, its body's lines and the first line in view.
pub struct MessageView {
    pub id: u32,
    lines: Vec<String>,
    first: usize,
    /// The screen showed the message: closing it stops the voice.
    shown: bool,
}

impl MessageView {
    pub fn new(id: u32) -> Self {
        Self { id, lines: Vec::new(), first: 0, shown: false }
    }

    /// Shows the message. It is marked read, and an unread story message queues its voice, when the package's handler
    /// says the screen is up (`INIT_COMPLETE`, 500 ms after the start), as the original does; speech runs while the
    /// screen is up and the line stops when it closes (`cell_call::message_screen`).
    pub fn show(&mut self, cx: &mut Cx) {
        crate::audio::cell_call::message_screen(true);
        self.shown = true;
        let body = lookup(&label_for(self.id));
        self.lines = wrap(cx, cx.guid(BODY_LINES[0]), &body);
        self.first = 0;
        self.fill(cx);
        self.scroll_bar(cx);
        set_text(cx, FROM, lookup(&from_label_for(self.id)));
        set_text(cx, SUBJECT, lookup(&subject_label_for(self.id)));
    }

    fn fill(&self, cx: &mut Cx) {
        for (at, guid) in BODY_LINES.into_iter().enumerate() {
            let line = self.lines.get(self.first + at).cloned().unwrap_or_default();
            set_text(cx, guid, line);
        }
    }

    /// The original keeps the scroll bar transparent while the body fits the window (its package's `HIDE` script), and
    /// shows it only for a body longer than the window.
    fn scroll_bar(&self, cx: &mut Cx) {
        let script = match self.lines.len() > BODY_LINES.len() {
            true => SHOW,
            false => HIDE,
        };
        for hash in SCROLL_BAR {
            let Some(object) = cx.object(hash) else { continue };
            let current = cx.rt.object(object).and_then(|s| s.current.and_then(|c| s.scripts.get(c))).map(|s| s.id);
            if current != Some(script) {
                cx.rt.run_script(object, script);
            }
        }
    }

    pub fn scroll(&mut self, cx: &mut Cx, delta: i32) {
        let first = scrolled_line(self.first, delta, self.lines.len());
        if first == self.first {
            return;
        }
        self.first = first;
        self.fill(cx);
    }

    /// The common messages of the screen: the open action, replay, delete (asking first, then running `then`) and the
    /// arrows. Returns whether the message was one of them.
    pub fn message(&mut self, cx: &mut Cx, message: u32, then: Command) -> bool {
        match message {
            INIT_COMPLETE => {
                open_message(self.id);
                self.scroll_bar(cx);
            }
            REPLAY => replay_voice(self.id),
            DELETE => cx.send(Command::Confirm(delete_question(self.id, Some(then)))),
            PAD_UP => self.scroll(cx, -1),
            PAD_DOWN => self.scroll(cx, 1),
            _ => return false,
        }
        true
    }

    /// Done and Delete, and Re-Play for a story message (a tip has no voice).
    pub fn footer(&self) -> Footer {
        Footer::SmsMessage { replay: is_story(self.id) }
    }
}

impl Drop for MessageView {
    fn drop(&mut self) {
        if self.shown {
            crate::audio::cell_call::message_screen(false);
        }
    }
}

/// The message the inbox opened, from the safe house, from the inbox key in free roam, or on delivery while driving
/// (the original opens the inbox at the delivered message, which opens it: `flow_sms.rs`).
pub struct MessageLogic {
    args: Args,
    view: Option<MessageView>,
}

impl MessageLogic {
    pub fn new(args: Args) -> Self {
        Self { args, view: None }
    }
}

impl ScreenLogic for MessageLogic {
    fn sound_trigger(
        &mut self,
        _: &blackbox_feng::Runtime,
        _: blackbox_feng::PackageId,
        message: u32,
        cue: Option<i32>,
    ) -> Option<i32> {
        replay_sound(self.view.as_ref(), message, cue)
    }

    fn start(&mut self, cx: &mut Cx) {
        let Some(id) = opened() else {
            leave(cx, self.args);
            return;
        };
        let mut view = MessageView::new(id);
        view.show(cx);
        self.view = Some(view);
    }

    fn message(&mut self, cx: &mut Cx, message: u32) {
        if message == EXIT_COMPLETE {
            leave(cx, self.args);
            return;
        }
        let then = leave_command(self.args);
        if let Some(view) = self.view.as_mut() {
            view.message(cx, message, then);
        }
    }

    fn scroll(&mut self, cx: &mut Cx, steps: i32) {
        if let Some(view) = self.view.as_mut() {
            view.scroll(cx, -steps);
        }
    }

    fn footer(&self) -> Footer {
        self.view.as_ref().map_or(Footer::SmsMessage { replay: false }, MessageView::footer)
    }
}

pub(super) fn replay_sound(view: Option<&MessageView>, message: u32, cue: Option<i32>) -> Option<i32> {
    if (message == 0x610F_B237 || message == REPLAY) && !view.is_some_and(|view| is_story(view.id)) {
        return None;
    }
    cue
}

#[cfg(test)]
#[test]
fn sms_replay_cue_uses_the_message_voice_type() {
    let story = MessageView::new(105);
    let tip = MessageView::new(110);
    assert_eq!(replay_sound(Some(&story), 0x610F_B237, Some(7)), Some(7));
    assert_eq!(replay_sound(Some(&tip), 0x610F_B237, Some(7)), None);
    assert_eq!(replay_sound(Some(&tip), 0x480C_9A58, Some(5)), Some(5));
}

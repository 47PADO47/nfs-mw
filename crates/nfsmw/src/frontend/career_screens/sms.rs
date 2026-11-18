//! The SMS inbox: the list (`SMS_MailBoxes.fng`, its two tabs and six rows at a time) and one message
//! (`SMS_Message.fng`), `docs/specs/career-phone.md` section 4. The rules are in `nfsmw_career::phone` (the list order, the tabs, reading);
//! this file builds the rows from them, marks a message read when it is opened and shows its text from the language
//! table. Back goes to the safe house menu when the inbox was opened there, otherwise it pops the screen.
//!
//! `Args` carries no message id, so the list stores the id it opens in [`OPENED`] and the message screen reads it when
//! it starts. Opening a story message the first time asks the audio system for its voice ([`crate::audio::cell_call`]);
//! Replay (pad button 4) asks again, cutting the line in progress. Delete (pad button 5, on the list for the highlighted
//! row and on the message) asks "Are you sure you want to delete this SMS message?" (`0x8C3C2171`); Yes empties the slot
//! (`Command::DeleteSms`), No keeps it. The message screen is in [`super::sms_message`]; the footers are drawn by
//! `prompts/sms.rs`, which replaces both of the packages' own copies (keyboard and console).

use std::sync::Mutex;

use blackbox_feng::ids::{PAD_BACK, PAD_BUTTON0};
use blackbox_text::label_hash;
use nfsmw_career::phone::{InboxTab, ListEntry, OpenEffect, from_label_for, subject_label_for};
use nfsmw_career::settings::SmsInbox;

use crate::audio::cell_call;
use crate::career::CareerHandle;
use crate::frontend::confirm::Confirm;
use crate::frontend::ids::{INPUT_ENABLE, screen};
use crate::frontend::logic::{Args, Command, Cx, Footer, ScreenLogic};
use crate::ui::Text;

/// The pad buttons of the message screen (`docs/specs/career-phone.md` 4.3, `docs/specs/story-flow-2.md` F-28): button 4
/// replays the voice and button 5 deletes the message. The screen's init message (`INIT_COMPLETE`, `0x35F8620B`) is its
/// open action.
pub const REPLAY: u32 = PAD_BUTTON0 + 4;
pub const DELETE: u32 = PAD_BUTTON0 + 5;
/// The delete question's label: "Are you sure you want to delete this SMS message?".
const DELETE_QUESTION: u32 = 0x8C3C_2171;

/// The messages the list sends: a row pressed (`0x0C407210`, also the Read hint's hotspot), the tab hotspots
/// (`0x9120409E`, either one), and the list's arrows (`0x72619778` up, `0x911C0A4B` down).
const ROW: u32 = 0x0C40_7210;
const TAB: u32 = 0x9120_409E;
const UP: u32 = 0x7261_9778;
const DOWN: u32 = 0x911C_0A4B;

/// Rows the list shows at once. Each row is a button group with its subject text and its new-message icon.
const ROWS: usize = 6;
const ROW_BUTTONS: [u32; ROWS] = [0x000F_3257, 0x000F_327E, 0x000F_3286, 0x000F_328E, 0x000F_3296, 0x000F_329E];
const ROW_TEXTS: [u32; ROWS] = [0x000F_319B, 0x000F_327F, 0x000F_3287, 0x000F_328F, 0x000F_3297, 0x000F_329F];
const ROW_NEW: [u32; ROWS] = [0x000F_3203, 0x000F_3280, 0x000F_3288, 0x000F_3290, 0x000F_3298, 0x000F_32A0];
/// The tabs: the story messages are the Voice tab (the phone icon, left, where the highlight starts) and the tips the
/// Text tab (the mail icon). Each tab has a new-message icon beside it.
const STORY_TAB: u32 = 0x000F_3404;
const TIPS_TAB: u32 = 0x000F_3420;
const STORY_NEW: u32 = 0x000F_321F;
const TIPS_NEW: u32 = 0x000F_337E;
/// The tab marks start on `HIDE` (no alpha); `SHOW` brings them up.
const SHOW: u32 = 0x001C_A7C0;
const HIDE: u32 = 0x0016_A259;
/// The package's own messages for its list: `LIST_IN` brings the tabs, the rows and the footer in (the message screen's
/// exit script sends it to every package, with `PANEL_IN`, which shows the group that holds them); `LIST_OUT` takes the
/// list and the footer out again, leaving the header. Their names are not known.
const LIST_IN: u32 = 0x775C_E5DF;
const PANEL_IN: u32 = 0x9310_7F76;
const LIST_OUT: u32 = 0x293E_75E8;
/// The tab group's pulse (the selected tab) and its still look (the other): `HIGHLIGHT` and `UNHIGHLIGHT`.
const TAB_SELECTED: u32 = 0x249D_B7B7;
const TAB_IDLE: u32 = 0x7AB5_521A;
/// The frame of corner marks around the selected tab, and its two places (`POS1`, `POS2`): over the Voice tab and over
/// the Text tab.
const TAB_FRAME: u32 = 0x000F_45EE;
const FRAME_AT_STORY: u32 = 0x001B_20C2;
const FRAME_AT_TIPS: u32 = 0x001B_20C3;

/// The message the list opened last: the message screen shows it.
static OPENED: Mutex<Option<u32>> = Mutex::new(None);

pub(super) fn opened() -> Option<u32> {
    *OPENED.lock().unwrap_or_else(|e| e.into_inner())
}

fn set_opened(id: u32) {
    *OPENED.lock().unwrap_or_else(|e| e.into_inner()) = Some(id);
}

/// The logic for `name` (lower case) when it is one of the inbox screens.
pub fn make(name: &str, args: Args) -> Option<Box<dyn ScreenLogic>> {
    if name == screen::SMS_MAILBOXES.to_ascii_lowercase() {
        return Some(Box::new(InboxLogic::new(args)));
    }
    if name == screen::SMS_MESSAGE.to_ascii_lowercase() {
        return Some(Box::new(super::sms_message::MessageLogic::new(args)));
    }
    None
}

/// The rows of the page starting at `first`: at most [`ROWS`] entries.
pub fn page(entries: &[ListEntry], first: usize) -> &[ListEntry] {
    let start = first.min(entries.len());
    let end = (start + ROWS).min(entries.len());
    &entries[start..end]
}

/// The first row after moving `delta` rows (negative is up), kept so that the page stays full.
pub fn scrolled(first: usize, delta: i32, len: usize) -> usize {
    let last = len.saturating_sub(ROWS);
    (first as i64 + i64::from(delta)).clamp(0, last as i64) as usize
}

/// The highlighted entry and the page's first entry after moving the highlight `delta` rows in a list of `len`: the
/// highlight stops at the ends and the page scrolls only to keep it in view.
pub fn moved(at: usize, first: usize, delta: i32, len: usize) -> (usize, usize) {
    if len == 0 {
        return (0, 0);
    }
    let at = (at as i64 + i64::from(delta)).clamp(0, len as i64 - 1) as usize;
    let first = first.min(at).max((at + 1).saturating_sub(ROWS));
    (at, scrolled(first, 0, len))
}

/// The other tab.
pub const fn other_tab(tab: InboxTab) -> InboxTab {
    match tab {
        InboxTab::Story => InboxTab::Tips,
        InboxTab::Tips => InboxTab::Story,
    }
}

/// The new-message icons beside the two tabs, `(story, tips)`: they mark unread messages in the tab the list is not on.
pub const fn unread_marks(tab: InboxTab, story_unread: bool, tips_unread: bool) -> (bool, bool) {
    match tab {
        InboxTab::Story => (false, tips_unread),
        InboxTab::Tips => (story_unread, false),
    }
}

/// Marks message `id` read, as opening it does (state 4). Returns what the opening asks for (the voice of an unread
/// story message), or `None` when the id is not in the list.
pub fn mark_read(inbox: &mut SmsInbox, id: u32) -> Option<OpenEffect> {
    inbox.open_message(id)
}

/// The id to replay for message `id`: a listed story message. Tips and unlisted messages replay nothing.
pub fn replay_id(inbox: &SmsInbox, id: u32) -> Option<u32> {
    inbox.replay(id).map(|_| id)
}

/// Deletes message `id` as the delete action does: its slot goes back to state 0. False for an id with no slot.
pub fn delete_message(inbox: &mut SmsInbox, id: u32) -> bool {
    inbox.delete(id)
}

/// Opens message `id` as the player does: it is marked read, and an unread story message asks for its voice.
pub fn open_message(id: u32) {
    let effect = CareerHandle::shared().with(|career| mark_read(&mut career.settings.sms, id)).flatten();
    if effect.is_some_and(|effect| effect.play_voice.is_some()) {
        cell_call::request(id);
    }
}

/// Replays the voice of message `id`, cutting the line in progress. Only a listed story message replays.
pub fn replay_voice(id: u32) {
    let Some(id) = CareerHandle::shared().with(|career| replay_id(&career.settings.sms, id)).flatten() else {
        return;
    };
    cell_call::replay(id);
}

/// Deletes message `id` from the career's inbox.
pub fn delete(id: u32) {
    CareerHandle::shared().with(|career| delete_message(&mut career.settings.sms, id));
}

/// The language table's text for `label`, or the original's default-string error text when no language has it.
pub(super) fn lookup(label: &str) -> String {
    Text::Game(label_hash(label)).show()
}

/// The row's caption: the sender and the subject of message `id`.
fn row_caption(id: u32) -> String {
    format!("{}: {}", lookup(&from_label_for(id)), lookup(&subject_label_for(id)))
}

pub(super) fn show_object(cx: &mut Cx, guid: u32, shown: bool) {
    if let Some(o) = cx.guid(guid) {
        cx.rt.set_hidden(o, !shown);
    }
}

pub(super) fn set_text(cx: &mut Cx, guid: u32, text: String) {
    if let Some(o) = cx.guid(guid) {
        cx.rt.set_text(o, text);
    }
}

/// Shows or hides a tab's new-message mark with its own scripts.
fn show_mark(cx: &mut Cx, guid: u32, shown: bool) {
    let Some(o) = cx.guid(guid) else { return };
    let script = match shown {
        true => SHOW,
        false => HIDE,
    };
    cx.rt.run_script(o, script);
}

/// Fills one row of the list, or hides it when the page has no entry for it.
fn show_row(cx: &mut Cx, slot: usize, entry: Option<ListEntry>) {
    let Some(entry) = entry else {
        show_object(cx, ROW_BUTTONS[slot], false);
        return;
    };
    show_object(cx, ROW_BUTTONS[slot], true);
    let caption = row_caption(entry.id);
    set_text(cx, ROW_TEXTS[slot], caption);
    show_object(cx, ROW_NEW[slot], entry.unread);
}

/// The question the delete button asks (`0x8C3C2171`): its Yes answer deletes message `id`, then runs `then` (to close
/// the message screen; the list stays and drops the row).
pub fn delete_question(id: u32, then: Option<Command>) -> Confirm {
    Confirm::new(Text::Game(DELETE_QUESTION), Command::DeleteSms { id, then: then.map(Box::new) })
}

/// What leaving the message screen does: back to the safe house menu when the inbox was opened there, otherwise the screen
/// pops.
pub fn leave_command(args: Args) -> Command {
    if !args.safehouse {
        return Command::Pop;
    }
    Command::Switch(screen::MAIN_MENU_SUB.to_string(), Args { safehouse: true, ..Args::default() })
}

/// Back: to the safe house menu when the inbox was opened there, otherwise the screen pops.
pub(super) fn leave(cx: &mut Cx, args: Args) {
    cx.send(leave_command(args));
}

/// The inbox list of one tab, six rows at a time.
pub struct InboxLogic {
    args: Args,
    tab: InboxTab,
    /// The highlighted entry and the first entry of the page.
    at: usize,
    first: usize,
    entries: Vec<ListEntry>,
    /// A message opened from the list stands over it: the list is out and draws no hints.
    reading: bool,
}

impl InboxLogic {
    fn new(args: Args) -> Self {
        Self { args, tab: InboxTab::Story, at: 0, first: 0, entries: Vec::new(), reading: false }
    }

    /// Brings the tabs, the rows and the footer in.
    fn list_in(&mut self, cx: &mut Cx) {
        self.reading = false;
        cx.rt.post_to_package(cx.package, PANEL_IN);
        cx.rt.post_to_package(cx.package, LIST_IN);
    }

    /// The career's inbox: the entries of the tab and whether each tab has an unread message.
    fn read(&self) -> (Vec<ListEntry>, bool, bool) {
        let tab = self.tab;
        CareerHandle::shared()
            .with(|career| {
                let sms = &career.settings.sms;
                (sms.list(tab), sms.has_unread(InboxTab::Story), sms.has_unread(InboxTab::Tips))
            })
            .unwrap_or_default()
    }

    /// Reads the career's inbox again and fills the page and the tab icons.
    fn refresh(&mut self, cx: &mut Cx) {
        let (entries, story_unread, tips_unread) = self.read();
        self.entries = entries;
        (self.at, self.first) = moved(self.at, self.first, 0, self.entries.len());
        let shown = page(&self.entries, self.first);
        for slot in 0..ROWS {
            show_row(cx, slot, shown.get(slot).copied());
        }
        // The rows are inside a group that is out when the package loads, so the runtime gives none of them the focus.
        if !self.entries.is_empty() {
            cx.rt.set_focus(cx.package, ROW_BUTTONS[self.at - self.first]);
        }
        let (story_mark, tips_mark) = unread_marks(self.tab, story_unread, tips_unread);
        show_mark(cx, STORY_NEW, story_mark);
        show_mark(cx, TIPS_NEW, tips_mark);
    }

    /// Puts the corner frame and the pulse on the selected tab, as the package's scripts for the two tabs do.
    fn show_tab(&self, cx: &mut Cx) {
        let (selected, idle, frame) = match self.tab {
            InboxTab::Story => (STORY_TAB, TIPS_TAB, FRAME_AT_STORY),
            InboxTab::Tips => (TIPS_TAB, STORY_TAB, FRAME_AT_TIPS),
        };
        for (guid, script) in [(selected, TAB_SELECTED), (idle, TAB_IDLE), (TAB_FRAME, frame)] {
            if let Some(o) = cx.guid(guid) {
                cx.rt.run_script(o, script);
            }
        }
    }

    /// Asks to delete the highlighted message; the list drops it once the question is answered Yes.
    fn delete_focused(&mut self, cx: &mut Cx) {
        let Some(entry) = self.focused(cx) else { return };
        cx.send(Command::Confirm(delete_question(entry.id, None)));
    }

    fn move_highlight(&mut self, cx: &mut Cx, delta: i32) {
        self.sync(cx);
        (self.at, self.first) = moved(self.at, self.first, delta, self.entries.len());
        self.refresh(cx);
    }

    /// Takes the highlight from the row that has the focus (the pointer moves it).
    fn sync(&mut self, cx: &Cx) {
        let slot = cx.rt.focus(cx.package).and_then(|guid| ROW_BUTTONS.iter().position(|&row| row == guid));
        if let Some(slot) = slot.filter(|slot| self.first + slot < self.entries.len()) {
            self.at = self.first + slot;
        }
    }

    /// The entry under the highlight.
    fn focused(&mut self, cx: &Cx) -> Option<ListEntry> {
        self.sync(cx);
        self.entries.get(self.at).copied()
    }

    /// Opens the highlighted message. With the safe house args it replaces this screen, so Back goes to the safe house;
    /// otherwise the list goes out under the message and comes back when the message closes.
    fn open_focused(&mut self, cx: &mut Cx) {
        let Some(entry) = self.focused(cx) else { return };
        set_opened(entry.id);
        if self.args.safehouse {
            cx.send(Command::Switch(screen::SMS_MESSAGE.to_string(), self.args));
            return;
        }
        self.reading = true;
        cx.rt.post_to_package(cx.package, LIST_OUT);
        cx.send(Command::Push(screen::SMS_MESSAGE.to_string(), self.args));
    }
}

impl ScreenLogic for InboxLogic {
    fn start(&mut self, cx: &mut Cx) {
        self.show_tab(cx);
        self.refresh(cx);
        self.list_in(cx);
        cx.rt.post_to_package(cx.package, INPUT_ENABLE);
    }

    fn resume(&mut self, cx: &mut Cx) {
        self.refresh(cx);
        if self.reading {
            self.list_in(cx);
        }
    }

    fn message(&mut self, cx: &mut Cx, message: u32) {
        match message {
            PAD_BACK => leave(cx, self.args),
            ROW => self.open_focused(cx),
            DELETE => self.delete_focused(cx),
            TAB => {
                self.tab = other_tab(self.tab);
                (self.at, self.first) = (0, 0);
                self.show_tab(cx);
                self.refresh(cx);
            }
            UP => self.move_highlight(cx, -1),
            DOWN => self.move_highlight(cx, 1),
            _ => {}
        }
    }

    /// A message deleted from the list (the question answered Yes) leaves it on the next frame.
    fn tick(&mut self, cx: &mut Cx, _dt: f32) {
        if self.read().0 != self.entries {
            self.refresh(cx);
        }
    }

    fn scroll(&mut self, cx: &mut Cx, steps: i32) {
        self.move_highlight(cx, -steps);
    }

    fn footer(&self) -> Footer {
        Footer::SmsInbox { has_messages: !self.entries.is_empty(), reading: self.reading }
    }
}

#[cfg(test)]
#[path = "sms_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "sms_playtest.rs"]
mod playtest;

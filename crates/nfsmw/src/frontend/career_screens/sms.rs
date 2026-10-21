//! The SMS inbox: the list (`SMS_MailBoxes.fng`, its two tabs and six rows at a time) and one message
//! (`SMS_Message.fng`), `docs/specs/career-phone.md` section 4. The rules are in `nfsmw_career::phone` (the list order, the tabs, reading);
//! this file builds the rows from them, marks a message read when it is opened and shows its text from the language
//! table. Back goes to the safe house menu when the inbox was opened there, otherwise it pops the screen.
//!
//! `Args` carries no message id, so the list stores the id it opens in [`OPENED`] and the message screen reads it when
//! it starts. Voice replay and the delete action are not wired yet: `Cx` has no audio call, and the delete action needs
//! its confirmation and a command of its own.

use std::sync::Mutex;

use blackbox_feng::ids::PAD_BACK;
use blackbox_text::label_hash;
use nfsmw_career::phone::{InboxTab, ListEntry, from_label_for, is_story, label_for, subject_label_for};
use nfsmw_career::settings::SmsInbox;

use crate::career::CareerHandle;
use crate::frontend::ids::{INPUT_ENABLE, screen};
use crate::frontend::logic::{Args, Command, Cx, ScreenLogic};
use crate::ui::Text;

/// The messages the list sends: the Read button (`0x775CE5DF`), a row (`0x0C407210`), the tab hotspots (`0x9120409E`,
/// either one), and the list's arrows (`0x72619778` up, `0x911C0A4B` down).
const READ: u32 = 0x775C_E5DF;
const ROW: u32 = 0x0C40_7210;
const TAB: u32 = 0x9120_409E;
const UP: u32 = 0x7261_9778;
const DOWN: u32 = 0x911C_0A4B;

/// Rows the list shows at once. Each row is a button group with its subject text and its new-message icon.
const ROWS: usize = 6;
const ROW_BUTTONS: [u32; ROWS] = [0x000F_3257, 0x000F_327E, 0x000F_3286, 0x000F_328E, 0x000F_3296, 0x000F_329E];
const ROW_TEXTS: [u32; ROWS] = [0x000F_319B, 0x000F_327F, 0x000F_3287, 0x000F_328F, 0x000F_3297, 0x000F_329F];
const ROW_NEW: [u32; ROWS] = [0x000F_3203, 0x000F_3280, 0x000F_3288, 0x000F_3290, 0x000F_3298, 0x000F_32A0];
/// The new-message icons of the two tab widgets: the story tab's and the tips tab's.
const STORY_NEW: u32 = 0x000F_337E;
const TIPS_NEW: u32 = 0x000F_321F;

/// The message screen's objects: the body, the sender and subject values, and the replay hint and voice group that are
/// hidden for non-story messages (the voice group's role is read from its script only, unconfirmed).
const BODY: u32 = 0x000F_A6B1;
const FROM: u32 = 0x000E_5A97;
const SUBJECT: u32 = 0x000F_9301;
const REPLAY_HINT: u32 = 0x000F_A582;
const VOICE: u32 = 0x000F_4BA4;

/// The message the list opened last: the message screen shows it.
static OPENED: Mutex<Option<u32>> = Mutex::new(None);

fn opened() -> Option<u32> {
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
        return Some(Box::new(MessageLogic { args }));
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

/// The other tab.
pub const fn other_tab(tab: InboxTab) -> InboxTab {
    match tab {
        InboxTab::Story => InboxTab::Tips,
        InboxTab::Tips => InboxTab::Story,
    }
}

/// The new-message icons of the two tab widgets, `(story, tips)`: they mark unread messages in the tab the list is
/// not on.
pub const fn unread_marks(tab: InboxTab, story_unread: bool, tips_unread: bool) -> (bool, bool) {
    match tab {
        InboxTab::Story => (false, tips_unread),
        InboxTab::Tips => (story_unread, false),
    }
}

/// Marks message `id` read, as opening it does (state 4). False when the id is not in the list.
pub fn mark_read(inbox: &mut SmsInbox, id: u32) -> bool {
    inbox.open_message(id).is_some()
}

/// The language table's text for `label`, or the label itself in brackets when the table has no such label.
fn lookup(cx: &Cx, label: &str) -> String {
    Text::Game(label_hash(label)).resolve(cx.assets).unwrap_or_else(|| format!("[{label}]"))
}

/// The row's caption: the sender and the subject of message `id`.
fn row_caption(cx: &Cx, id: u32) -> String {
    format!("{}: {}", lookup(cx, &from_label_for(id)), lookup(cx, &subject_label_for(id)))
}

fn show_object(cx: &mut Cx, guid: u32, shown: bool) {
    if let Some(o) = cx.guid(guid) {
        cx.rt.set_hidden(o, !shown);
    }
}

fn set_text(cx: &mut Cx, guid: u32, text: String) {
    if let Some(o) = cx.guid(guid) {
        cx.rt.set_text(o, text);
    }
}

/// Fills one row of the list, or hides it when the page has no entry for it.
fn show_row(cx: &mut Cx, slot: usize, entry: Option<ListEntry>) {
    let Some(entry) = entry else {
        show_object(cx, ROW_BUTTONS[slot], false);
        return;
    };
    show_object(cx, ROW_BUTTONS[slot], true);
    let caption = row_caption(cx, entry.id);
    set_text(cx, ROW_TEXTS[slot], caption);
    show_object(cx, ROW_NEW[slot], entry.unread);
}

/// Back: to the safe house menu when the inbox was opened there, otherwise the screen pops.
fn leave(cx: &mut Cx, args: Args) {
    if !args.safehouse {
        cx.send(Command::Pop);
        return;
    }
    cx.send(Command::Switch(screen::MAIN_MENU_SUB.to_string(), Args { safehouse: true, ..Args::default() }));
}

/// The inbox list of one tab, six rows at a time.
pub struct InboxLogic {
    args: Args,
    tab: InboxTab,
    /// The first entry of the page.
    first: usize,
    entries: Vec<ListEntry>,
}

impl InboxLogic {
    fn new(args: Args) -> Self {
        Self { args, tab: InboxTab::Story, first: 0, entries: Vec::new() }
    }

    /// Reads the career's inbox again and fills the page and the tab icons.
    fn refresh(&mut self, cx: &mut Cx) {
        let tab = self.tab;
        let (entries, story_unread, tips_unread) = CareerHandle::shared()
            .with(|career| {
                let sms = &career.settings.sms;
                (sms.list(tab), sms.has_unread(InboxTab::Story), sms.has_unread(InboxTab::Tips))
            })
            .unwrap_or_default();
        self.entries = entries;
        self.first = scrolled(self.first, 0, self.entries.len());
        let shown = page(&self.entries, self.first);
        for slot in 0..ROWS {
            show_row(cx, slot, shown.get(slot).copied());
        }
        let (story_mark, tips_mark) = unread_marks(self.tab, story_unread, tips_unread);
        show_object(cx, STORY_NEW, story_mark);
        show_object(cx, TIPS_NEW, tips_mark);
    }

    fn move_page(&mut self, cx: &mut Cx, delta: i32) {
        self.first = scrolled(self.first, delta, self.entries.len());
        self.refresh(cx);
    }

    /// The entry under the highlight: the row that has the focus, or the first row when none has it.
    fn focused(&self, cx: &Cx) -> Option<ListEntry> {
        let shown = page(&self.entries, self.first);
        let slot = cx.rt.focus(cx.package).and_then(|guid| ROW_BUTTONS.iter().position(|&row| row == guid));
        shown.get(slot.unwrap_or(0)).copied()
    }

    /// Opens the highlighted message. With the safe house args it replaces this screen, so Back goes to the safe house.
    fn open_focused(&self, cx: &mut Cx) {
        let Some(entry) = self.focused(cx) else { return };
        set_opened(entry.id);
        let command = match self.args.safehouse {
            true => Command::Switch(screen::SMS_MESSAGE.to_string(), self.args),
            false => Command::Push(screen::SMS_MESSAGE.to_string(), self.args),
        };
        cx.send(command);
    }
}

impl ScreenLogic for InboxLogic {
    fn start(&mut self, cx: &mut Cx) {
        self.refresh(cx);
        cx.rt.post_to_package(cx.package, INPUT_ENABLE);
    }

    fn resume(&mut self, cx: &mut Cx) {
        self.refresh(cx);
    }

    fn message(&mut self, cx: &mut Cx, message: u32) {
        match message {
            PAD_BACK => leave(cx, self.args),
            READ | ROW => self.open_focused(cx),
            TAB => {
                self.tab = other_tab(self.tab);
                self.first = 0;
                self.refresh(cx);
            }
            UP => self.move_page(cx, -1),
            DOWN => self.move_page(cx, 1),
            _ => {}
        }
    }

    fn scroll(&mut self, cx: &mut Cx, steps: i32) {
        self.move_page(cx, -steps);
    }
}

/// One message: its body, sender and subject, read from the language table; opening it marks it read.
pub struct MessageLogic {
    args: Args,
}

/// Shows message `id` on the message screen and marks it read, as opening it does. The body, sender and subject come
/// from the language table; the replay hint and the voice group show for story messages only.
pub fn show_message(cx: &mut Cx, id: u32) {
    CareerHandle::shared().with(|career| mark_read(&mut career.settings.sms, id));
    let body = lookup(cx, &label_for(id));
    set_text(cx, BODY, body);
    let sender = lookup(cx, &from_label_for(id));
    set_text(cx, FROM, sender);
    let subject = lookup(cx, &subject_label_for(id));
    set_text(cx, SUBJECT, subject);
    let story = is_story(id);
    show_object(cx, REPLAY_HINT, story);
    show_object(cx, VOICE, story);
}

impl ScreenLogic for MessageLogic {
    fn start(&mut self, cx: &mut Cx) {
        let Some(id) = opened() else {
            leave(cx, self.args);
            return;
        };
        show_message(cx, id);
        cx.rt.post_to_package(cx.package, INPUT_ENABLE);
    }

    fn message(&mut self, cx: &mut Cx, message: u32) {
        if message == PAD_BACK {
            leave(cx, self.args);
        }
    }
}

#[cfg(test)]
#[path = "sms_tests.rs"]
mod tests;

//! Confirmation dialogs: a question with two answers, and what happens when the player says yes. The text of
//! each part is either the caller's own or a game string. A screen asks with `Command::Confirm`; the screens
//! open `Dialog.fng` with [`ConfirmLogic`] (docs/specs/confirmation-dialogs.md).

use blackbox_feng::ObjectRef;
use blackbox_feng::ids::{PAD_BACK, PAD_LEFT, PAD_RIGHT};

use super::ids;
use super::logic::{Command, Cx, ScreenLogic};
use crate::ui::Text;

/// The original quit question (English table `0x075CAC3B`).
pub const QUIT_MESSAGE: u32 = 0x075c_ac3b;
/// The answers the original dialogs use (English table `0x70E01038` "Yes", `0x417B25E4` "No").
pub const YES: u32 = 0x70e0_1038;
pub const NO: u32 = 0x417b_25e4;

/// A question with a yes and a no, and the command to run on yes (no runs nothing and closes the dialog).
#[derive(Clone, Debug, PartialEq)]
pub struct Confirm {
    /// The header above the question. The original's quit question has none (its caller passes an empty name).
    pub title: Option<Text>,
    pub message: Text,
    pub yes: Text,
    pub no: Text,
    pub on_yes: Box<Command>,
}

impl Confirm {
    /// A question with the game's own Yes and No answers.
    pub fn new(message: Text, on_yes: Command) -> Self {
        Self { title: None, message, yes: Text::Game(YES), no: Text::Game(NO), on_yes: Box::new(on_yes) }
    }

    /// The question the original asks before quitting to the main menu.
    pub fn quit_to_menu() -> Self {
        Self::new(Text::Game(QUIT_MESSAGE), Command::QuitToMenu)
    }
}

/// The objects of `Dialog.fng` (by name hash, as the package's dump shows them): the question, and the answer buttons.
/// The dialog shows two answers: Yes in the middle (Button 1) and No on the right (Button 2). Button 3 on the left stays
/// hidden, so that the package's left and right moves between the two answers.
const TITLE: u32 = 0x42ad_b44c;
const MESSAGE: u32 = 0x1e26_40fa;
const YES_BUTTON: u32 = 0xf936_3f30;
const NO_BUTTON: u32 = 0xfb8b_67d1;
const UNUSED_BUTTON: u32 = 0xfde0_9072;
/// The button holders (the objects that take the focus and send the press): Yes's, No's and the unused one's, by
/// GUID. The package's own left and right do not move the focus between them, so the logic does.
const YES_HOLDER: u32 = 0x000f_70e8;
const NO_HOLDER: u32 = 0x000f_72c2;
const UNUSED_HOLDER: u32 = 0x000f_731f;
/// Sent to the game when a button is pressed (the answer buttons' `BUTTON_PRESSED` response, `Dialog.fng`). Which
/// button it was is the package's focus.
const ANSWERED: u32 = 0x1b91_ebf4;

/// Shows a [`Confirm`]. No has the focus at first, so a stray accept cannot quit; the package moves the focus with
/// left and right, a press answers with the focused button, and back answers no.
pub struct ConfirmLogic {
    confirm: Confirm,
}

impl ConfirmLogic {
    pub fn new(confirm: Confirm) -> Self {
        Self { confirm }
    }

    fn answer(&self, cx: &mut Cx, yes: bool) {
        cx.send(Command::Pop);
        if yes {
            cx.send((*self.confirm.on_yes).clone());
        }
    }
}

/// Sets an object's text to a [`Text`]; a game string the table lacks leaves the object as the package has it.
fn set_text(cx: &mut Cx, name: u32, text: &Text) {
    let (Some(o), Some(value)) = (cx.object(name), text.resolve(cx.assets)) else { return };
    cx.rt.set_text(o, value);
}

/// The object with this GUID in the screen's package.
fn by_guid(cx: &Cx, guid: u32) -> Option<ObjectRef> {
    let def = cx.rt.package(cx.package)?;
    let index = def.objects.iter().position(|o| o.guid == guid)?;
    Some(ObjectRef { package: cx.package, index })
}

fn yes_is_focused(cx: &Cx) -> bool {
    let (Some(yes), Some(focus)) = (by_guid(cx, YES_HOLDER), cx.rt.current_button(cx.package)) else { return false };
    yes == focus
}

/// Makes a holder the current button.
fn focus(cx: &mut Cx, guid: u32) {
    if let Some(o) = by_guid(cx, guid) {
        cx.rt.focus_button(o);
    }
}

impl ScreenLogic for ConfirmLogic {
    fn start(&mut self, cx: &mut Cx) {
        cx.hide(UNUSED_BUTTON, true);
        match &self.confirm.title {
            Some(title) => set_text(cx, TITLE, title),
            None => {
                if let Some(o) = cx.object(TITLE) {
                    cx.rt.set_text(o, String::new());
                }
            }
        }
        set_text(cx, MESSAGE, &self.confirm.message);
        set_text(cx, YES_BUTTON, &self.confirm.yes);
        set_text(cx, NO_BUTTON, &self.confirm.no);
        if let Some(o) = by_guid(cx, UNUSED_HOLDER) {
            cx.rt.set_hidden(o, true);
        }
        focus(cx, NO_HOLDER);
    }

    fn message(&mut self, cx: &mut Cx, message: u32) {
        match message {
            ANSWERED => {
                let yes = yes_is_focused(cx);
                self.answer(cx, yes);
            }
            // The package sets its own focus when it has appeared, after `start`: No takes it back.
            ids::INIT_COMPLETE => focus(cx, NO_HOLDER),
            PAD_LEFT => focus(cx, YES_HOLDER),
            PAD_RIGHT => focus(cx, NO_HOLDER),
            PAD_BACK => self.answer(cx, false),
            _ => {}
        }
    }
}

/// The game's "Continue" (English table `0x8098A54C`), the one answer of an alert.
pub const CONTINUE: u32 = 0x8098_a54c;

/// The game's "save/load" title (English table `0x774E4F02`), the title of the save and load notices.
pub const SAVE_LOAD_TITLE: u32 = 0x774e_4f02;

/// A notice with one answer. Closing it (the answer or back) runs `on_close`, if the caller gave one.
#[derive(Clone, Debug, PartialEq)]
pub struct Alert {
    pub title: Text,
    pub message: Text,
    pub on_close: Option<Box<Command>>,
}

impl Alert {
    /// A notice titled "save/load" that does nothing when it is closed.
    pub fn new(message: Text) -> Self {
        Self { title: Text::Game(SAVE_LOAD_TITLE), message, on_close: None }
    }

    /// A notice that runs `then` when it is closed.
    #[allow(dead_code)] // no caller needs a follow-up yet
    pub fn then(message: Text, then: Command) -> Self {
        Self { title: Text::Game(SAVE_LOAD_TITLE), message, on_close: Some(Box::new(then)) }
    }
}

/// Shows an [`Alert`] in `Dialog.fng`: the middle button alone, labelled Continue, with the other answer and its
/// holder hidden. The package's left and right have nothing to move to, so the one answer keeps the focus.
pub struct AlertLogic {
    alert: Alert,
}

impl AlertLogic {
    pub fn new(alert: Alert) -> Self {
        Self { alert }
    }

    fn close(&self, cx: &mut Cx) {
        cx.send(Command::Pop);
        if let Some(then) = &self.alert.on_close {
            cx.send((**then).clone());
        }
    }
}

impl ScreenLogic for AlertLogic {
    fn start(&mut self, cx: &mut Cx) {
        cx.hide(UNUSED_BUTTON, true);
        cx.hide(NO_BUTTON, true);
        set_text(cx, MESSAGE, &self.alert.message);
        set_text(cx, ids::HEADER_TEXT, &self.alert.title);
        set_text(cx, YES_BUTTON, &Text::Game(CONTINUE));
        for guid in [NO_HOLDER, UNUSED_HOLDER] {
            if let Some(o) = by_guid(cx, guid) {
                cx.rt.set_hidden(o, true);
            }
        }
        focus(cx, YES_HOLDER);
    }

    fn message(&mut self, cx: &mut Cx, message: u32) {
        match message {
            ANSWERED | PAD_BACK => self.close(cx),
            ids::INIT_COMPLETE => focus(cx, YES_HOLDER),
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_alert_without_a_follow_up_closes_to_nothing() {
        let a = Alert::new(Text::Custom("Load complete.".into()));
        assert_eq!(a.on_close, None);
    }

    #[test]
    fn an_alert_remembers_what_to_run_when_closed() {
        let a = Alert::then(Text::Custom("Saved.".into()), Command::Resume);
        assert_eq!(a.on_close.as_deref(), Some(&Command::Resume));
    }

    #[test]
    fn game_answers_are_the_original_labels() {
        let c = Confirm::quit_to_menu();
        assert_eq!(c.message, Text::Game(QUIT_MESSAGE));
        assert_eq!(c.yes, Text::Game(YES));
        assert_eq!(c.no, Text::Game(NO));
        assert_eq!(*c.on_yes, Command::QuitToMenu);
    }

    #[test]
    fn custom_text_stays_as_given() {
        let c = Confirm::new(Text::Custom("Delete car?".into()), Command::QuitGame);
        assert_eq!(c.message, Text::Custom("Delete car?".into()));
        assert_eq!(*c.on_yes, Command::QuitGame);
    }
}

//! Confirmation dialogs: a question with two answers, and what happens when the player says yes. The text of
//! each part is either the caller's own or a game string. A screen asks with `Command::Confirm`; the screens
//! open `Dialog.fng` with [`ConfirmLogic`] (docs/specs/confirmation-dialogs.md).

use blackbox_feng::ObjectRef;
use blackbox_feng::ids::{PAD_BACK, PAD_LEFT, PAD_RIGHT};

use super::ids;
use super::logic::{Command, Cx, Pick, QuitTo, ScreenLogic};
use crate::ui::theme::{DEFAULT_ASPECT, MenuEntry, MenuList, MenuStyle};
use crate::ui::{Text, UiAssets};

/// The original quit question (English table `0x075CAC3B`).
pub const QUIT_MESSAGE: u32 = 0x075c_ac3b;
/// The quit and restart questions of the pause menu in a race (English table).
const QUIT_MAIN_MENU_RACE: u32 = 0xa2e9_b449;
const QUIT_QUICK_RACE_MENU: u32 = 0x1db1_cde5;
const QUIT_CAREER_MENU: u32 = 0xecd9_2696;
const RESTART_RACE: u32 = 0x4d33_99a8;
/// The answers the original dialogs use (English table `0x70E01038` "Yes", `0x417B25E4` "No").
pub const YES: u32 = 0x70e0_1038;
pub const NO: u32 = 0x417b_25e4;
/// The answers of the in-race restart question ("OK" `0x417B2601`, "Cancel" `0x1A294DAD`, the pair the game's alerts use).
pub const OK: u32 = 0x417b_2601;
pub const CANCEL: u32 = 0x1a29_4dad;

/// The header and picture of a dialog (`feDialogScreen`, the dialog's title kind): the alert's "attention" over a
/// triangle with a mark, the confirmation's "selection" over a question mark (docs/specs/challenge-series.md).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Look {
    Attention,
    Selection,
}

impl Look {
    /// The header label and the picture texture of the look (the original's `0x6FD91524` and `0x60249A74` labels).
    fn parts(self) -> (u32, u32) {
        match self {
            Look::Attention => (0x6fd9_1524, 0x6948_e2b3),
            Look::Selection => (0x6024_9a74, 0x3994_9433),
        }
    }
}

/// The picture objects of `Dialog.fng` (the header's icon and the big faint one behind the panel).
const ICON_OBJECTS: [u32; 2] = [0x0d4f_4069, 0xfac8_8427];

/// A question with a yes and a no, and the command to run on yes (no runs nothing and closes the dialog, unless the
/// caller gave a command for no). Back closes the dialog without answering.
#[derive(Clone, Debug, PartialEq)]
pub struct Confirm {
    /// The header above the question. The original's quit question has none (its caller passes an empty name).
    pub title: Option<Text>,
    pub message: Text,
    pub yes: Text,
    pub no: Text,
    pub on_yes: Box<Command>,
    /// What the second answer runs; none runs nothing.
    pub on_no: Option<Box<Command>>,
    /// The first answer has the focus when the dialog opens (the second has it otherwise).
    pub focus_yes: bool,
    /// The dialog's header and picture; none leaves the package's own.
    pub look: Option<Look>,
}

impl Confirm {
    /// A question with the game's own Yes and No answers.
    pub fn new(message: Text, on_yes: Command) -> Self {
        Self {
            title: None,
            message,
            yes: Text::Game(YES),
            no: Text::Game(NO),
            on_yes: Box::new(on_yes),
            on_no: None,
            focus_yes: false,
            look: None,
        }
    }

    /// Gives the dialog a header and a picture.
    pub fn looking(mut self, look: Look) -> Self {
        self.title = Some(Text::Game(look.parts().0));
        self.look = Some(look);
        self
    }

    /// Two named answers that each run a command (the transmission choice): `first` on the left, `second` on the right.
    pub fn choice(message: Text, first: (Text, Command), second: (Text, Command)) -> Self {
        let mut confirm = Self::new(message, first.1);
        confirm.yes = first.0;
        confirm.no = second.0;
        confirm.on_no = Some(Box::new(second.1));
        confirm
    }

    /// The first answer has the focus (or not) when the dialog opens.
    pub fn focusing_first(mut self, first: bool) -> Self {
        self.focus_yes = first;
        self
    }

    /// The same question with `command` run when it is answered No.
    pub fn or_else(mut self, command: Command) -> Self {
        self.on_no = Some(Box::new(command));
        self
    }

    /// The question before quitting to `to`. A race in progress adds that its progress will be lost (the labels
    /// `0xA2E9B449`, `0x1DB1CDE5` and `0xECD92696`; free roam has no progress, so the main menu keeps the plain question).
    pub fn quit_to(to: QuitTo, race: bool) -> Self {
        let message = match (to, race) {
            (QuitTo::Main, false) => QUIT_MESSAGE,
            (QuitTo::Main, true) => QUIT_MAIN_MENU_RACE,
            (QuitTo::QuickRace, _) => QUIT_QUICK_RACE_MENU,
            (QuitTo::Career, _) => QUIT_CAREER_MENU,
        };
        Self::new(Text::Game(message), Command::QuitTo(to))
    }

    /// The question before leaving the game from the main menu: the quit question under the question-mark header.
    pub fn quit_game() -> Self {
        Self::new(Text::Game(QUIT_MESSAGE), Command::QuitGame).looking(Look::Selection)
    }

    /// The restart question of the post-race screens: OK answers the race with a restart.
    pub fn restart_results() -> Self {
        Self::restart_race(Command::RaceAnswer(crate::viewer::RaceAnswer::Restart))
    }

    /// The question before running the race again (`0x4D3399A8`), as the game asks it: the attention header and triangle,
    /// the answers OK and Cancel, Cancel focused. OK runs `on_ok` (the pause menu's restart, or the results screen's).
    pub fn restart_race(on_ok: Command) -> Self {
        let mut confirm = Self::new(Text::Game(RESTART_RACE), on_ok).looking(Look::Attention);
        confirm.yes = Text::Game(OK);
        confirm.no = Text::Game(CANCEL);
        confirm
    }
}

/// The objects of `Dialog.fng` (by name hash, as the package's dump shows them): the question, and the answer buttons.
/// The dialog shows two answers: Yes in the middle (Button 1) and No on the right (Button 2). Button 3 on the left stays
/// hidden, so that the package's left and right moves between the two answers.
const TITLE: u32 = 0x42ad_b44c;
pub(super) const MESSAGE: u32 = 0x1e26_40fa;
pub(super) const YES_BUTTON: u32 = 0xf936_3f30;
pub(super) const NO_BUTTON: u32 = 0xfb8b_67d1;
const UNUSED_BUTTON: u32 = 0xfde0_9072;
/// The button holders (the objects that take the focus and send the press): Yes's, No's and the unused one's, by
/// GUID. The package's own left and right do not move the focus between them, so the logic does.
const YES_HOLDER: u32 = 0x000f_70e8;
const NO_HOLDER: u32 = 0x000f_72c2;
const UNUSED_HOLDER: u32 = 0x000f_731f;
/// Sent to the game when a button is pressed (the answer buttons' `BUTTON_PRESSED` response, `Dialog.fng`). Which
/// button it was is the package's focus.
pub(super) const ANSWERED: u32 = 0x1b91_ebf4;

/// Shows a [`Confirm`]. No has the focus at first, so a stray accept cannot quit; the package moves the focus with
/// left and right, a press answers with the focused button, and back answers no.
pub struct ConfirmLogic {
    confirm: Confirm,
    /// Whether the Remastered look draws this dialog as a panel (`Cx::vertical_look`), and whether Yes has the focus.
    vertical: bool,
    yes: bool,
}

impl ConfirmLogic {
    pub fn new(confirm: Confirm) -> Self {
        Self { confirm, vertical: false, yes: false }
    }

    fn answer(&self, cx: &mut Cx, yes: bool) {
        cx.send(Command::Pop);
        if yes {
            cx.send((*self.confirm.on_yes).clone());
            return;
        }
        if let Some(no) = &self.confirm.on_no {
            cx.send((**no).clone());
        }
    }

    /// The holder of the answer that has the focus when the dialog opens.
    fn first_holder(&self) -> u32 {
        if self.confirm.focus_yes { YES_HOLDER } else { NO_HOLDER }
    }
}

/// Sets an object's text to a [`Text`]; a game string the table lacks leaves the object as the package has it.
pub(super) fn set_text(cx: &mut Cx, name: u32, text: &Text) {
    let (Some(o), Some(value)) = (cx.object(name), text.resolve()) else { return };
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
        self.vertical = cx.vertical_look();
        cx.hide(UNUSED_BUTTON, true);
        match &self.confirm.title {
            Some(title) => set_text(cx, TITLE, title),
            None => {
                if let Some(o) = cx.object(TITLE) {
                    cx.rt.set_text(o, String::new());
                }
            }
        }
        if let Some(look) = self.confirm.look {
            for name in ICON_OBJECTS {
                if let Some(o) = cx.object(name) {
                    cx.rt.set_texture(o, look.parts().1);
                }
            }
        }
        set_text(cx, MESSAGE, &self.confirm.message);
        set_text(cx, YES_BUTTON, &self.confirm.yes);
        set_text(cx, NO_BUTTON, &self.confirm.no);
        if let Some(o) = by_guid(cx, UNUSED_HOLDER) {
            cx.rt.set_hidden(o, true);
        }
        focus(cx, self.first_holder());
    }

    fn tick(&mut self, cx: &mut Cx, _dt: f32) {
        self.vertical = cx.vertical_look();
        self.yes = yes_is_focused(cx);
    }

    fn vertical(&self) -> bool {
        self.vertical
    }

    /// The answers are one above the other, so Up and Down are the package's Left and Right.
    fn rotate_pad(&self) -> bool {
        self.vertical
    }

    fn pick_row(&mut self, cx: &mut Cx, row: usize, pick: Pick) {
        match pick {
            Pick::Hover => focus(cx, if row == 0 { YES_HOLDER } else { NO_HOLDER }),
            Pick::Press(..) => {
                crate::audio::menu::request(blackbox_feng::ids::BUTTON_PRESSED);
                self.answer(cx, row == 0);
            }
        }
    }

    fn list(&self, _assets: &UiAssets) -> Option<MenuList> {
        if !self.vertical {
            return None;
        }
        let resolve = |text: &Text| text.resolve().unwrap_or_default();
        let c = &self.confirm;
        Some(MenuList {
            title: c.title.as_ref().map(resolve).unwrap_or_default(),
            message: resolve(&c.message),
            entries: vec![MenuEntry::plain(resolve(&c.yes), true), MenuEntry::plain(resolve(&c.no), true)],
            selected: if self.yes { 0 } else { 1 },
            first: 0,
            hints: Vec::new(),
            style: MenuStyle::Dialog,
            nudge: None,
            scroll: 0.0,
            aspect: DEFAULT_ASPECT,
        })
    }

    /// The focus follows the pointer over the two answers, so the one under it shows as selected before a click.
    fn hover(&mut self, cx: &mut Cx, point: glam::Vec2) {
        if self.vertical {
            return;
        }
        let (Some(yes), Some(no)) = (by_guid(cx, YES_HOLDER), by_guid(cx, NO_HOLDER)) else { return };
        if let Some(hit) = cx.rt.object_at(&[yes, no], point) {
            cx.rt.focus_button(hit);
        }
    }

    /// A click answers the dialog only on Yes or No. The dialog takes every click, so one that misses both does nothing
    /// and does not reach the screen or the map under it.
    fn click(&mut self, cx: &mut Cx, point: glam::Vec2) -> bool {
        if self.vertical {
            return true;
        }
        let (Some(yes), Some(no)) = (by_guid(cx, YES_HOLDER), by_guid(cx, NO_HOLDER)) else { return true };
        if let Some(hit) = cx.rt.object_at(&[yes, no], point) {
            cx.rt.focus_button(hit);
            crate::audio::menu::request(blackbox_feng::ids::BUTTON_PRESSED);
            self.answer(cx, hit == yes);
        }
        true
    }

    fn message(&mut self, cx: &mut Cx, message: u32) {
        match message {
            ANSWERED => {
                let yes = yes_is_focused(cx);
                self.answer(cx, yes);
            }
            // The package sets its own focus when it has appeared, after `start`: the first answer takes it back.
            ids::INIT_COMPLETE => focus(cx, self.first_holder()),
            PAD_LEFT => focus(cx, YES_HOLDER),
            PAD_RIGHT => focus(cx, NO_HOLDER),
            PAD_BACK => cx.send(Command::Pop),
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
    pub fn then(message: Text, then: Command) -> Self {
        Self { title: Text::Game(SAVE_LOAD_TITLE), message, on_close: Some(Box::new(then)) }
    }
}

/// Shows an [`Alert`] in `Dialog.fng`: the middle button alone, labelled Continue, with the other answer and its
/// holder hidden. The package's left and right have nothing to move to, so the one answer keeps the focus.
pub struct AlertLogic {
    alert: Alert,
    vertical: bool,
}

/// Sets `Dialog.fng` up as a notice: the middle button alone, labelled Continue, with the focus on it.
pub(super) fn prepare_notice(cx: &mut Cx) {
    cx.hide(UNUSED_BUTTON, true);
    cx.hide(NO_BUTTON, true);
    set_text(cx, YES_BUTTON, &Text::Game(CONTINUE));
    for guid in [NO_HOLDER, UNUSED_HOLDER] {
        if let Some(o) = by_guid(cx, guid) {
            cx.rt.set_hidden(o, true);
        }
    }
    focus(cx, YES_HOLDER);
}

impl AlertLogic {
    pub fn new(alert: Alert) -> Self {
        Self { alert, vertical: false }
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
        self.vertical = cx.vertical_look();
        prepare_notice(cx);
        set_text(cx, MESSAGE, &self.alert.message);
        set_text(cx, ids::HEADER_TEXT, &self.alert.title);
    }

    fn tick(&mut self, cx: &mut Cx, _dt: f32) {
        self.vertical = cx.vertical_look();
    }

    fn vertical(&self) -> bool {
        self.vertical
    }

    fn click(&mut self, _cx: &mut Cx, _point: glam::Vec2) -> bool {
        // The notice takes every click, so none reaches the screen under it.
        self.vertical
    }

    fn pick_row(&mut self, cx: &mut Cx, _row: usize, pick: Pick) {
        if matches!(pick, Pick::Press(..)) {
            crate::audio::menu::request(blackbox_feng::ids::BUTTON_PRESSED);
            self.close(cx);
        }
    }

    fn list(&self, _assets: &UiAssets) -> Option<MenuList> {
        if !self.vertical {
            return None;
        }
        let resolve = |text: &Text| text.resolve().unwrap_or_default();
        Some(MenuList {
            title: resolve(&self.alert.title),
            message: resolve(&self.alert.message),
            entries: vec![MenuEntry::plain(resolve(&Text::Game(CONTINUE)), true)],
            selected: 0,
            first: 0,
            hints: Vec::new(),
            style: MenuStyle::Dialog,
            nudge: None,
            scroll: 0.0,
            aspect: DEFAULT_ASPECT,
        })
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
        let a = Alert::new(Text::Literal("Load complete.".into()));
        assert_eq!(a.on_close, None);
    }

    #[test]
    fn an_alert_remembers_what_to_run_when_closed() {
        let a = Alert::then(Text::Literal("Saved.".into()), Command::Resume);
        assert_eq!(a.on_close.as_deref(), Some(&Command::Resume));
    }

    #[test]
    fn game_answers_are_the_original_labels() {
        let c = Confirm::quit_to(QuitTo::Main, false);
        assert_eq!(c.message, Text::Game(QUIT_MESSAGE));
        assert_eq!(c.yes, Text::Game(YES));
        assert_eq!(c.no, Text::Game(NO));
        assert_eq!(*c.on_yes, Command::QuitTo(QuitTo::Main));
    }

    #[test]
    fn the_restart_question_is_the_attention_dialog_with_ok_and_cancel() {
        let c = Confirm::restart_race(Command::RestartRace);
        assert_eq!(c.message, Text::Game(0x4d33_99a8));
        assert_eq!((c.yes, c.no), (Text::Game(0x417b_2601), Text::Game(0x1a29_4dad)));
        assert_eq!(c.title, Some(Text::Game(0x6fd9_1524)), "the attention header");
        assert_eq!(c.look, Some(Look::Attention));
        assert!(!c.focus_yes, "Cancel has the focus");
        assert_eq!(*c.on_yes, Command::RestartRace);
        assert_eq!(c.on_no, None, "Cancel only closes the dialog");
    }

    #[test]
    fn custom_text_stays_as_given() {
        let c = Confirm::new(Text::Literal("Delete car?".into()), Command::QuitGame);
        assert_eq!(c.message, Text::Literal("Delete car?".into()));
        assert_eq!(*c.on_yes, Command::QuitGame);
    }
}

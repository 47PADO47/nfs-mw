//! Naming an alias (docs/specs/career-parity-menus.md, section 3): the keyboard prompt of the alias manager's Create and the
//! career menu's Start New Career. The original asks for the name with the keyboard group of `MC_Main.fng` (`Enter name:`);
//! the PC reads the typed text, which this notice shows as it is typed. Accept takes the name, Back leaves.

use blackbox_feng::ids::PAD_BACK;
use nfsmw_career::savefile::state::ALIAS_MAX_CHARS;

use super::career_screens::save::{list_aliases, next_alias};
use super::confirm::{Alert, set_text};
use super::logic::{Command, Cx, Pick, ScreenLogic};
use super::{confirm, ids};
use crate::i18n::Args;
use crate::ui::theme::{DEFAULT_ASPECT, MenuEntry, MenuList, MenuStyle};
use crate::ui::{Text, UiAssets};

/// "Enter name:" (`0x22729437`), the prompt, and the game's "Alias Manager" (`0xBCB18F38`), the header.
const PROMPT: u32 = 0x2272_9437;
const HEADER: u32 = 0xBCB1_8F38;

/// What the typed name is for.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NameFor {
    /// The alias manager's Create: a new alias, loaded but not played.
    Create,
    /// The career menu's Start New Career: a new alias, played at once.
    NewCareer,
}

impl NameFor {
    fn command(self, name: String) -> Command {
        match self {
            NameFor::Create => Command::AliasCreate(name),
            NameFor::NewCareer => Command::CareerNew(name),
        }
    }
}

/// A key typed while a screen takes text.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TextKey {
    Char(char),
    Backspace,
}

/// The alias being typed.
pub struct NameEntry {
    purpose: NameFor,
    text: String,
    vertical: bool,
}

impl NameEntry {
    /// The prompt starts with the first free `PLAYER` name, which Accept takes as it is.
    pub fn new(purpose: NameFor) -> Self {
        Self { purpose, text: next_alias(&list_aliases()), vertical: false }
    }

    /// The text the notice shows: the prompt, the typed name and a caret.
    fn shown(&self, prompt: &str) -> String {
        format!("{prompt} {}_", self.text)
    }

    fn prompt(&self) -> String {
        Text::Game(PROMPT).show()
    }

    fn refresh(&self, cx: &mut Cx) {
        let message = self.shown(&self.prompt());
        set_text(cx, confirm::MESSAGE, &Text::Literal(message));
    }

    /// Takes the name when it is usable, else says why not.
    fn accept(&self, cx: &mut Cx) {
        if self.text.is_empty() {
            return;
        }
        if list_aliases().iter().any(|alias| alias.eq_ignore_ascii_case(&self.text)) {
            let notice = Text::Key("dialogs-alias-exists", Args::new().with("alias", self.text.as_str()));
            cx.send(Command::Alert(Alert::new(notice)));
            return;
        }
        cx.send(Command::Pop);
        cx.send(self.purpose.command(self.text.clone()));
    }
}

/// Whether a typed character can be part of an alias: letters, digits, `_` and `-` (no space, which the list hides).
pub fn allowed(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '_' || c == '-'
}

impl ScreenLogic for NameEntry {
    fn start(&mut self, cx: &mut Cx) {
        self.vertical = cx.list_look();
        confirm::prepare_notice(cx);
        set_text(cx, ids::HEADER_TEXT, &Text::Game(HEADER));
        self.refresh(cx);
    }

    fn tick(&mut self, cx: &mut Cx, _dt: f32) {
        self.vertical = cx.list_look();
    }

    fn vertical(&self) -> bool {
        self.vertical
    }

    fn wants_text(&self) -> bool {
        true
    }

    fn typed(&mut self, cx: &mut Cx, key: TextKey) {
        match key {
            TextKey::Backspace => {
                self.text.pop();
            }
            TextKey::Char(c) if allowed(c) && self.text.chars().count() < ALIAS_MAX_CHARS => self.text.push(c),
            TextKey::Char(_) => {}
        }
        self.refresh(cx);
    }

    fn click(&mut self, _cx: &mut Cx, _point: glam::Vec2) -> bool {
        self.vertical
    }

    fn pick_row(&mut self, cx: &mut Cx, _row: usize, pick: Pick) {
        if matches!(pick, Pick::Press(..)) {
            let before = cx.commands.len();
            self.accept(cx);
            if matches!(cx.commands.get(before), Some(Command::Pop)) {
                crate::audio::menu::request(blackbox_feng::ids::BUTTON_PRESSED);
            }
        }
    }

    fn list(&self, _assets: &UiAssets) -> Option<MenuList> {
        if !self.vertical {
            return None;
        }
        let resolve = |text: &Text| text.resolve().unwrap_or_default();
        Some(MenuList {
            title: resolve(&Text::Game(HEADER)),
            message: self.shown(&self.prompt()),
            entries: vec![MenuEntry::plain(resolve(&Text::Game(confirm::CONTINUE)), true)],
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
            confirm::ANSWERED => self.accept(cx),
            PAD_BACK => cx.send(Command::Pop),
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_alias_takes_letters_digits_underscore_and_dash() {
        assert!(allowed('a') && allowed('Z') && allowed('7') && allowed('_') && allowed('-'));
        assert!(!allowed(' ') && !allowed('.') && !allowed('/') && !allowed('é'));
    }

    #[test]
    fn the_notice_shows_the_prompt_the_name_and_a_caret() {
        let mut entry = NameEntry::new(NameFor::Create);
        entry.text = "mwo".into();
        assert_eq!(entry.shown("Enter name:"), "Enter name: mwo_");
    }

    #[test]
    fn the_purpose_chooses_the_command() {
        assert_eq!(NameFor::Create.command("A".into()), Command::AliasCreate("A".into()));
        assert_eq!(NameFor::NewCareer.command("A".into()), Command::CareerNew("A".into()));
    }
}

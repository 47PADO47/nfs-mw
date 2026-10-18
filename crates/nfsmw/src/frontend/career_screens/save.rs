//! The career save screens (`docs/specs/career-save-screens.md`, `save-alias-manager.md`). `MC_Main.fng` is the manual save
//! prompt: the safe house Save item opens it, it asks "save the alias named X?" with Yes and No, and it returns to the
//! safe house menu either way. `MC_List.fng` is the profile list (nine rows in the spec; eight in the package): a pick
//! loads that profile or asks to delete it. The screens only show and dispatch; the profile work is `frontend/flow_save.rs`.
//!
//! Screens cannot reach the front end's profile folder, so they list `ProfileDir::default_dir()`, read only. A screenshot
//! run therefore lists the player's real folder while it saves into its temp folder. Delete asks with the dialog
//! (`Command::Confirm`); the original never opens `MC_DeleteProfile.fng`, so this build does not use it.
//!
//! The list's mode (load or delete) is set just before the list is pushed: a screen gets no constructor data.

use std::sync::Mutex;

use blackbox_feng::ids::{PAD_ACCEPT, PAD_BACK, PAD_DOWN, PAD_LEFT, PAD_RIGHT, PAD_UP};

use super::{INITIALIZE_SCREEN, MC_MAIN_PREPARE, post_opening};
use crate::career::{CareerHandle, ProfileDir};
use crate::frontend::confirm::{Alert, Confirm};
use crate::frontend::ids::{self, SCRIPT_HIGHLIGHT, SCRIPT_UNHIGHLIGHT, screen};
use crate::frontend::logic::{Args, Command, Cx, ScreenLogic};
use crate::ui::Text;

/// The rows the list shows at once (`option_name_1` to `option_name_8`). The dump of `MC_List.fng` has eight row names;
/// the spec says nine, and `option_name_9` is not in the package.
pub const ROWS: usize = 8;

/// The list's title text object (guid `0xF6B4E`, the string "load"). Its label is the verb: `0x18ECFF` ("Load") or the
/// delete label. The delete label is a guess among three "Delete" strings in the table; see [`title_label`].
const TITLE: u32 = 0x000F_6B4E;
const TITLE_LOAD: u32 = 0x0018_ECFF;
const TITLE_DELETE: u32 = 0x99A5_3F16;

/// The prompt's objects (`MC_Main.fng`, by guid from the dump). The question is the "Generic Message" string. The two
/// button texts are "Button 1" (right, No) and "Button 2" (centre, Yes); "Button 3" and its button (left) are hidden.
const QUESTION: u32 = 0x000F_7CF7;
const NO_TEXT: u32 = 0x000F_7D04;
const YES_TEXT: u32 = 0x000F_7D05;
const THIRD_TEXT: u32 = 0x000F_7D06;
const THIRD_BUTTON: u32 = 0x000F_7D53;
/// The labels of the two buttons: "No" and "Yes".
const NO_LABEL: u32 = 0x417B_25E4;
const YES_LABEL: u32 = 0x70E0_1038;
/// The manual-save question, "Are you sure you want to save the alias named %s?" (`%s` is the alias).
const SAVE_QUESTION: u32 = 0x39B3_CCBA;
/// The question when the table has no text for [`SAVE_QUESTION`] (or it has no `%s` to fill).
const SAVE_QUESTION_FALLBACK: &str = "Save your career progress?";

/// The label the title shows for `mode`.
pub(crate) const fn title_label(mode: ListMode) -> u32 {
    match mode {
        ListMode::Load => TITLE_LOAD,
        ListMode::Delete => TITLE_DELETE,
    }
}

/// What a pick on the list does.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ListMode {
    Load,
    Delete,
}

/// The mode the next list opened shows. Set just before `MC_List.fng` is pushed.
static LIST_MODE: Mutex<ListMode> = Mutex::new(ListMode::Load);

/// Sets the mode the next list opened shows (`alias_options.rs` opens the list in load or delete mode).
pub(crate) fn set_list_mode(mode: ListMode) {
    *LIST_MODE.lock().unwrap_or_else(|e| e.into_inner()) = mode;
}

fn list_mode() -> ListMode {
    *LIST_MODE.lock().unwrap_or_else(|e| e.into_inner())
}

/// The question the prompt asks: the save question with the alias filled in, or the fallback without an alias.
pub fn question(template: Option<&str>, alias: Option<&str>) -> String {
    let (Some(template), Some(alias)) = (template, alias) else { return SAVE_QUESTION_FALLBACK.to_owned() };
    if !template.contains("%s") {
        return SAVE_QUESTION_FALLBACK.to_owned();
    }
    template.replace("%s", alias)
}

/// What Accept does on the prompt.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Answer {
    /// Save the career, then go back to the safe house menu.
    Save,
    /// Go back to the safe house menu without saving.
    Leave,
}

/// The answer Accept gives: Yes saves when there is an alias to save; No, or Yes without an alias, only leaves.
pub const fn answer(yes: bool, has_alias: bool) -> Answer {
    match (yes, has_alias) {
        (true, true) => Answer::Save,
        _ => Answer::Leave,
    }
}

/// The name a new career gets when the folder holds `taken`: `PLAYER`, then `PLAYER2` to `PLAYER9`, then `P10` on.
/// Names are compared without case, as the Windows folder is. Every name is shorter than 8 characters without a space,
/// so the list finds it (`career/io.rs`).
pub(crate) fn next_alias(taken: &[String]) -> String {
    let mut number = 1;
    loop {
        let name = alias_candidate(number);
        if !taken.iter().any(|t| t.eq_ignore_ascii_case(&name)) {
            return name;
        }
        number += 1;
    }
}

fn alias_candidate(number: u32) -> String {
    match number {
        1 => "PLAYER".to_owned(),
        2..=9 => format!("PLAYER{number}"),
        _ => format!("P{number}"),
    }
}

/// The first list index on the page that holds `selected`.
pub fn page_start(selected: usize) -> usize {
    selected / ROWS * ROWS
}

/// The names on the page that holds `selected`, at most [`ROWS`].
pub fn page_rows(aliases: &[String], selected: usize) -> Vec<String> {
    aliases.iter().skip(page_start(selected)).take(ROWS).cloned().collect()
}

/// The selection one step `down` or up, kept inside `count` names.
pub fn moved(selected: usize, count: usize, down: bool) -> usize {
    if count == 0 {
        return 0;
    }
    if down {
        return (selected + 1).min(count - 1);
    }
    selected.saturating_sub(1)
}

/// The text of a list row; the selected one is marked.
pub fn row_text(alias: &str, selected: bool) -> String {
    if selected {
        return format!("> {alias}");
    }
    alias.to_owned()
}

/// The profiles the list shows: the folder's listable aliases, or none when there is no folder.
pub(crate) fn list_aliases() -> Vec<String> {
    ProfileDir::default_dir().map(|dir| dir.aliases()).unwrap_or_default()
}

/// The safe house menu the prompt returns to, with the safe house flag set.
fn back_to_safehouse(cx: &mut Cx, args: Args) {
    let args = Args { safehouse: true, ..args };
    cx.send(Command::Switch(screen::MAIN_MENU_SUB.into(), args));
}

/// The manual save prompt (`MC_Main.fng`). Left and Up pick Yes, Right and Down pick No (No starts focused). Accept
/// carries out the answer; Back leaves without saving.
pub struct SavePromptLogic {
    args: Args,
    alias: Option<String>,
    yes: bool,
}

impl SavePromptLogic {
    fn new(args: Args) -> Self {
        Self { args, alias: None, yes: false }
    }

    /// Fills the question and the two labels, hides the third button and focuses the selected answer.
    fn show(&self, cx: &mut Cx) {
        let template = Text::Game(SAVE_QUESTION).resolve(cx.assets);
        let text = question(template.as_deref(), self.alias.as_deref());
        if let Some(q) = cx.guid(QUESTION) {
            cx.rt.set_text(q, text);
        }
        for (guid, label) in [(NO_TEXT, NO_LABEL), (YES_TEXT, YES_LABEL)] {
            let Some(o) = cx.guid(guid) else { continue };
            cx.label(o, label);
        }
        if let Some(third) = cx.guid(THIRD_TEXT) {
            cx.rt.set_hidden(third, true);
        }
        if let Some(third) = cx.guid(THIRD_BUTTON) {
            cx.rt.set_hidden(third, true);
        }
        self.focus(cx);
    }

    /// Highlights the selected answer's text and clears the other.
    fn focus(&self, cx: &mut Cx) {
        let (yes_script, no_script) = match self.yes {
            true => (SCRIPT_HIGHLIGHT, SCRIPT_UNHIGHLIGHT),
            false => (SCRIPT_UNHIGHLIGHT, SCRIPT_HIGHLIGHT),
        };
        if let Some(o) = cx.guid(YES_TEXT) {
            cx.rt.run_script(o, yes_script);
        }
        if let Some(o) = cx.guid(NO_TEXT) {
            cx.rt.run_script(o, no_script);
        }
    }

    fn accept(&self, cx: &mut Cx) {
        match answer(self.yes, self.alias.is_some()) {
            Answer::Save => cx.send(Command::CareerSave),
            Answer::Leave => {}
        }
        back_to_safehouse(cx, self.args);
    }
}

impl ScreenLogic for SavePromptLogic {
    fn start(&mut self, cx: &mut Cx) {
        post_opening(cx, &[MC_MAIN_PREPARE, INITIALIZE_SCREEN]);
        self.alias = CareerHandle::shared().with(|career| career.alias.clone());
        cx.rt.post_to_package(cx.package, ids::INPUT_ENABLE);
        self.show(cx);
    }

    fn message(&mut self, cx: &mut Cx, message: u32) {
        match message {
            PAD_LEFT | PAD_UP => {
                self.yes = true;
                self.focus(cx);
            }
            PAD_RIGHT | PAD_DOWN => {
                self.yes = false;
                self.focus(cx);
            }
            PAD_ACCEPT => self.accept(cx),
            PAD_BACK => back_to_safehouse(cx, self.args),
            _ => {}
        }
    }
}

/// The profile list (`MC_List.fng`). A pick loads that profile and plays it, or asks to delete it.
pub struct AliasListLogic {
    #[allow(dead_code)] // kept for the screen the alias manager returns to
    args: Args,
    mode: ListMode,
    aliases: Vec<String>,
    selected: usize,
}

impl AliasListLogic {
    fn new(args: Args, mode: ListMode) -> Self {
        Self { args, mode, aliases: Vec::new(), selected: 0 }
    }

    /// Shows the title for the list's mode.
    fn show_title(&self, cx: &mut Cx) {
        let Some(title) = cx.guid(TITLE) else { return };
        cx.label(title, title_label(self.mode));
    }

    /// Shows the page that holds the selection, with the selection marked (the highlight script, as the jukebox
    /// does for its rows).
    fn show(&self, cx: &mut Cx) {
        let start = page_start(self.selected);
        let rows = page_rows(&self.aliases, self.selected);
        for index in 0..ROWS {
            let selected = start + index == self.selected;
            let text = match rows.get(index) {
                Some(alias) => row_text(alias, selected),
                None => String::new(),
            };
            let Some(row) = cx.named(&format!("option_name_{}", index + 1)) else { continue };
            cx.rt.set_text(row, text);
            let script = match selected && index < rows.len() {
                true => SCRIPT_HIGHLIGHT,
                false => SCRIPT_UNHIGHLIGHT,
            };
            cx.rt.run_script(row, script);
        }
    }

    fn pick(&self, cx: &mut Cx) {
        let Some(alias) = self.aliases.get(self.selected) else { return };
        match self.mode {
            ListMode::Load => cx.send(Command::CareerLoad(alias.clone())),
            ListMode::Delete => {
                let question = format!("Delete the career named {alias}? It cannot be undone.");
                cx.send(Command::Pop);
                cx.send(Command::Confirm(Confirm::new(Text::Custom(question), Command::CareerDelete(alias.clone()))));
            }
        }
    }
}

impl ScreenLogic for AliasListLogic {
    fn start(&mut self, cx: &mut Cx) {
        self.aliases = list_aliases();
        if self.aliases.is_empty() {
            cx.send(Command::Alert(Alert::new(Text::Custom("There are no careers saved yet.".to_owned()))));
        }
        cx.rt.post_to_package(cx.package, ids::INPUT_ENABLE);
        self.show_title(cx);
        self.show(cx);
    }

    fn message(&mut self, cx: &mut Cx, message: u32) {
        match message {
            PAD_UP => {
                self.selected = moved(self.selected, self.aliases.len(), false);
                self.show(cx);
            }
            PAD_DOWN => {
                self.selected = moved(self.selected, self.aliases.len(), true);
                self.show(cx);
            }
            PAD_ACCEPT => self.pick(cx),
            PAD_BACK => cx.send(Command::Pop),
            _ => {}
        }
    }
}

/// The logic for `name` (lower case) when it is one of this module's screens. `MC_DeleteProfile.fng` is not one: the
/// delete question is the dialog, so nothing opens that package.
pub fn make(name: &str, args: Args) -> Option<Box<dyn ScreenLogic>> {
    if name == screen::MC_MAIN.to_ascii_lowercase() {
        return Some(Box::new(SavePromptLogic::new(args)));
    }
    if name == screen::MC_LIST.to_ascii_lowercase() {
        return Some(Box::new(AliasListLogic::new(args, list_mode())));
    }
    None
}

#[cfg(test)]
#[path = "save_tests.rs"]
mod tests;

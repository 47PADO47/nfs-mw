//! The alias manager (`MC_ProfileManager.fng`): Create, Load, Save and Delete, each with its own picture
//! (docs/specs/career-parity-menus.md, section 3). The aliases are the career profiles of the profile folder. Create makes a
//! new alias and loads it, Load and Delete open the profile list (`career_screens/save.rs`), Save asks about the loaded alias.

use crate::career::CareerHandle;
use crate::frontend::career_screens::save::{self, ListMode, open_list};
use crate::frontend::confirm::{Alert, Confirm};
use crate::frontend::icon_items::Id;
use crate::frontend::logic::{Command, Cx};
use crate::frontend::name_entry::NameFor;
use crate::ui::Text;

/// "Are you sure you want to save the alias named %s?" (`0x39B3CCBA`).
const SAVE_QUESTION: u32 = 0x39B3_CCBA;

/// What pressing an icon of the alias manager does.
pub fn press(cx: &mut Cx, id: Id) {
    match id {
        Id::AliasCreate => cx.send(Command::EnterName(NameFor::Create)),
        Id::AliasLoad => open_list(cx, ListMode::Load),
        Id::AliasSave => ask_to_save(cx),
        Id::AliasDelete => open_list(cx, ListMode::Delete),
        _ => {}
    }
}

/// Asks to save the loaded alias, or says there is none.
fn ask_to_save(cx: &mut Cx) {
    let alias = CareerHandle::shared().with_loaded(|career| career.alias.clone());
    let Some(alias) = alias else {
        cx.send(Command::Alert(Alert::new(Text::key("dialogs-no-alias-to-save-first"))));
        return;
    };
    let template = Text::Game(SAVE_QUESTION).resolve();
    let question = save::question(template.as_deref(), Some(&alias));
    cx.send(Command::Confirm(Confirm::new(Text::Literal(question), Command::CareerSave)));
}

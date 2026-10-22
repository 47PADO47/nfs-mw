//! The career menu (`MainMenu_Sub.fng` with `Sub::Career`, the original's "career main menu"): Resume Career, Co-op Career,
//! Start New Career and Load for the loaded profile (docs/specs/career-parity-menus.md, section 6). Resume Career plays the
//! profile (`Command::ResumeCareer`), Co-op Career opens Press Start for player 2 (`custom_race/press_start.rs`), Start New
//! Career makes a new alias and plays it, Load picks another alias.

use nfsmw_career::settings::{Flags, PROLOGUE_STAGE};

use super::save::{ListMode, open_list};
use crate::career::CareerHandle;
use crate::frontend::icon_items::{Id, career_icons};
use crate::frontend::icon_menu::{IconMenu, Kind};
use crate::frontend::ids::screen;
use crate::frontend::logic::{Args, Command, Cx, ScreenLogic};
use crate::frontend::name_entry::NameFor;

/// The logic of the career menu: its row follows the loaded profile (`career_icons`).
pub fn make(args: Args) -> Box<dyn ScreenLogic> {
    let flags = CareerHandle::shared().with_loaded(|career| career.settings.flags).unwrap_or(Flags::EMPTY);
    let started = flags.contains(Flags::CAREER_STARTED);
    let won = flags.contains(Flags::GAME_OVER);
    let prologue = CareerHandle::shared().with_loaded(|career| career.stage()) == Some(PROLOGUE_STAGE);
    Box::new(IconMenu::with_icons(Kind::Career, args, career_icons(started, won, prologue)))
}

/// What pressing an icon of the career menu does.
pub fn press(cx: &mut Cx, id: Id) {
    match id {
        Id::CareerResume => cx.send(Command::ResumeCareer),
        // Player 2's Press Start; co-op is set up there and in the car select (`flow_coop.rs`).
        Id::CareerCoop => {
            cx.send(Command::Switch(screen::PRESS_START.into(), Args { player: 1, coop: true, ..Args::default() }))
        }
        Id::CareerNew => cx.send(Command::EnterName(NameFor::NewCareer)),
        Id::CareerLoad => open_list(cx, ListMode::Load),
        _ => {}
    }
}

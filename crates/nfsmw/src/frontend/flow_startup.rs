//! The profile step of the boot flow: after the title screen the game asks which saved profile to play, or, with none
//! saved, asks to create one, before the main menu (the original's `MC_Main.fng` in start-up state `0x20`;
//! `docs/specs/boot-profile-prompt.md`).

use std::collections::VecDeque;

use super::{BOOT, Boot, Frontend, Stage};
use crate::app::Host;
use crate::frontend::career_screens::save::{ListMode, list_aliases, set_list_mode};
use crate::frontend::confirm::{Alert, Confirm, ConfirmLogic, Look};
use crate::frontend::ids::screen::{DIALOG, MC_LIST};
use crate::frontend::logic::{Args, Command, Env};
use crate::frontend::name_entry::NameFor;
use crate::frontend::scene::MenuScene;
use crate::ui::Text;

/// "Create a new alias? If you do not create a new alias, changes may be lost." (`MC_CREATE_NEW_PROFILE_MENU`,
/// `0xBE97590F`), the question the original asks at start-up when no alias is saved.
const CREATE_QUESTION: u32 = 0xBE97_590F;
/// The two notices that follow Yes (`MC_AUTOSAVE_WARNING_PC` `0xB01BE5DD` and `MC_AUTOSAVE_WARNING2_PC` `0xB380362F`),
/// each with Continue, before the name is asked.
const AUTOSAVE_NOTICE_1: u32 = 0xB01B_E5DD;
const AUTOSAVE_NOTICE_2: u32 = 0xB380_362F;

/// What the profile step does.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Step {
    /// A profile is loaded already: the main menu.
    Menu,
    /// Exactly one profile is saved: it is loaded at once, as the original does.
    Load,
    /// The profile list, to load one from.
    List,
    /// No profile is saved: ask to create one.
    Create,
}

/// The step for a game that has `saved` profiles to list and whether one is `loaded`. The original loads at once when
/// exactly one is saved (`docs/provenance/boot-profile-prompt.md`).
pub(super) fn decide(loaded: bool, saved: usize) -> Step {
    if loaded {
        return Step::Menu;
    }
    if saved == 0 {
        return Step::Create;
    }
    match saved {
        1 => Step::Load,
        _ => Step::List,
    }
}

/// The boot order: the EA logo, the public service announcement, the title screen. Without `movies` only the title screen
/// stays (the `skip_intro` setting, which the original's `SkipMovies` extra option also means).
pub(super) fn boot_steps(movies: bool) -> VecDeque<Boot> {
    BOOT.into_iter().filter(|step| movies || !matches!(step, Boot::Movie { .. })).collect()
}

/// True when the profile step is over because its last screen closed (Back, No, or the notice after a load or a create).
pub(super) fn is_over(startup: bool, stage: &Stage, top: Option<&str>) -> bool {
    startup && *stage == Stage::Menus && top.is_none()
}

/// The question asked when no alias is saved. Yes shows the two autosave notices, then asks for the name, which makes,
/// saves and loads the alias (`Command::AliasCreate`); No leaves for the main menu.
pub(super) fn create_question() -> Confirm {
    let notices = Command::Alert(Alert::then(
        Text::Game(AUTOSAVE_NOTICE_1),
        Command::Alert(Alert::then(Text::Game(AUTOSAVE_NOTICE_2), Command::EnterName(NameFor::Create))),
    ));
    Confirm::new(Text::Game(CREATE_QUESTION), notices).looking(Look::Selection)
}

impl Frontend {
    /// After the boot flow: the profile step (`decide`), then the main menu.
    pub(super) fn after_boot(&mut self, host: &mut Host, env: &mut Env) {
        let aliases = list_aliases();
        match decide(self.career.is_loaded(), aliases.len()) {
            Step::Menu => self.show_menu(host, env),
            Step::Load => self.load_only_profile(&aliases[0], host, env),
            Step::List => self.show_profile_list(host, env),
            Step::Create => self.ask_to_create(host, env),
        }
    }

    /// Starts the profile step with nothing else up, over a black screen: the original runs it before the front end's stage
    /// scene exists. The main menu that follows builds the real backdrop (`show_backdrop`, as `menu_in_window` is false).
    fn begin_startup(&mut self, host: &mut Host) {
        self.latch = true;
        self.screens.clear();
        self.replace_scene(host, Box::new(MenuScene::new(None)), false);
        self.startup = true;
        self.stage = Stage::Menus;
    }

    /// The only saved profile is loaded without a list; the notice that follows leads to the main menu.
    fn load_only_profile(&mut self, alias: &str, host: &mut Host, env: &mut Env) {
        log::info!("boot: one profile saved, loading {alias}");
        self.begin_startup(host);
        self.career_load(alias, host, env);
    }

    /// The list of saved profiles ("load") over the menu backdrop. Back leaves for the main menu with none loaded.
    fn show_profile_list(&mut self, host: &mut Host, env: &mut Env) {
        log::info!("boot: the profile list");
        set_list_mode(ListMode::Load);
        self.begin_startup(host);
        let commands = self.screens.open(MC_LIST, Args::default(), true, env);
        self.apply(commands, host, env);
    }

    /// No profile is saved: asks to create one.
    fn ask_to_create(&mut self, host: &mut Host, env: &mut Env) {
        log::info!("boot: no profile saved, asking to create one");
        self.begin_startup(host);
        let logic = Box::new(ConfirmLogic::new(create_question()));
        let commands = self.screens.open_with(DIALOG, logic, true, env);
        self.apply(commands, host, env);
    }

    /// Goes on to the main menu when the profile step's last screen has closed. Called every frame of the menus.
    pub(in crate::frontend) fn poll_startup(&mut self, host: &mut Host, env: &mut Env) {
        if !is_over(self.startup, &self.stage, self.screens.top()) {
            return;
        }
        log::info!("boot: the profile step is over");
        self.show_menu(host, env);
    }
}

#[cfg(test)]
#[path = "flow_startup_tests.rs"]
mod tests;

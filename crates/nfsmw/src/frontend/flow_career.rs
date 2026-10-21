//! Career on the main menu: the flow starts or resumes the career, then goes on to free roam with it
//! (`career-flow.md` sections 2 to 4). The start itself is `career::start_or_resume`.

use crate::app::Host;
use crate::career::{AutosaveInputs, CareerRequest, handle_autosave};
use crate::frontend::career_screens::car_lot::{self, CarLotAction};
use crate::frontend::confirm::{Alert, AlertLogic};
use crate::frontend::flow::Stage;
use crate::frontend::ids::screen::{CAR_SELECT, DIALOG, MAIN_MENU_SUB, SAFEHOUSE_REGION_UNLOCK};
use crate::frontend::logic::{Args, Env, QuitTo, Sub};
use nfsmw_career::safehouse::FrontEndContext;
use nfsmw_career::savefile::Trigger;
use nfsmw_career::settings::PROLOGUE_STAGE;

use super::Frontend;
use super::flow_loading::LoadingScreen;

impl Frontend {
    /// Career menu, Resume Career: the loaded profile is played. A career still in its prologue starts the world at once (the
    /// opening race restarts from the beginning); any other goes to the safe house menu, which stands before the world until
    /// Resume Free Roam (`career-flow.md` section 4).
    pub(super) fn resume_career(&mut self, host: &mut Host, env: &mut Env) {
        self.career.begin_play();
        self.origin = QuitTo::Career;
        let prologue = self.career.with(|career| career.stage() == PROLOGUE_STAGE).unwrap_or(false);
        if prologue {
            log::info!("career: resuming in the prologue");
            self.save_settings(env.changed);
            return self.start_free_roam(LoadingScreen::Tips, host, env);
        }
        log::info!("career: resuming at the safe house");
        self.career.set_prelude(true);
        self.in_safe_house = true;
        self.latch = true;
        self.screens.clear();
        self.open_menu(MAIN_MENU_SUB, Args { safehouse: true, ..Args::default() }, host, env);
    }

    /// Resume Free Roam in the safe house menu that stands before the world: the roaming-start autosave, then the world.
    pub(super) fn leave_prelude(&mut self, host: &mut Host, env: &mut Env) {
        if !self.career.is_prelude() {
            return;
        }
        self.show_menu_at(MAIN_MENU_SUB, Args { sub: Sub::Career, ..Args::default() }, host, env);
    }

    /// The safe house menu's Resume Free Roam was confirmed before the world exists: the world starts behind the loading
    /// screen. The roaming-start autosave is asked for first, as leaving the safe house does in the world.
    pub(super) fn start_from_safe_house(&mut self, host: &mut Host, env: &mut Env) {
        self.save_settings(env.changed);
        self.career.set_prelude(false);
        self.leave_safe_house();
        self.start_free_roam(LoadingScreen::Tips, host, env);
    }

    /// Quick Play, Accept: the world starts the race with the car the brief picked.
    pub(super) fn start_quick_play(&mut self, race: String, car: String, host: &mut Host, env: &mut Env) {
        self.career.end_play();
        self.origin = QuitTo::QuickRace;
        self.quick_race = Some(race);
        self.quick_car = Some(car);
        self.quick_play = true;
        self.start_free_roam(LoadingScreen::Plain, host, env);
    }

    /// Leaves the game in progress for the menu it was started from (the pause menu's Quit).
    pub(super) fn quit_to(&mut self, to: QuitTo, host: &mut Host, env: &mut Env) {
        match to {
            QuitTo::Main => self.show_menu(host, env),
            QuitTo::QuickRace => {
                self.show_menu_at(MAIN_MENU_SUB, Args { sub: Sub::QuickRace, ..Args::default() }, host, env)
            }
            QuitTo::Career => self.show_menu_at(MAIN_MENU_SUB, Args { sub: Sub::Career, ..Args::default() }, host, env),
        }
    }

    /// The pause menu's Restart: the race in progress starts again from its grid, and the game goes on.
    pub(super) fn restart_race(&mut self, host: &mut Host) {
        if !host.scene.restart_race() {
            return;
        }
        self.resume();
    }

    /// Carries out what the career in the world asks for (`career::CareerRequest`), while driving: the safe house
    /// menu, the car lot and the region unlock screen open over the frozen game, as the bust ticket does.
    pub(in crate::frontend) fn poll_career(&mut self, host: &mut Host, env: &mut Env) {
        if self.stage != Stage::Playing {
            return;
        }
        let Some(request) = host.scene.take_career_request() else { return };
        log::info!("career: {request:?}");
        let (screen, args) = match request {
            CareerRequest::SafeHouse { .. } => (MAIN_MENU_SUB, Args { safehouse: true, ..Args::default() }),
            CareerRequest::CarLot { .. } => (CAR_SELECT, Args { safehouse: true, ..Args::default() }),
            CareerRequest::RegionUnlock { .. } => {
                (SAFEHOUSE_REGION_UNLOCK, Args { safehouse: true, ..Args::default() })
            }
            // The garage is not a front end screen: the update system opens it over the world, which keeps running.
            CareerRequest::AutoShop { .. } => {
                self.auto_shop_requested = true;
                return;
            }
        };
        self.in_safe_house = screen == MAIN_MENU_SUB;
        self.latch = true;
        self.paused.set(true);
        self.stage = Stage::Paused;
        let commands = self.screens.open(screen, args, true, env);
        self.apply(commands, host, env);
    }

    /// Leaving the safe house restarts free roam (`SetRoaming`), which asks for the roaming-start autosave
    /// (`career-save-screens-2.md` section 8). Other resumes (the pause menu, the bust ticket) do not.
    pub(super) fn leave_safe_house(&mut self) {
        if !std::mem::take(&mut self.in_safe_house) {
            return;
        }
        let inputs = AutosaveInputs {
            context: FrontEndContext::CAREER,
            node_id: None,
            boss_race: false,
            gameplay_idle: true,
            profile_loaded: true,
        };
        handle_autosave(&self.career, self.profiles.as_ref(), Trigger::RoamingStart, inputs);
    }

    /// Carries out a confirmed car lot purchase or sale on the career; a refusal is shown in an alert.
    pub(super) fn car_lot_action(&mut self, action: CarLotAction, host: &mut Host, env: &mut Env) {
        let Some(Err(text)) = self.career.with(|career| car_lot::apply(career, &action)) else { return };
        let commands = self.screens.open_with(DIALOG, Box::new(AlertLogic::new(Alert::new(text))), false, env);
        self.apply(commands, host, env);
    }
}

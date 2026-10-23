//! Career on the main menu: the flow starts or resumes the career, then goes on to free roam with it
//! (`career-flow.md` sections 2 to 4). The start itself is `career::start_or_resume`.

use nfsmw_career::blacklist::FINAL_RACE_EVENT;
use nfsmw_career::story::final_resume::resume_final_race;

use crate::app::Host;
use crate::career::{AutosaveInputs, CareerRequest, handle_autosave};
use crate::frontend::career_screens::car_lot::{self, CarLotAction};
use crate::frontend::confirm::{Alert, AlertLogic};
use crate::frontend::flow::Stage;
use crate::frontend::ids::screen::{DIALOG, MAIN_MENU_SUB, SAFEHOUSE_REGION_UNLOCK};
use crate::frontend::logic::{Args, Env, QuitTo, Sub};
use nfsmw_career::safehouse::FrontEndContext;
use nfsmw_career::savefile::Trigger;

use super::Frontend;
use super::flow_arrival;
use super::flow_loading::LoadingScreen;

impl Frontend {
    /// Career menu, Resume Career: the loaded profile is played. A career in its prologue starts the first prologue race it has
    /// not won, as a career race (`story-flow.md` section 2.7); a finished Blacklist (race 1.2.3 won, career not completed)
    /// starts the final race 1.8.1; any other goes to the safe house menu, which stands before the world until Resume Free
    /// Roam (`career-flow.md` section 4).
    pub(super) fn resume_career(&mut self, host: &mut Host, env: &mut Env) {
        self.career.begin_play();
        self.origin = QuitTo::Career;
        if let Some(event) = self.resume_prologue_event() {
            log::info!("career: resuming the prologue at {event}");
            self.save_settings(env.changed);
            return self.start_career_race(event, host, env);
        }
        if self.resume_first_car(host, env) {
            return;
        }
        if self.resume_final_race_due() {
            log::info!("career: resuming at the final race {FINAL_RACE_EVENT}");
            self.save_settings(env.changed);
            return self.start_career_race(FINAL_RACE_EVENT, host, env);
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
        self.safe_house_start.pending = true;
        self.quick_car = Some(self.profile_car());
        self.start_free_roam(LoadingScreen::Tips, host, env);
    }

    /// Quick Play, Accept: the world starts the race with the car the brief picked.
    pub(super) fn start_quick_play(&mut self, race: String, car: String, host: &mut Host, env: &mut Env) {
        self.career.end_play();
        self.origin = QuitTo::QuickRace;
        self.quick_race = Some(race);
        self.quick_car = Some(car);
        self.race_options = Some(Default::default());
        self.start_free_roam(LoadingScreen::Plain, host, env);
    }

    /// Whether the loaded career resumes at the final race: race 1.2.3 is won and the career is not completed
    /// (`story-flow.md` section 2.7).
    fn resume_final_race_due(&self) -> bool {
        self.career.with(|career| resume_final_race(&career.settings.nodes(), career.settings.flags)).unwrap_or(false)
    }

    /// Leaves the game in progress for the menu it was started from (the pause menu's Quit).
    pub(in crate::frontend) fn quit_to(&mut self, to: QuitTo, host: &mut Host, env: &mut Env) {
        match to {
            QuitTo::Main if self.series.active => self.show_series(host, env),
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
        // The race goes back to its own grid: no loading screen covers it (`flow_cover.rs`).
        self.restarting = true;
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
        if let CareerRequest::SafeHouse { zone } = &request {
            self.safe_house_start.zone = Some(zone.clone());
        }
        // The first safe house visit (stage 16) plays its intro movie, then the rival bio (`flow_first_visit.rs`).
        if matches!(request, CareerRequest::SafeHouse { .. }) && self.first_visit_intro(host) {
            return;
        }
        let (screen, args) = match request {
            // A rival beaten since the last safe house visit: its arrival sequence runs in place of the menu.
            CareerRequest::SafeHouse { .. } => {
                if flow_arrival::take_due(&self.career) {
                    return self.start_arrival(host, env);
                }
                // The safe house is a room of its own: the world gives way to it, and Resume Free Roam starts the world
                // again at the zone (`start_from_safe_house`).
                self.career.set_prelude(true);
                return self.open_safe_house(host, env);
            }
            // The car lot is a room of its own: the world gives way to it (`flow_prologue.rs`).
            CareerRequest::CarLot { zone } => return self.open_car_lot(Some(zone), host, env),
            CareerRequest::FirstCarLot => return self.open_car_lot(None, host, env),
            // The arrival sequence shows this screen at its region step, in the safe house.
            CareerRequest::RegionUnlock { .. } if flow_arrival::is_due(&self.career) => {
                log::info!("career: the region screen waits for the arrival sequence");
                return;
            }
            CareerRequest::RegionUnlock { .. } => {
                (SAFEHOUSE_REGION_UNLOCK, Args { safehouse: true, ..Args::default() })
            }
            // The garage is not a front end screen: the update system opens it over the world, which keeps running.
            CareerRequest::AutoShop { .. } => {
                self.auto_shop_requested = true;
                return;
            }
            // The message screen is opened with its own logic, which closes back to the game (`flow_sms.rs`).
            CareerRequest::Sms { id } => return self.open_sms(id, host, env),
            // The final race is won: the end-game chain runs over the game (`flow_endgame.rs`).
            CareerRequest::EndGame => return self.start_end_game(host, env),
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

    /// Carries out a confirmed car lot purchase or sale on the career; a refusal is shown in an alert. The first car of the
    /// prologue starts the drive to the first safe house once it is bought (`start_first_drive`).
    pub(super) fn car_lot_action(&mut self, action: CarLotAction, host: &mut Host, env: &mut Env) {
        let first_drive = action.starts_first_drive();
        match self.career.with(|career| car_lot::apply(career, &action)) {
            Some(Ok(())) if first_drive => self.start_first_drive(host, env),
            Some(Err(text)) => {
                let commands = self.screens.open_with(DIALOG, Box::new(AlertLogic::new(Alert::new(text))), false, env);
                self.apply(commands, host, env);
            }
            _ => {}
        }
    }
}

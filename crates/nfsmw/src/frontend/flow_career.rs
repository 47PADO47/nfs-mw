//! Career on the main menu: the flow starts or resumes the career, then goes on to free roam with it
//! (`career-flow.md` sections 2 to 4). The start itself is `career::start_or_resume`.

use crate::app::Host;
use crate::career::{AutosaveInputs, CareerRequest, handle_autosave, start_or_resume};
use crate::frontend::career_screens::car_lot::{self, CarLotAction};
use crate::frontend::confirm::{Alert, AlertLogic};
use crate::frontend::flow::Stage;
use crate::frontend::ids::screen::{CAR_SELECT, DIALOG, MAIN_MENU_SUB, SAFEHOUSE_REGION_UNLOCK};
use crate::frontend::logic::Args;
use crate::frontend::logic::Env;
use crate::ui::Text;
use nfsmw_career::safehouse::FrontEndContext;
use nfsmw_career::savefile::Trigger;

use super::Frontend;

impl Frontend {
    /// Career on the main menu. On success the career is the one being played and free roam starts. On failure the
    /// reason is logged and shown in an alert, and the main menu stays up.
    pub(super) fn start_career(&mut self, host: &mut Host, env: &mut Env) {
        let started = match self.profiles.as_ref() {
            Some(profiles) => start_or_resume(&self.dir, profiles),
            None => Err(anyhow::anyhow!("there is no per-user folder for the profiles")),
        };
        let career = match started {
            Ok(career) => career,
            Err(e) => return self.career_failed(e, host, env),
        };
        log::info!("career: playing {}", career.alias);
        self.career.set(career);
        self.start_free_roam(host, env);
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

    /// Shows why the career could not start, over the main menu.
    fn career_failed(&mut self, e: anyhow::Error, host: &mut Host, env: &mut Env) {
        log::warn!("the career could not be started: {e:#}");
        let alert = AlertLogic::new(Alert::new(Text::Custom(format!("The career could not be started. {e}"))));
        let commands = self.screens.open_with(DIALOG, Box::new(alert), false, env);
        self.apply(commands, host, env);
    }
}

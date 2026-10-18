//! Career on the main menu: the flow starts or resumes the career, then goes on to free roam with it
//! (`career-flow.md` sections 2 to 4). The start itself is `career::start_or_resume`.

use crate::app::Host;
use crate::career::start_or_resume;
use crate::frontend::confirm::{Alert, AlertLogic};
use crate::frontend::ids::screen::DIALOG;
use crate::frontend::logic::Env;
use crate::ui::Text;

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

    /// Shows why the career could not start, over the main menu.
    fn career_failed(&mut self, e: anyhow::Error, host: &mut Host, env: &mut Env) {
        log::warn!("the career could not be started: {e:#}");
        let alert = AlertLogic::new(Alert::new(Text::Custom(format!("The career could not be started. {e}"))));
        let commands = self.screens.open_with(DIALOG, Box::new(alert), false, env);
        self.apply(commands, host, env);
    }
}

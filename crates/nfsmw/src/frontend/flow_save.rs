//! The career's save commands: save, load, new and delete profiles (`MC_Main.fng`; `docs/specs/career-save-screens.md`).
//! Each one answers with a notice over the screen that asked, except a load or a new career, which starts free roam
//! with the career it made (`start_free_roam`, as `flow_career.rs` does for Career on the main menu).

use std::sync::Arc;

use anyhow::Result;
use game_install::GameDir;
use nfsmw_career::settings::NewCareer;

use crate::app::Host;
use crate::career::{Career, CareerData, ProfileDir};
use crate::frontend::confirm::{Alert, AlertLogic};
use crate::frontend::ids::screen::DIALOG;
use crate::frontend::logic::Env;
use crate::ui::Text;

use super::Frontend;

/// The notice when there is no per-user folder to keep profiles in.
const NO_FOLDER: &str = "There is no per-user folder for the careers.";

impl Frontend {
    /// Saves the career being played to its profile.
    pub(super) fn career_save(&mut self, host: &mut Host, env: &mut Env) {
        let Some(profiles) = self.profiles.clone() else {
            return self.career_notice(NO_FOLDER.to_owned(), host, env);
        };
        let Some((alias, bytes)) = self.career.with(|career| (career.alias.clone(), career.profile_bytes())) else {
            return self.career_notice("There is no career to save.".to_owned(), host, env);
        };
        let result = profiles.write(&alias, &bytes);
        let message = saved_message(&alias, &result);
        self.career_notice(message, host, env);
    }

    /// Loads the profile of `alias` and plays it. A profile that fails the load checks is not played: its notice stays.
    pub(super) fn career_load(&mut self, alias: &str, host: &mut Host, env: &mut Env) {
        let Some(profiles) = self.profiles.clone() else {
            return self.career_notice(NO_FOLDER.to_owned(), host, env);
        };
        let loaded = profiles.read(alias).and_then(|bytes| load_career(alias, &bytes, &self.dir));
        let career = match loaded {
            Ok(career) => career,
            Err(e) => {
                log::warn!("the career {alias} could not be loaded: {e:#}");
                return self.career_notice(loaded_failed(alias, &e), host, env);
            }
        };
        log::info!("career: loaded {alias}");
        self.career.set(career);
        self.start_free_roam(host, env);
    }

    /// Makes a new career under `alias`, saves it and plays it.
    pub(super) fn career_new(&mut self, alias: &str, host: &mut Host, env: &mut Env) {
        let Some(profiles) = self.profiles.clone() else {
            return self.career_notice(NO_FOLDER.to_owned(), host, env);
        };
        let career = match new_saved(alias, &self.dir, &profiles) {
            Ok(career) => career,
            Err(e) => {
                log::warn!("the career {alias} could not be made: {e:#}");
                return self.career_notice(format!("The career could not be started. {e:#}"), host, env);
            }
        };
        log::info!("career: new career {alias}");
        self.career.set(career);
        self.start_free_roam(host, env);
    }

    /// Deletes the profile of `alias`. The career being played is left when it is the one deleted.
    pub(super) fn career_delete(&mut self, alias: &str, host: &mut Host, env: &mut Env) {
        let Some(profiles) = self.profiles.clone() else {
            return self.career_notice(NO_FOLDER.to_owned(), host, env);
        };
        let result = profiles.remove(alias);
        if result.is_ok() && self.career.with(|career| career.alias == alias).unwrap_or(false) {
            self.career.clear();
        }
        let message = deleted_message(alias, &result);
        self.career_notice(message, host, env);
    }

    /// Shows `message` as a notice over the screen that is up.
    fn career_notice(&mut self, message: String, host: &mut Host, env: &mut Env) {
        let alert = AlertLogic::new(Alert::new(Text::Custom(message)));
        let commands = self.screens.open_with(DIALOG, Box::new(alert), false, env);
        self.apply(commands, host, env);
    }
}

/// Reads the career tables and makes the career of a profile's bytes. The bytes are checked by the caller's read.
fn load_career(alias: &str, bytes: &[u8], dir: &GameDir) -> Result<Career> {
    let data = Arc::new(CareerData::load(dir)?);
    Career::from_profile_bytes(alias, bytes, data)
}

/// Makes a new career under `alias` and writes its profile at once, as `start.rs` does for a new career.
fn new_saved(alias: &str, dir: &GameDir, profiles: &ProfileDir) -> Result<Career> {
    let data = Arc::new(CareerData::load(dir)?);
    let mut career = Career::new(alias, data, &NewCareer::default())?;
    profiles.write(alias, &career.profile_bytes())?;
    Ok(career)
}

/// The notice after a save.
fn saved_message(alias: &str, result: &Result<()>) -> String {
    match result {
        Ok(()) => format!("{alias} saved."),
        Err(e) => format!("The career could not be saved. {e:#}"),
    }
}

/// The notice after a delete.
fn deleted_message(alias: &str, result: &Result<()>) -> String {
    match result {
        Ok(()) => format!("{alias} deleted."),
        Err(e) => format!("The career could not be deleted. {e:#}"),
    }
}

/// The notice when a profile could not be loaded (its checks refused it, or the tables could not be read).
fn loaded_failed(alias: &str, e: &anyhow::Error) -> String {
    format!("The career {alias} could not be loaded. {e:#}")
}

#[cfg(test)]
#[path = "flow_save_tests.rs"]
mod tests;

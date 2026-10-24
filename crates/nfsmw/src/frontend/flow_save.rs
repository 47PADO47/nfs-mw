//! The career's save commands: save, load, new and delete profiles (`MC_Main.fng`; `docs/specs/career-save-screens.md`).
//! Each one answers with a notice over the screen that asked, except a new career, which starts its prologue
//! (`flow_prologue.rs`).
//!
//! A load, a new career and a created alias need the career tables (`CareerData::cached`), which take seconds the first
//! time in a session. They are read on a thread (`SaveJob`) while the screen keeps running, and the command goes on from
//! `poll_save_job` once the tables are in; a second command of the three is ignored until then.

use std::sync::Arc;
use std::thread::JoinHandle;

use anyhow::{Result, anyhow};
use nfsmw_career::settings::NewCareer;

use crate::app::Host;
use crate::career::{Career, CareerData, ProfileDir, Sources};
use crate::frontend::confirm::{Alert, AlertLogic};
use crate::frontend::ids::screen::DIALOG;
use crate::frontend::logic::{Command, Env, QuitTo};
use crate::ui::Text;

use super::Frontend;
use super::flow_prologue::FIRST_PROLOGUE_RACE;

/// The notice when there is no per-user folder to keep profiles in.
const NO_FOLDER: &str = "There is no per-user folder for the careers.";

/// What a command does once the career tables are read.
enum Work {
    /// A load: the profile `alias` of these bytes. `top` is the screen that asked; it closes if it is still on top.
    Load { alias: String, bytes: Vec<u8>, top: Option<String> },
    /// A new career under `alias`, saved in `profiles` and played.
    New { alias: String, profiles: ProfileDir },
    /// The alias manager's Create: a new career under `alias`, saved in `profiles` and loaded.
    Create { alias: String, profiles: ProfileDir },
}

/// A career command waiting for the career tables, which a thread reads.
pub(super) struct SaveJob {
    tables: JoinHandle<Result<Arc<CareerData>>>,
    work: Work,
}

impl Frontend {
    /// Saves the loaded profile to its file.
    pub(super) fn career_save(&mut self, host: &mut Host, env: &mut Env) {
        let Some(profiles) = self.profiles.clone() else {
            return self.career_notice(NO_FOLDER.to_owned(), host, env);
        };
        let Some((alias, bytes)) = self.career.with_loaded(|career| (career.alias.clone(), career.profile_bytes()))
        else {
            return self.career_notice("There is no alias to save.".to_owned(), host, env);
        };
        let result = profiles.write(&alias, &bytes);
        let message = saved_message(&alias, &result);
        self.career_notice(message, host, env);
    }

    /// Loads the profile of `alias` as the player's profile (it is not played yet). The list closes and "Load complete." shows
    /// (the library's notice, `career-save-screens-2.md` section 9.5); from the list the game opened at the start the main menu
    /// follows it. A profile that fails the load checks is not loaded: its notice shows over the list, which stays.
    ///
    /// The profile file is read now; the career tables are read on a thread, and the load goes on in `career_loaded`.
    pub(super) fn career_load(&mut self, alias: &str, host: &mut Host, env: &mut Env) {
        if self.save_job.is_some() {
            return;
        }
        // This build's folder first, then the original game's, which is only read (`career/sources.rs`).
        let sources = Sources::with_own(self.profiles.clone());
        let bytes = match sources.read(alias) {
            Ok(bytes) => bytes,
            Err(e) => return self.load_failed(alias, &e, host, env),
        };
        let top = self.screens.top().map(str::to_owned);
        self.start_save_job(Work::Load { alias: alias.to_owned(), bytes, top }, host, env);
    }

    /// The load of `alias` has its career, or the reason it has none.
    fn career_loaded(
        &mut self,
        alias: &str,
        loaded: Result<Career>,
        top: Option<String>,
        host: &mut Host,
        env: &mut Env,
    ) {
        let career = match loaded {
            Ok(career) => career,
            Err(e) => return self.load_failed(alias, &e, host, env),
        };
        log::info!("career: loaded {alias}");
        self.career.load(career);
        // The screen that asked may have been left while the tables were read: only that one closes.
        if self.screens.top() == top.as_deref() {
            let commands = self.screens.pop(env);
            self.apply(commands, host, env);
        }
        let at_start = self.screens.top().is_none();
        let message = Text::Literal(LOAD_COMPLETE.to_owned());
        let alert = match at_start {
            true => Alert::then(message, Command::QuitTo(QuitTo::Main)),
            false => Alert::new(message),
        };
        self.show_alert(alert, host, env);
    }

    fn load_failed(&mut self, alias: &str, e: &anyhow::Error, host: &mut Host, env: &mut Env) {
        log::warn!("the career {alias} could not be loaded: {e:#}");
        self.career_notice(loaded_failed(alias, e), host, env);
    }

    /// Makes a new career under `alias`, saves it and plays it: the world starts with the first prologue race (`16.1.0`) in
    /// the car it forces, and the chain of the prologue goes on from there (`flow_prologue.rs`).
    pub(super) fn career_new(&mut self, alias: &str, host: &mut Host, env: &mut Env) {
        if self.save_job.is_some() {
            return;
        }
        let Some(profiles) = self.profiles.clone() else {
            return self.career_notice(NO_FOLDER.to_owned(), host, env);
        };
        self.start_save_job(Work::New { alias: alias.to_owned(), profiles }, host, env);
    }

    /// The new career of `alias` is made and saved, or the reason it is not: it is played.
    fn career_made(&mut self, alias: &str, made: Result<Career>, host: &mut Host, env: &mut Env) {
        let career = match made {
            Ok(career) => career,
            Err(e) => {
                log::warn!("the career {alias} could not be made: {e:#}");
                return self.career_notice(format!("The career could not be started. {e:#}"), host, env);
            }
        };
        log::info!("career: new career {alias}");
        self.career.set(career);
        self.start_career_race(FIRST_PROLOGUE_RACE, host, env);
    }

    /// The alias manager's Create: a new alias is made, saved and loaded, and the notice says so. It is not played: Career
    /// then Resume Career plays it.
    pub(super) fn alias_create(&mut self, alias: &str, host: &mut Host, env: &mut Env) {
        if self.save_job.is_some() {
            return;
        }
        let Some(profiles) = self.profiles.clone() else {
            return self.career_notice(NO_FOLDER.to_owned(), host, env);
        };
        self.start_save_job(Work::Create { alias: alias.to_owned(), profiles }, host, env);
    }

    /// The alias of the alias manager is made and saved, or the reason it is not: it is loaded and the notice says so.
    fn alias_created(&mut self, alias: &str, made: Result<Career>, host: &mut Host, env: &mut Env) {
        let career = match made {
            Ok(career) => career,
            Err(e) => {
                log::warn!("the alias {alias} could not be made: {e:#}");
                return self.career_notice(format!("The alias could not be created. {e:#}"), host, env);
            }
        };
        log::info!("career: created the alias {alias}");
        self.career.load(career);
        self.career_notice(format!("{alias} created."), host, env);
    }

    /// Deletes the profile of `alias`. The loaded profile is unloaded when it is the one deleted.
    pub(super) fn career_delete(&mut self, alias: &str, host: &mut Host, env: &mut Env) {
        let Some(profiles) = self.profiles.clone() else {
            return self.career_notice(NO_FOLDER.to_owned(), host, env);
        };
        let result = profiles.remove(alias);
        if result.is_ok() && self.career.with_loaded(|career| career.alias == alias).unwrap_or(false) {
            self.career.clear();
        }
        let message = deleted_message(alias, &result);
        self.career_notice(message, host, env);
    }

    /// Starts reading the career tables on a thread; `work` goes on when they are in (`poll_save_job`).
    fn start_save_job(&mut self, work: Work, host: &mut Host, env: &mut Env) {
        let dir = self.dir.clone();
        let spawned =
            std::thread::Builder::new().name("career-tables".to_owned()).spawn(move || CareerData::cached(&dir));
        match spawned {
            Ok(tables) => self.save_job = Some(SaveJob { tables, work }),
            Err(e) => {
                let tables = Err(anyhow!("the career tables could not be read: {e}"));
                self.finish_save_job(work, tables, host, env);
            }
        }
    }

    /// Goes on with the command waiting for the career tables once they are read. Called every frame of the menus.
    /// `true` while it is still waiting.
    pub(super) fn poll_save_job(&mut self, host: &mut Host, env: &mut Env) -> bool {
        let Some(job) = self.save_job.take() else { return false };
        if !job.tables.is_finished() {
            self.save_job = Some(job);
            return true;
        }
        let tables = job.tables.join().unwrap_or_else(|_| Err(anyhow!("the career tables thread panicked")));
        self.finish_save_job(job.work, tables, host, env);
        false
    }

    fn finish_save_job(&mut self, work: Work, tables: Result<Arc<CareerData>>, host: &mut Host, env: &mut Env) {
        match work {
            Work::Load { alias, bytes, top } => {
                let loaded = tables.and_then(|data| Career::from_profile_bytes(&alias, &bytes, data));
                self.career_loaded(&alias, loaded, top, host, env);
            }
            Work::New { alias, profiles } => {
                let made = tables.and_then(|data| new_saved(&alias, data, &profiles));
                self.career_made(&alias, made, host, env);
            }
            Work::Create { alias, profiles } => {
                let made = tables.and_then(|data| new_saved(&alias, data, &profiles));
                self.alias_created(&alias, made, host, env);
            }
        }
    }

    /// Shows `message` as a notice over the screen that is up.
    fn career_notice(&mut self, message: String, host: &mut Host, env: &mut Env) {
        self.show_alert(Alert::new(Text::Literal(message)), host, env);
    }

    fn show_alert(&mut self, alert: Alert, host: &mut Host, env: &mut Env) {
        let commands = self.screens.open_with(DIALOG, Box::new(AlertLogic::new(alert)), false, env);
        self.apply(commands, host, env);
    }
}

/// The notice after a successful load.
const LOAD_COMPLETE: &str = "Load complete.";

/// Makes a new career under `alias` from the career tables and writes its profile at once, as `start.rs` does for a new
/// career.
fn new_saved(alias: &str, data: Arc<CareerData>, profiles: &ProfileDir) -> Result<Career> {
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

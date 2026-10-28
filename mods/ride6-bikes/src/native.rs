//! The native mod entry: `ride6-bikes` imports the bikes of the user's RIDE 6 exports in the background and adds them to the
//! game. Modelled on `mods/nfsmw-2012-content/src/lib.rs`: the settings come from `bb.config`, the import runs on a thread
//! ([`job`]), a finished first run is promoted and announced with `bb.content` (`rescan`), and a later start whose inputs did
//! not change converts nothing ([`stamp`]).

use std::path::PathBuf;

use blackbox_modding::sdk::{Ctx, Event, Level, Mod, ModResult, declare_mod};

use crate::batch::Report;
use crate::config::{self, MOD_ID};
use crate::job::{self, Finished, Job, Start};

/// The mod's state: the import in flight, if any.
pub struct Ride6Bikes {
    mod_dir: PathBuf,
    job: Option<Job>,
    /// The folder already held content when the job started: the game has indexed it, so a new run waits for the next start
    /// instead of replacing it under the game.
    live: bool,
}

impl Mod for Ride6Bikes {
    fn load(ctx: &mut Ctx) -> ModResult<Self> {
        let Ok(mod_dir) = ctx.mod_dir() else {
            ctx.log(Level::Warn, "ride6-bikes: the host does not say where the mod is");
            return Ok(Self { mod_dir: PathBuf::new(), job: None, live: false });
        };
        let mut state = Self { mod_dir: mod_dir.clone(), job: None, live: false };
        let supplied = match supplied_config(ctx) {
            Ok(supplied) => supplied,
            Err(error) => {
                ctx.log(Level::Warn, &format!("ride6-bikes: no bike is added, the settings are not read: {error}"));
                return Ok(state);
            }
        };
        match job::start(&mod_dir, supplied.as_ref()) {
            Ok(Start::NotConfigured { config, created }) => {
                let made = if created { " (written just now)" } else { "" };
                let place = format!("{}{made}, or under [mods.{MOD_ID}] in the game's config file,", config.display());
                ctx.log(
                    Level::Info,
                    &format!("ride6-bikes: no RIDE 6 export is set. Put `source = '<folder>'` in {place} and restart"),
                );
            }
            Ok(Start::UpToDate { cars }) => ctx.log(Level::Info, &format!("ride6-bikes: {cars} bikes are ready")),
            Ok(Start::Started { job, live }) => {
                ctx.log(Level::Info, "ride6-bikes: importing the RIDE 6 bikes in the background");
                state.job = Some(job);
                state.live = live;
            }
            Err(error) => ctx.log(Level::Warn, &format!("ride6-bikes: no bike is added: {error:#}")),
        }
        Ok(state)
    }

    fn event(&mut self, ctx: &mut Ctx, _event: &Event<'_>) {
        let Some(finished) = self.job.as_ref().and_then(Job::poll) else { return };
        self.job = None;
        match finished {
            Finished::Done(report) => self.finish(ctx, &report),
            Finished::Failed(reason) => {
                ctx.log(Level::Warn, &format!("ride6-bikes: the import failed, no bike is added: {reason}"));
            }
        }
    }
}

/// The settings the host merged from the mod's `config.toml` and the game's `[mods.ride6-bikes]` table (`bb.config`). `None`
/// for a host that does not offer `bb.config`: the mod then reads `config.toml` alone.
fn supplied_config(ctx: &Ctx) -> ModResult<Option<config::Supplied>> {
    let Some(host) = ctx.config() else { return Ok(None) };
    let text = host.text()?;
    Ok(Some(config::Supplied { text, has_file: host.has_file(), has_main_section: host.has_main_section() }))
}

impl Ride6Bikes {
    /// A run is over: say what it did, and make its bikes available.
    fn finish(&self, ctx: &Ctx, report: &Report) {
        let (done, left_out) = (report.imported.len(), report.skipped.len());
        ctx.log(Level::Info, &format!("ride6-bikes: imported {done} bikes, {left_out} left out"));
        for (bike, reason) in &report.skipped {
            ctx.log(Level::Warn, &format!("ride6-bikes: {bike} is left out: {reason}"));
        }
        if self.live {
            ctx.log(Level::Info, "ride6-bikes: the new bikes replace the old ones at the next start");
            return;
        }
        match job::promote(&self.mod_dir) {
            Ok(true) => {}
            Ok(false) => return ctx.log(Level::Warn, "ride6-bikes: the imported bikes are missing"),
            Err(error) => {
                return ctx.log(Level::Warn, &format!("ride6-bikes: the bikes are not in place: {error:#}"));
            }
        }
        match ctx.content().map(|content| content.rescan()) {
            Some(Ok(())) => ctx.log(Level::Info, &format!("ride6-bikes: {done} bikes are ready")),
            Some(Err(error)) => {
                ctx.log(Level::Warn, &format!("ride6-bikes: the host did not take the bikes ({error}); restart"));
            }
            None => ctx.log(Level::Info, "ride6-bikes: the bikes are ready at the next start"),
        }
    }
}

declare_mod!(Ride6Bikes);

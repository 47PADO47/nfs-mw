//! `nfsmw-2012-content`: adds the cars of NFS Most Wanted (2012) to the game.
//!
//! The 2012 game runs on the Chameleon engine, not on the 2005 Black Box engine, so its car models and textures are
//! converted: `chameleon-vehicle` reads a car out of its bundles, this mod fits it to the wheel layout of a stock 2005
//! car and writes it as that car's `GEOMETRY.BIN` and `TEXTURES.BIN`, with a `cars/<id>.toml`, in its own folder. The
//! game loads those like the cars of any content mod (`docs/modding/api-content.md`). Nothing from either game is in
//! the repository: the cars are made on the user's machine from the user's own installs, and without a 2012 install
//! named in `config.toml` ([`config::Config::source`]) the mod adds nothing.
//!
//! The conversion runs on a background thread ([`job`]) so the game does not wait for it. The first time, the cars are
//! written to a staging folder; when the job ends, the mod moves them into place and tells the host through
//! `bb.content`, and everything the game opens after that (the garage, the career) lists the cars. Later starts find
//! the content ready and convert nothing, unless the settings or the installs changed.

pub mod config;
mod convert;
mod donor;
mod emit;
mod engine;
mod fit;
pub mod import;
pub mod job;
mod liveries;
mod names;
pub mod performance;
mod source;
mod stamp;
pub mod stats;
mod textures;

use std::path::PathBuf;

use blackbox_modding::sdk::{Ctx, Event, Level, Mod, ModResult, declare_mod};
use job::{Finished, Job, Start};

/// The mod's state: the conversion in flight, if any.
pub struct NfsmwContent {
    mod_dir: PathBuf,
    job: Option<Job>,
    /// The folder already held content when the job started: the game has indexed it, so a new run waits for the next
    /// start instead of replacing it under the game.
    live: bool,
}

impl Mod for NfsmwContent {
    fn load(ctx: &mut Ctx) -> ModResult<Self> {
        let (Ok(mod_dir), Ok(game_dir)) = (ctx.mod_dir(), ctx.game_dir()) else {
            ctx.log(Level::Warn, "nfsmw-2012-content: the host does not say where the mod and the game are");
            return Ok(Self { mod_dir: PathBuf::new(), job: None, live: false });
        };
        let mut state = Self { mod_dir: mod_dir.clone(), job: None, live: false };
        match job::start(&mod_dir, &game_dir) {
            Ok(Start::NotConfigured { config, created }) => {
                let made = if created { " (written just now)" } else { "" };
                ctx.log(
                    Level::Info,
                    &format!(
                        "nfsmw-2012-content: no 2012 install is set. Put `source = '<folder>'` in {}{made} and restart",
                        config.display()
                    ),
                );
            }
            Ok(Start::UpToDate { cars }) => {
                ctx.log(Level::Info, &format!("nfsmw-2012-content: {cars} cars are ready"));
            }
            Ok(Start::Started { job, live }) => {
                ctx.log(Level::Info, "nfsmw-2012-content: converting the 2012 cars in the background");
                state.job = Some(job);
                state.live = live;
            }
            Err(error) => ctx.log(Level::Warn, &format!("nfsmw-2012-content: no car is added: {error:#}")),
        }
        Ok(state)
    }

    fn event(&mut self, ctx: &mut Ctx, _event: &Event<'_>) {
        let Some(finished) = self.job.as_ref().and_then(Job::poll) else { return };
        self.job = None;
        match finished {
            Finished::Done(report) => self.finish(ctx, &report),
            Finished::Failed(reason) => {
                ctx.log(Level::Warn, &format!("nfsmw-2012-content: the conversion failed, no car is added: {reason}"));
            }
        }
    }
}

impl NfsmwContent {
    /// A run is over: say what it did, and make its cars available.
    fn finish(&self, ctx: &Ctx, report: &import::Report) {
        let (done, left_out) = (report.imported.len(), report.skipped.len());
        ctx.log(Level::Info, &format!("nfsmw-2012-content: converted {done} cars, {left_out} left out"));
        for (car, reason) in &report.skipped {
            ctx.log(Level::Warn, &format!("nfsmw-2012-content: {car} is left out: {reason}"));
        }
        if self.live {
            ctx.log(Level::Info, "nfsmw-2012-content: the new cars replace the old ones at the next start");
            return;
        }
        match job::promote(&self.mod_dir) {
            Ok(true) => {}
            Ok(false) => return ctx.log(Level::Warn, "nfsmw-2012-content: the converted cars are missing"),
            Err(error) => {
                return ctx.log(Level::Warn, &format!("nfsmw-2012-content: the cars are not in place: {error:#}"));
            }
        }
        match ctx.content().map(|content| content.rescan()) {
            Some(Ok(())) => ctx.log(Level::Info, &format!("nfsmw-2012-content: {done} cars are ready")),
            Some(Err(error)) => {
                ctx.log(Level::Warn, &format!("nfsmw-2012-content: the host did not take the cars ({error}); restart"));
            }
            None => ctx.log(Level::Info, "nfsmw-2012-content: the cars are ready at the next start"),
        }
    }
}

declare_mod!(NfsmwContent);

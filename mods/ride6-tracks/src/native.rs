//! The native mod entry: `ride6-tracks` imports the track packs of the user's RIDE 6 exports in the background and offers them
//! as second worlds. Modelled on `mods/ride6-bikes/src/native.rs`: the settings come from `bb.config`, the import runs on a
//! thread ([`job`]), a finished first run is promoted and announced with `bb.content` (`rescan`), and a later start whose
//! inputs did not change converts nothing ([`stamp`]).

use std::path::PathBuf;

use blackbox_modding::sdk::{Ctx, Event, Level, MessageStyle, Mod, ModResult, declare_mod};

use crate::config::{self, MOD_ID};
use crate::job::{self, Finished, Job, Start, Summary};
use crate::lap::{self, Timer};

/// The mod's state: the import in flight, if any.
pub struct Ride6Tracks {
    mod_dir: PathBuf,
    job: Option<Job>,
    /// The folder already held content when the job started: the game has indexed it, so a new run waits for the next start
    /// instead of replacing it under the game.
    live: bool,
    /// The lap timer of the world being played, with the world's id; none in a world without a lap file (the city).
    lap: Option<(String, Timer)>,
    /// Scene ticks seen, to look at the world's name now and then.
    ticks: u64,
}

impl Mod for Ride6Tracks {
    fn load(ctx: &mut Ctx) -> ModResult<Self> {
        let Ok(mod_dir) = ctx.mod_dir() else {
            ctx.log(Level::Warn, "ride6-tracks: the host does not say where the mod is");
            return Ok(Self { mod_dir: PathBuf::new(), job: None, live: false, lap: None, ticks: 0 });
        };
        let mut state = Self { mod_dir: mod_dir.clone(), job: None, live: false, lap: None, ticks: 0 };
        let supplied = match supplied_config(ctx) {
            Ok(supplied) => supplied,
            Err(error) => {
                ctx.log(Level::Warn, &format!("ride6-tracks: no world is added, the settings are not read: {error}"));
                return Ok(state);
            }
        };
        match job::start(&mod_dir, supplied.as_ref()) {
            Ok(Start::NotConfigured { config, created }) => {
                let made = if created { " (written just now)" } else { "" };
                let place = format!("{}{made}, or under [mods.{MOD_ID}] in the game's config file,", config.display());
                ctx.log(
                    Level::Info,
                    &format!(
                        "ride6-tracks: no track pack folder is set. Put `source = '<folder>'` in {place} and restart"
                    ),
                );
            }
            Ok(Start::UpToDate { worlds }) => ctx.log(Level::Info, &format!("ride6-tracks: {worlds} worlds are ready")),
            Ok(Start::Started { job, live }) => {
                ctx.log(Level::Info, "ride6-tracks: converting the RIDE 6 tracks in the background");
                state.job = Some(job);
                state.live = live;
            }
            Err(error) => ctx.log(Level::Warn, &format!("ride6-tracks: no world is added: {error:#}")),
        }
        Ok(state)
    }

    fn event(&mut self, ctx: &mut Ctx, event: &Event<'_>) {
        if let Event::SceneTick { dt_sim, paused: false, .. } = event {
            self.time_the_lap(ctx, *dt_sim);
        }
        let Some(finished) = self.job.as_ref().and_then(Job::poll) else { return };
        self.job = None;
        match finished {
            Finished::Done(summary) => self.finish(ctx, &summary),
            Finished::Failed(reason) => {
                ctx.log(Level::Warn, &format!("ride6-tracks: the import failed, no world is added: {reason}"));
            }
        }
    }
}

/// The settings the host merged from the mod's `config.toml` and the game's `[mods.ride6-tracks]` table (`bb.config`). `None`
/// for a host that does not offer `bb.config`: the mod then reads `config.toml` alone.
fn supplied_config(ctx: &Ctx) -> ModResult<Option<config::Supplied>> {
    let Some(host) = ctx.config() else { return Ok(None) };
    let text = host.text()?;
    Ok(Some(config::Supplied { text, has_file: host.has_file(), has_main_section: host.has_main_section() }))
}

impl Ride6Tracks {
    /// One scene tick of lap timing: follow the world being played, and time the player's car on its lap.
    fn time_the_lap(&mut self, ctx: &Ctx, dt: f32) {
        let (Some(world), Some(vehicles)) = (ctx.world(), ctx.vehicles()) else { return };
        if self.ticks % 120 == 0 {
            self.follow_world(ctx, &world);
        }
        self.ticks += 1;
        let Some((_, timer)) = self.lap.as_mut() else { return };
        let Ok(state) = vehicles.player() else { return };
        let Some(event) = timer.update(state.position, dt) else { return };
        let Some(hud) = ctx.hud() else { return };
        let (text, style) = match event {
            lap::Event::Started => ("Lap timing started".to_owned(), MessageStyle::Info),
            lap::Event::Lap { number, seconds, best } => {
                let suffix = if best { "  best lap" } else { "" };
                (format!("Lap {number}  {}{suffix}", lap::clock(seconds)), MessageStyle::Big)
            }
        };
        let _ = hud.message(&text, style, 6.0);
    }

    /// Loads the lap of the world being played when it is not the one timed, `worlds/<id>.lap` of the mod.
    fn follow_world(&mut self, ctx: &Ctx, world: &blackbox_modding::sdk::World) {
        let Ok(track) = world.track() else { return };
        if self.lap.as_ref().is_some_and(|(id, _)| *id == track) {
            return;
        }
        self.lap = None;
        let path = self.mod_dir.join("worlds").join(format!("{track}.lap"));
        let Ok(text) = std::fs::read_to_string(&path) else { return };
        match lap::Track::from_text(&text) {
            Ok(lap) => {
                ctx.log(Level::Info, &format!("ride6-tracks: timing laps of {track} ({:.0} m)", lap.length()));
                if let Some(hud) = ctx.hud() {
                    let _ = hud.message(
                        &format!("Lap timing on: {:.0} m, the line is ahead of the grid", lap.length()),
                        MessageStyle::Info,
                        6.0,
                    );
                }
                self.lap = Some((track, Timer::new(lap)));
            }
            Err(error) => ctx.log(Level::Warn, &format!("ride6-tracks: the lap of {track} is not read: {error:#}")),
        }
    }

    /// A run is over: say what it did, and make its worlds available.
    fn finish(&self, ctx: &Ctx, summary: &Summary) {
        for report in &summary.imported {
            let megabytes = report.bytes as f64 / 1e6;
            ctx.log(
                Level::Info,
                &format!(
                    "ride6-tracks: world {}: {} cells, {} triangles, {} pictures, {megabytes:.1} MB",
                    report.world, report.cells, report.triangles, report.pictures
                ),
            );
            for (picture, reason) in &report.missing_pictures {
                ctx.log(Level::Warn, &format!("ride6-tracks: picture {picture} is drawn flat: {reason}"));
            }
        }
        for (pack, reason) in &summary.failed {
            ctx.log(Level::Warn, &format!("ride6-tracks: {pack} is left out: {reason}"));
        }
        if self.live {
            ctx.log(Level::Info, "ride6-tracks: the new worlds replace the old ones at the next start");
            return;
        }
        match job::promote(&self.mod_dir) {
            Ok(true) => {}
            Ok(false) => return ctx.log(Level::Warn, "ride6-tracks: the converted worlds are missing"),
            Err(error) => {
                return ctx.log(Level::Warn, &format!("ride6-tracks: the worlds are not in place: {error:#}"));
            }
        }
        match ctx.content().map(|content| content.rescan()) {
            Some(Ok(())) => ctx.log(Level::Info, &format!("ride6-tracks: {} worlds are ready", summary.imported.len())),
            Some(Err(error)) => {
                ctx.log(Level::Warn, &format!("ride6-tracks: the host did not take the worlds ({error}); restart"));
            }
            None => ctx.log(Level::Info, "ride6-tracks: the worlds are ready at the next start"),
        }
    }
}

declare_mod!(Ride6Tracks);

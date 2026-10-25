//! `mw2012-cars`: adds the cars of NFS Most Wanted (2012) to the game.
//!
//! The work is done in `load`. The mod reads `config.toml` in its own folder, which names the user's 2012 install, and
//! converts the cars into its own `files/` and `cars/` folders (`nfsmw-import2012`). The host reads those folders after
//! every mod has loaded, so the cars are part of the game's content for this run, like the cars of any content mod.
//! A later start converts nothing when the settings and the installs are as they were.
//!
//! A mod that fails does not stop the game: every problem is logged and the game starts without the cars.

use blackbox_modding::sdk::{Ctx, Level, Mod, ModResult, declare_mod};
use nfsmw_import2012::{Outcome, Report, prepare};

/// The mod keeps no state: the cars are files.
pub struct Mw2012Cars;

impl Mod for Mw2012Cars {
    fn load(ctx: &mut Ctx) -> ModResult<Self> {
        let (Ok(mod_dir), Ok(game_dir)) = (ctx.mod_dir(), ctx.game_dir()) else {
            ctx.log(
                Level::Warn,
                "mw2012-cars: the host does not say where the mod and the game are, so no car is added",
            );
            return Ok(Mw2012Cars);
        };
        match prepare(&mod_dir, &game_dir) {
            Ok(Outcome::NotConfigured { config, created }) => {
                let made = if created { " (written just now)" } else { "" };
                let message = format!(
                    "mw2012-cars: no 2012 install is set. Put `source = '<folder>'` in {}{made} and restart to add the cars",
                    config.display()
                );
                ctx.log(Level::Info, &message);
            }
            Ok(Outcome::UpToDate { cars }) => {
                ctx.log(Level::Info, &format!("mw2012-cars: {cars} cars from the last conversion are still current"));
            }
            Ok(Outcome::Imported(report)) => log_report(ctx, &report),
            Err(error) => ctx.log(Level::Warn, &format!("mw2012-cars: no car is added: {error:#}")),
        }
        Ok(Mw2012Cars)
    }
}

fn log_report(ctx: &Ctx, report: &Report) {
    ctx.log(
        Level::Info,
        &format!("mw2012-cars: converted {} cars, {} left out", report.imported.len(), report.skipped.len()),
    );
    for (car, reason) in &report.skipped {
        ctx.log(Level::Warn, &format!("mw2012-cars: {car} is left out: {reason}"));
    }
}

declare_mod!(Mw2012Cars);

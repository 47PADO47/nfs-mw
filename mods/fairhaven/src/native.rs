//! The native mod entry: `fairhaven` writes the manifests and stubs of the world (a few hundred small text files, once), and gives
//! the host the maker of the units' lazy files. Everything else happens when the game reads a unit (`maker`). The `files/`
//! folder this writes exists before the host indexes the mod's layers, so no rescan is needed.

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Instant;

use blackbox_modding::sdk::{Ctx, Event, LazyFiles, Level, Mod, ModResult, declare_mod};

use crate::config::{self, Config, MOD_ID};
use crate::convert::Options;
use crate::install::{Install, read_zone_list};
use crate::maker::Maker;
use crate::manifest::{self, WORLD_ID, WORLDS_FOLDER};
use crate::start;

/// The name the menu shows.
pub const WORLD_NAME: &str = "Fairhaven (NFS Most Wanted 2012)";
/// How often the conversions made by the loader threads are told to the log, in scene ticks.
const REPORT_EVERY: u64 = 60;

/// The mod's state: the maker, when the world is offered.
pub struct Fairhaven {
    maker: Option<Arc<Maker>>,
    ticks: u64,
}

/// The maker as the host's lazy file maker: the mod keeps its own handle to read the reports.
struct Shared(Arc<Maker>);

impl LazyFiles for Shared {
    fn read(&self, key: &str) -> ModResult<Vec<u8>> {
        self.0.read(key)
    }
}

impl Mod for Fairhaven {
    fn load(ctx: &mut Ctx) -> ModResult<Self> {
        let nothing = Self { maker: None, ticks: 0 };
        let Ok(mod_dir) = ctx.mod_dir() else {
            ctx.log(Level::Warn, "fairhaven: the host does not say where the mod is");
            return Ok(nothing);
        };
        let supplied = match supplied_config(ctx) {
            Ok(supplied) => supplied,
            Err(error) => {
                ctx.log(Level::Warn, &format!("fairhaven: no world is added, the settings are not read: {error}"));
                return Ok(nothing);
            }
        };
        let (config, created) = match Config::resolve(supplied.as_ref(), &mod_dir.join(config::FILE_NAME)) {
            Ok(resolved) => resolved,
            Err(error) => {
                ctx.log(Level::Warn, &format!("fairhaven: no world is added: {error:#}"));
                return Ok(nothing);
            }
        };
        if !config.enabled {
            ctx.log(Level::Info, "fairhaven: switched off in the settings (enabled = false)");
            return Ok(nothing);
        }
        let Some(source) = config.source.clone() else {
            let made = if created { " (written just now)" } else { "" };
            let place = format!(
                "{}{made}, or under [mods.{MOD_ID}] in the game's config file,",
                mod_dir.join(config::FILE_NAME).display()
            );
            ctx.log(
                Level::Info,
                &format!("fairhaven: no 2012 install is set. Put `source = '<folder>'` in {place} and restart"),
            );
            return Ok(nothing);
        };
        let Some(content) = ctx.content() else {
            ctx.log(Level::Warn, "fairhaven: the host has no bb.content, no world is added");
            return Ok(nothing);
        };
        if !content.supports_lazy_files() {
            ctx.log(
                Level::Warn,
                "fairhaven: this game is too old to make files when they are read (bb.content 1.1), no world is added",
            );
            return Ok(nothing);
        }
        match prepare(&mod_dir, &source) {
            Ok(summary) => ctx.log(Level::Info, &summary),
            Err(error) => {
                ctx.log(Level::Warn, &format!("fairhaven: no world is added: {error:#}"));
                return Ok(nothing);
            }
        }
        let maker =
            Arc::new(Maker::new(source, Options { max_texture: config.max_texture, brightness: config.brightness }));
        if let Err(error) = content.provide_lazy_files(Shared(maker.clone())) {
            ctx.log(
                Level::Warn,
                &format!("fairhaven: the host did not take the maker of the units ({error:?}), no world is added"),
            );
            return Ok(nothing);
        }
        Ok(Self { maker: Some(maker), ticks: 0 })
    }

    fn event(&mut self, ctx: &mut Ctx, event: &Event<'_>) {
        let Event::SceneTick { .. } = event else { return };
        self.ticks += 1;
        let Some(maker) = &self.maker else { return };
        if !self.ticks.is_multiple_of(REPORT_EVERY) {
            return;
        }
        for report in maker.take_reports() {
            match report {
                Ok(report) => {
                    let s = &report.stats;
                    ctx.log(
                        Level::Info,
                        &format!(
                            "fairhaven: unit {} made in {} ms: {} placements, {} solids, {} triangles, {} pictures ({} flat), {} collision triangles, {:.1} MB",
                            report.unit,
                            s.millis,
                            s.placements,
                            s.solids,
                            s.triangles,
                            s.textures,
                            s.textures_failed,
                            s.ground,
                            s.bytes as f64 / 1e6
                        ),
                    );
                }
                Err(reason) => ctx.log(Level::Warn, &format!("fairhaven: {reason}")),
            }
        }
    }
}

/// The settings the host merged from the mod's `config.toml` and the game's `[mods.fairhaven]` table (`bb.config`). `None` for a
/// host that does not offer `bb.config`: the mod then reads `config.toml` alone.
fn supplied_config(ctx: &Ctx) -> ModResult<Option<config::Supplied>> {
    let Some(host) = ctx.config() else { return Ok(None) };
    let text = host.text()?;
    Ok(Some(config::Supplied { text, has_file: host.has_file(), has_main_section: host.has_main_section() }))
}

/// Writes the manifests and stubs for the install in `source`; on the first start also finds where the player starts. Says what
/// it did in one line.
fn prepare(mod_dir: &Path, source: &Path) -> anyhow::Result<String> {
    let started = Instant::now();
    let zones = read_zone_list(source)?;
    let world_file: PathBuf = mod_dir.join(WORLDS_FOLDER).join(format!("{WORLD_ID}.toml"));
    let mut world = None;
    if !world_file.is_file() {
        let install = Install::open(source)?;
        let found = start::choose(&install)?;
        let (position, heading) = found.map_or(([0.0, 0.0, 100.0], 0.0), |start| (start.position, start.heading));
        world = Some((WORLD_NAME, position, heading));
    }
    let written = manifest::write(mod_dir, &zones, world)?;
    Ok(format!(
        "fairhaven: {} units are ready ({} files written in {} ms); each is made from the install when the camera comes near",
        written.units,
        written.changed,
        started.elapsed().as_millis()
    ))
}

declare_mod!(Fairhaven);

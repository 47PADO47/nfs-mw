//! The 2012 game's drift nitrous as a Gameplay setting: "2012 drift nitrous", off by default.
//!
//! In NFS Most Wanted (2012) a drift fills the nitrous tank. The rule, its numbers and where they are in the install are
//! `chameleon-vehicle`'s ([`chameleon_vehicle::DriftModel`], docs/formats/chameleon-drift.md). While the setting is on, this
//! module steps that model once a scene tick with what `bb.vehicles` (1.1) says of the player's car, whatever car it is, and
//! pays the nitrous it earned into the car's tank with `add_nos`. While a drift pays, it shows the 2012 text for it
//! ("DRIFTING" and the distance) on the HUD message line (`bb.hud`). With the setting off, or on a game without `bb.vehicles`
//! 1.1, nothing is read and nothing is changed.
//!
//! The numbers are the install's when it can be read: the payout of the burn nitrous comes from the first car bundle of the
//! 2012 install (it is the same in all 93 cars), the physics of a drift are the median car's.

use std::cell::RefCell;
use std::path::{Path, PathBuf};

use blackbox_modding::sdk::{Ctx, Event, Level, MessageStyle, VehicleState};
use chameleon_bundle::Bundle;
use chameleon_vehicle::{DriftInput, DriftModel, DriftParams, DriftPhysics, read_drift_nitrous};

use crate::config::{self, Config};

/// The option's id, as the game saves it under `[mod_values.nfsmw]`.
pub const OPTION: &str = "drift_nitrous";
/// What the Gameplay page shows.
pub const LABEL: &str = "2012 drift nitrous";
/// Seconds between two refreshes of the HUD text, and how long one stays up (a little longer, so it does not blink).
const MESSAGE_EVERY: f32 = 0.2;
const MESSAGE_LASTS: f32 = 0.6;

/// What one tick asks the host for.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Tick {
    /// The part of a full tank to add.
    pub gain: f32,
    /// The HUD text to show, when it is time to refresh it.
    pub message: Option<String>,
}

/// The rule running over the player's car.
pub struct Feed {
    model: DriftModel,
    since_message: f32,
}

impl Feed {
    pub fn new(params: DriftParams) -> Self {
        Self { model: DriftModel::new(params), since_message: MESSAGE_EVERY }
    }

    /// Forget the drift in progress (the setting went off, the car or the world went away).
    pub fn reset(&mut self) {
        self.model.reset();
        self.since_message = MESSAGE_EVERY;
    }

    /// Step the rule by `dt` seconds with the car as `bb.vehicles` reports it.
    pub fn tick(&mut self, car: &VehicleState, dt: f32) -> Tick {
        let input =
            DriftInput { dt, speed: car.speed, slip_angle: car.slip_angle, grounded: car.on_ground, hit: car.impact };
        let out = self.model.step(&input);
        self.since_message += dt.max(0.0);
        if !out.paying || self.since_message < MESSAGE_EVERY {
            return Tick { gain: out.gain, message: None };
        }
        self.since_message = 0.0;
        Tick { gain: out.gain, message: Some(message_text(out.distance)) }
    }
}

/// The 2012 text for a drift of `metres` ("DRIFTING {Distance}", string 24010 of its text tables).
pub fn message_text(metres: f32) -> String {
    format!("DRIFTING {metres:.0} m")
}

/// The rule's numbers from the car bundles of `source`, the 2012 install: the burn nitrous's payout from the first car, the
/// physics of the median car. `None` when no bundle reads.
pub fn params_from_install(source: &Path) -> Option<DriftParams> {
    let mut files: Vec<PathBuf> = std::fs::read_dir(source.join("VEHICLES"))
        .ok()?
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| {
            let name = path.file_name().map(|name| name.to_string_lossy().to_uppercase()).unwrap_or_default();
            name.starts_with("VEH_") && name.ends_with("_HI.BNDL")
        })
        .collect();
    files.sort();
    let data = std::fs::read(files.first()?).ok()?;
    let nitrous = read_drift_nitrous(&Bundle::parse(&data).ok()?).ok()?;
    Some(DriftParams::new(&DriftPhysics::TYPICAL, &nitrous.burn))
}

/// What the module keeps between ticks.
#[derive(Default)]
struct Runtime {
    on: bool,
    source: Option<PathBuf>,
    feed: Option<Feed>,
}

thread_local! {
    /// The mod runs on the host's main thread only.
    static RUNTIME: RefCell<Runtime> = RefCell::new(Runtime::default());
}

/// Add the setting to the Gameplay page. Offered only when a 2012 install is set (the caller checks), on a game with
/// `bb.settings`.
pub fn register(ctx: &Ctx) {
    let Some(settings) = ctx.settings() else {
        return;
    };
    let added = settings.add_toggle(OPTION, LABEL, false, |_ctx, on| {
        RUNTIME.with(|runtime| {
            let mut runtime = runtime.borrow_mut();
            runtime.on = on;
            if let Some(feed) = runtime.feed.as_mut() {
                feed.reset();
            }
        });
    });
    if let Err(error) = added {
        ctx.log(Level::Warn, &format!("nfsmw-2012-content: the drift nitrous option is not added: {error}"));
        return;
    }
    let saved = settings.get(OPTION).is_ok_and(|value| value == "on");
    let source = source_of(ctx);
    RUNTIME.with(|runtime| {
        let mut runtime = runtime.borrow_mut();
        runtime.on = saved;
        runtime.source = source;
    });
}

/// The 2012 install named in the mod's settings.
fn source_of(ctx: &Ctx) -> Option<PathBuf> {
    let mod_dir = ctx.mod_dir().ok()?;
    let supplied = super::supplied_config(ctx).ok()?;
    let (config, _) = Config::resolve(supplied.as_ref(), &mod_dir.join(config::FILE_NAME)).ok()?;
    config.source
}

/// Run the rule for a scene tick; any other event does nothing but forget a drift when the world goes.
pub fn event(ctx: &mut Ctx, event: &Event<'_>) {
    match event {
        Event::SceneTick { dt_sim, paused: false, .. } => tick(ctx, *dt_sim),
        Event::WorldUnloading { .. } => RUNTIME.with(|runtime| {
            if let Some(feed) = runtime.borrow_mut().feed.as_mut() {
                feed.reset();
            }
        }),
        _ => {}
    }
}

fn tick(ctx: &Ctx, dt: f32) {
    let Some(mut feed) = RUNTIME.with(|runtime| take_feed(&mut runtime.borrow_mut(), ctx)) else {
        return;
    };
    let vehicles = ctx.vehicles();
    let stepped = vehicles.as_ref().and_then(|vehicles| vehicles.player().ok()).map(|car| feed.tick(&car, dt));
    if stepped.is_none() {
        // No car, or a game without `bb.vehicles` 1.1: nothing is read, and a drift in progress is forgotten.
        feed.reset();
    }
    RUNTIME.with(|runtime| runtime.borrow_mut().feed = Some(feed));
    let (Some(tick), Some(vehicles)) = (stepped, vehicles) else {
        return;
    };
    if tick.gain > 0.0
        && let Err(error) = vehicles.add_nos(tick.gain.clamp(0.0, 1.0))
    {
        ctx.log(Level::Warn, &format!("nfsmw-2012-content: the drift nitrous is not added: {error}"));
    }
    if let (Some(text), Some(hud)) = (tick.message, ctx.hud()) {
        let _ = hud.message(&text, MessageStyle::Info, MESSAGE_LASTS);
    }
}

/// The feed to step, built the first time the setting is on; `None` while it is off. A feed that was running when the setting
/// went off is reset and kept.
fn take_feed(runtime: &mut Runtime, ctx: &Ctx) -> Option<Feed> {
    if !runtime.on {
        if let Some(feed) = runtime.feed.as_mut() {
            feed.reset();
        }
        return None;
    }
    if let Some(feed) = runtime.feed.take() {
        return Some(feed);
    }
    let params = match runtime.source.as_deref().and_then(params_from_install) {
        Some(params) => params,
        None => {
            ctx.log(Level::Info, "nfsmw-2012-content: the drift nitrous uses the median car's numbers");
            DriftParams::default()
        }
    };
    Some(Feed::new(params))
}

#[cfg(test)]
mod tests;

//! The 2012 game's drifting, for the 2012 cars: always on, with no setting. It covers the nitrous a drift earns and the drift
//! physics, and it applies only while the player drives one of this mod's converted cars ([`crate::drift_cars`]: the id the
//! game reports for the player's car, `bb.engine`, against the mod's `cars/` manifests). A stock car or a car of another mod
//! drives as it always did, and so does the same save after a switch of cars. Traffic and police cars are never read: the
//! rules follow the player's car only.
//!
//! **Physics.** While the player's car is a 2012 car, the mod keeps its drift assist on (`bb.vehicles` 1.2
//! `set_drift_assist`, re-sent every scene tick so it follows a change of car): the handbrake and a turned wheel start a
//! slide, the wheel holds its angle and the rear tyres let go part of their grip. The car's own numbers (when a slide is a
//! drift, the largest angle, the yaw torque) are keys of its `[stats]` (`drift_stats.rs`). In any other car the mod sends
//! `false` every tick. A game without `bb.vehicles` 1.2 answers `Unsupported` and the car drives as it did.
//!
//! **Nitrous.** In NFS Most Wanted (2012) a drift fills the nitrous tank. The rule, its numbers and where they are in the install are
//! `chameleon-vehicle`'s ([`chameleon_vehicle::DriftModel`], docs/formats/chameleon-drift.md). While the player drives a 2012
//! car, this module steps that model once a scene tick with what `bb.vehicles` (1.1) says of the car and pays the nitrous it
//! earned into the car's tank with `add_nos`. While a drift pays, it shows the 2012 text for it ("DRIFTING" and the distance)
//! on the HUD message line (`bb.hud`). In any other car, or on a game without `bb.engine` or `bb.vehicles` 1.1, nothing is
//! read and no nitrous is paid.
//!
//! The numbers are the install's when it can be read: the payout of the burn nitrous comes from the first car bundle of the
//! 2012 install (it is the same in all 93 cars), the physics of a drift are the median car's.

use std::cell::RefCell;
use std::path::{Path, PathBuf};

use blackbox_modding::sdk::{Ctx, Event, Level, MessageStyle, VehicleState};
use chameleon_bundle::Bundle;
use chameleon_vehicle::{DriftInput, DriftModel, DriftParams, DriftPhysics, read_drift_nitrous};

use crate::config::{self, Config};
use crate::drift_cars::KnownCars;

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

    /// Forget the drift in progress (the car changed to another make, or the world went away).
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
    source: Option<PathBuf>,
    cars: Option<KnownCars>,
    feed: Option<Feed>,
}

thread_local! {
    /// The mod runs on the host's main thread only.
    static RUNTIME: RefCell<Runtime> = RefCell::new(Runtime::default());
}

/// Start following the player's car. Called only when a 2012 install is set (the caller checks); it needs no setting.
pub fn register(ctx: &Ctx, mod_dir: &Path) {
    let source = source_of(ctx);
    RUNTIME.with(|runtime| {
        let mut runtime = runtime.borrow_mut();
        runtime.source = source;
        runtime.cars = Some(KnownCars::new(mod_dir));
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

/// Whether the player drives one of the mod's cars now. `false` without `bb.engine` (no way to tell), without a car, or
/// before [`register`].
fn player_is_2012(runtime: &mut Runtime, ctx: &Ctx) -> bool {
    let Some(cars) = runtime.cars.as_mut() else {
        return false;
    };
    let Some(engine) = ctx.engine() else {
        return false;
    };
    let Ok(car) = engine.car() else {
        return false;
    };
    cars.contains(&car)
}

fn tick(ctx: &Ctx, dt: f32) {
    let is_2012 = RUNTIME.with(|runtime| player_is_2012(&mut runtime.borrow_mut(), ctx));
    let vehicles = ctx.vehicles();
    if let Some(vehicles) = vehicles.as_ref() {
        // Sent every tick, true or false, so the assist follows the car: off in a stock car, on in a 2012 one. Not ready
        // without a car, unsupported before `bb.vehicles` 1.2: both leave the car as it was.
        let _ = vehicles.set_drift_assist(is_2012);
    }
    let Some(mut feed) = RUNTIME.with(|runtime| take_feed(&mut runtime.borrow_mut(), ctx, is_2012)) else {
        return;
    };
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

/// The feed to step, built the first time the player drives a 2012 car; `None` while the car is another make. A feed that was
/// running when the car changed is reset and kept.
fn take_feed(runtime: &mut Runtime, ctx: &Ctx, is_2012: bool) -> Option<Feed> {
    if !is_2012 {
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

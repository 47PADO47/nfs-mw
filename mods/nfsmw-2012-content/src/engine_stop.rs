//! The engine auto-stop of the 2012 game, as a gameplay setting (`docs/formats/chameleon-engine-state.md`,
//! `docs/modding/api-engine.md`). With **2012 engine auto-stop** on, a car that stands still for a few seconds while the
//! player drives free of any event switches its engine off, and the throttle (or a fresh press of the brake, which asks for
//! reverse from a stop) starts it again with the car's start-up sound. The game holds the gearbox in neutral and the lights
//! dark while the engine is off or cranking (`bb.engine`).
//!
//! The rule is `chameleon-engine-state`'s machine; the game does the switching through `bb.engine` (no throttle, no revs,
//! no engine sound while the engine is off). The setting is off by default, so the game plays as it did. A second row, **engine
//! auto-stop delay**, picks how long the vehicle (a car or a bike: the switch is the player's vehicle's) must stand still first.

use std::cell::{Cell, RefCell};
use std::path::{Path, PathBuf};
use std::rc::Rc;

use chameleon_engine_state::{EngineMachine, EngineParams, Event, Input};

use crate::ignition;
use blackbox_modding::sdk::{Ctx, Engine, Event as ModEvent, Level, SettingValue};

/// The setting's id in the gameplay settings.
pub const OPTION: &str = "engine_auto_stop";
/// What the menu shows for it: the id of its message in the mod's catalogue.
pub const LABEL: &str = "nfsmw2012-engine-auto-stop";
/// The id of the delay setting: how many seconds the vehicle stands still before the engine stops.
pub const DELAY_OPTION: &str = "engine_auto_stop_delay";
/// What the menu shows for the delay setting (a message id).
pub const DELAY_LABEL: &str = "nfsmw2012-engine-auto-stop-delay";
/// The delays the setting offers: the value id (seconds), its message id and the seconds. The first is the default, 5 s, the delay the
/// setting had before it could be chosen; the others are design values (`docs/specs/ride6-bike-standstill-hold.md`, section 5).
pub const DELAYS: [(&str, &str, f32); 5] = [
    ("5", "nfsmw2012-engine-delay-5", 5.0),
    ("10", "nfsmw2012-engine-delay-10", 10.0),
    ("20", "nfsmw2012-engine-delay-20", 20.0),
    ("30", "nfsmw2012-engine-delay-30", 30.0),
    ("60", "nfsmw2012-engine-delay-60", 60.0),
];
/// The gain the start sound plays at: the recording is already brought to its level (`ignition.rs`).
const START_GAIN: f32 = 1.0;

/// What the controller asks the game to do.
#[derive(Debug, Clone, PartialEq)]
pub enum Action {
    /// Switch the engine off.
    Stop,
    /// Start it with `sound` (a path in the mod's `files/`), running after `seconds`.
    Start { sound: String, seconds: f32 },
    /// Let it run at once, with no sound (the rule stopped applying, or the setting went off).
    Resume,
}

/// The driver's input and the car's speed in one tick.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Sample {
    pub dt: f32,
    pub speed: f32,
    pub throttle: f32,
    pub brake: f32,
    pub free_roam: bool,
}

/// The rule for the player's car: the machine, and which car it follows.
pub struct Controller {
    machine: EngineMachine,
    car: Option<String>,
    /// The start sound of `car`.
    sound: String,
}

/// The seconds of the delay value `id` (`DELAYS`); the default delay for an id that is not one.
pub fn delay_seconds(id: &str) -> f32 {
    DELAYS.iter().find(|(known, _, _)| *known == id).map_or(DELAYS[0].2, |(_, _, seconds)| *seconds)
}

impl Controller {
    pub fn new(params: EngineParams) -> Self {
        Self { machine: EngineMachine::new(params), car: None, sound: ignition::SHARED_PATH.to_owned() }
    }

    /// The seconds the vehicle must stand still before the engine stops (the delay setting).
    pub fn set_delay(&mut self, seconds: f32) {
        self.machine.set_idle_seconds(seconds);
    }

    /// One tick. `own_sound` says whether a car has an ignition recording of its own in the mod's files. A change of car, or
    /// the setting going off, puts the engine back to running (at once, with no sound, if it was off).
    pub fn step(
        &mut self,
        enabled: bool,
        sample: Sample,
        car: &str,
        own_sound: impl Fn(&str) -> bool,
    ) -> Option<Action> {
        let changed = self.car.as_deref() != Some(car);
        if changed {
            self.car = Some(car.to_owned());
            self.sound = match own_sound(car) {
                true => ignition::car_path(car),
                false => ignition::SHARED_PATH.to_owned(),
            };
        }
        if !enabled || changed {
            let was_running = self.machine.is_running();
            self.machine.reset();
            return (!was_running).then_some(Action::Resume);
        }
        let input = Input {
            dt: sample.dt,
            speed: sample.speed,
            throttle: sample.throttle,
            brake: sample.brake,
            active: sample.free_roam,
        };
        match self.machine.step(input) {
            Event::Stopped => Some(Action::Stop),
            Event::Cranking { seconds } => Some(Action::Start { sound: self.sound.clone(), seconds }),
            Event::Resumed => Some(Action::Resume),
            Event::Running | Event::None => None,
        }
    }
}

/// The feature as the mod holds it: the setting, the rule, and where the car files are.
pub struct EngineStop {
    enabled: Rc<Cell<bool>>,
    /// The seconds of the delay setting.
    delay: Rc<Cell<f32>>,
    controller: Controller,
    files: PathBuf,
}

thread_local! {
    /// The feature, kept for the main thread the mods run on.
    static ACTIVE: RefCell<Option<EngineStop>> = const { RefCell::new(None) };
}

/// Add the setting and start following the player's car. Does nothing when the game lacks `bb.settings` or `bb.engine`.
pub fn register(ctx: &Ctx, mod_dir: &Path) {
    ACTIVE.with(|active| *active.borrow_mut() = EngineStop::build(ctx, mod_dir));
}

/// Hand a game event to the feature.
pub fn event(ctx: &Ctx, event: &ModEvent<'_>) {
    ACTIVE.with(|active| {
        if let Some(feature) = active.borrow_mut().as_mut() {
            feature.event(ctx, event);
        }
    });
}

impl EngineStop {
    /// Add the setting. `None` when the game offers no `bb.settings` or no `bb.engine`: the feature is not there.
    fn build(ctx: &Ctx, mod_dir: &Path) -> Option<Self> {
        ctx.engine()?;
        let settings = ctx.settings()?;
        let enabled = Rc::new(Cell::new(false));
        let flag = enabled.clone();
        if let Err(error) =
            settings.section(crate::SECTION).add_toggle(OPTION, LABEL, false, move |_ctx, on| flag.set(on))
        {
            ctx.log(Level::Warn, &format!("nfsmw-2012-content: the engine auto-stop setting is not added: {error}"));
            return None;
        }
        enabled.set(settings.get(OPTION).is_ok_and(|value| value == "on"));
        let delay = Rc::new(Cell::new(DELAYS[0].2));
        let values = DELAYS.map(|(id, label, _)| SettingValue { id, label });
        let held = delay.clone();
        let added =
            settings
                .section(crate::SECTION)
                .add_option_with(DELAY_OPTION, DELAY_LABEL, &values, move |_ctx, id| held.set(delay_seconds(id)));
        if let Err(error) = added {
            ctx.log(
                Level::Warn,
                &format!("nfsmw-2012-content: the engine auto-stop delay setting is not added: {error}"),
            );
        }
        if let Ok(id) = settings.get(DELAY_OPTION) {
            delay.set(delay_seconds(&id));
        }
        let controller = Controller::new(EngineParams::default());
        Some(Self { enabled, delay, controller, files: mod_dir.join("files") })
    }

    /// Follow the scene's tick: read the car, run the rule, and switch the engine.
    fn event(&mut self, ctx: &Ctx, event: &ModEvent<'_>) {
        let ModEvent::SceneTick { dt_sim, paused, .. } = *event else { return };
        if paused || dt_sim <= 0.0 {
            return;
        }
        let Some(engine) = ctx.engine() else { return };
        let (Ok(state), Ok(car)) = (engine.state(), engine.car()) else { return };
        let sample = Sample {
            dt: dt_sim,
            speed: state.speed,
            throttle: state.throttle,
            brake: state.brake,
            free_roam: state.free_roam,
        };
        let files = &self.files;
        let own = |car: &str| files.join(ignition::car_path(car)).is_file();
        self.controller.set_delay(self.delay.get());
        let Some(action) = self.controller.step(self.enabled.get(), sample, &car, own) else { return };
        if let Err(error) = apply(&engine, &action) {
            ctx.log(Level::Warn, &format!("nfsmw-2012-content: the engine switch failed ({action:?}): {error}"));
        }
    }
}

/// Make `action` on the game's engine.
fn apply(engine: &Engine, action: &Action) -> blackbox_modding::sdk::ModResult<()> {
    match action {
        Action::Stop => engine.stop(),
        Action::Start { sound, seconds } => engine.start(sound, START_GAIN, *seconds),
        Action::Resume => engine.start("", START_GAIN, 0.0),
    }
}

#[cfg(test)]
mod tests;

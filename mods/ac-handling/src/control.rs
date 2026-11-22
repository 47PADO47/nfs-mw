//! The state machine behind the switches: what the player asked for, which hooks are in place, and the model of the current car.
//!
//! Pure (no host calls), so the toggle logic is tested without a game. The glue in `lib.rs` asks [`Control::plan`] at each frame
//! start, does what it says with the host, and tells the result back with [`Control::confirm`]. Settings handlers, the hotkey and
//! the console command only change the request; nothing registers or unregisters inside them, so a switch always lands on a
//! frame boundary. The three hooks are the tyre hook (the master switch), the suspension hook (the suspension unit) and the
//! controls hook (the aids or the steering unit); the thermal unit lives inside the tyre hook. The master switch off means no
//! hook at all.

use std::collections::HashSet;
use std::path::PathBuf;

use blackbox_modding::sdk::Level;

use crate::commands::LevelKind;
use crate::mapping::{CarSpec, MapOptions, map_car};
use crate::model::Model;
use crate::presets::{CarFile, PresetSet, UnitDefaults, parse_car_file};
use crate::settings::{PresetChoice, Settings, Unit, UnitMode, clamp_grip_gain};
use crate::steering::Device;

/// The ambient temperature the host reports when it has no weather, degrees Celsius.
pub const DEFAULT_AMBIENT_C: f64 = 20.0;
/// Seconds of simulated time between two HUD messages of the tyre temperatures.
pub(crate) const HUD_PERIOD: f64 = 5.0;

/// One of the three hooks of `bb.handling`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Hook {
    /// The tyre hook (the master switch).
    Tyre,
    /// The suspension hook.
    Suspension,
    /// The controls hook.
    Controls,
}

/// What to do with one hook at this frame boundary.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Step {
    /// Nothing.
    Nothing,
    /// Register it.
    Register,
    /// Unregister it.
    Unregister,
}

/// What the glue must do with the host at this frame boundary.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Plan {
    /// The tyre hook.
    pub tyre: Step,
    /// The suspension hook.
    pub suspension: Step,
    /// The controls hook.
    pub controls: Step,
}

impl Plan {
    /// Whether every step is nothing.
    pub fn is_idle(&self) -> bool {
        [self.tyre, self.suspension, self.controls].iter().all(|step| *step == Step::Nothing)
    }
}

/// Which hooks are registered.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Applied {
    /// The tyre hook.
    pub tyre: bool,
    /// The suspension hook.
    pub suspension: bool,
    /// The controls hook.
    pub controls: bool,
}

impl Applied {
    fn get(&self, hook: Hook) -> bool {
        match hook {
            Hook::Tyre => self.tyre,
            Hook::Suspension => self.suspension,
            Hook::Controls => self.controls,
        }
    }

    fn set(&mut self, hook: Hook, on: bool) {
        match hook {
            Hook::Tyre => self.tyre = on,
            Hook::Suspension => self.suspension = on,
            Hook::Controls => self.controls = on,
        }
    }
}

/// A line for the game log.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Note {
    /// How loud.
    pub level: Level,
    /// The text, one line.
    pub text: String,
}

/// The switches, the presets and the model.
pub struct Control {
    /// The player asked for the model (the master toggle, the hotkey or the console).
    pub requested: bool,
    /// Which hooks are registered.
    pub applied: Applied,
    /// The compound setting.
    pub preset: PresetChoice,
    /// The grip gain.
    pub grip_gain: f64,
    /// The substeps setting (0 automatic).
    pub substeps: u32,
    /// The mode of each optional unit.
    pub units: crate::settings::UnitModes,
    /// The levels the player set.
    pub levels: crate::settings::LevelSettings,
    /// The steering device.
    pub device: Device,
    /// The pad's speed sensitivity, when the player set it.
    pub steer_sens: Option<f64>,
    /// The force-feedback gain, when the player set it.
    pub ffb_gain: Option<f64>,
    /// The force-feedback curve exponent, when the player set it.
    pub ffb_curve: Option<f64>,
    /// The tyres start warm.
    pub warm_start: bool,
    /// Show the tyre temperatures on the HUD.
    pub hud: bool,
    pub(crate) presets: PresetSet,
    pub(crate) mod_dir: Option<PathBuf>,
    pub(crate) model: Option<Model>,
    pub(crate) last_spec: Option<CarSpec>,
    pub(crate) needs_spec: bool,
    pub(crate) ambient_c: f64,
    pub(crate) logged_cars: HashSet<String>,
    pub(crate) notes: Vec<Note>,
    pub(crate) blocked: [bool; 2],
    pub(crate) last_mz: [f64; 2],
    pub(crate) last_omega: [f64; 2],
    pub(crate) front_steer: [f64; 2],
    pub(crate) sim_time: f64,
    pub(crate) hud_due: f64,
}

impl Control {
    /// A control in its starting state: nothing registered, the model of no car built.
    pub fn new(presets: PresetSet, mod_dir: Option<PathBuf>, settings: Settings) -> Control {
        Control {
            requested: settings.enabled,
            applied: Applied::default(),
            preset: settings.preset,
            grip_gain: settings.grip_gain,
            substeps: settings.substeps,
            units: settings.units,
            levels: settings.levels,
            device: settings.device,
            steer_sens: settings.steer_sens,
            ffb_gain: settings.ffb_gain,
            ffb_curve: settings.ffb_curve,
            warm_start: settings.warm_start,
            hud: settings.hud,
            presets,
            mod_dir,
            model: None,
            last_spec: None,
            needs_spec: false,
            ambient_c: DEFAULT_AMBIENT_C,
            logged_cars: HashSet::new(),
            notes: Vec::new(),
            blocked: [false; 2],
            last_mz: [0.0; 2],
            last_omega: [0.0; 2],
            front_steer: [0.0; 2],
            sim_time: 0.0,
            hud_due: HUD_PERIOD,
        }
    }

    /// Queue a line for the log.
    pub fn note(&mut self, level: Level, text: impl Into<String>) {
        self.notes.push(Note { level, text: text.into() });
    }

    /// The lines queued since the last call.
    pub fn take_notes(&mut self) -> Vec<Note> {
        std::mem::take(&mut self.notes)
    }

    /// The unit defaults of the car being driven (the hard-coded ones before a car is read).
    pub fn unit_defaults(&self) -> UnitDefaults {
        self.model.as_ref().map(|model| model.params.units).unwrap_or_default()
    }

    /// Whether `unit` is on, given the mode and the car's preset default. The master switch is not part of it.
    pub fn unit_on(&self, unit: Unit) -> bool {
        let defaults = self.unit_defaults();
        let preset = match unit {
            Unit::Suspension => defaults.suspension,
            Unit::Aids => defaults.aids,
            Unit::Steering => defaults.steering,
            Unit::Thermal => defaults.thermal,
        };
        self.units.get(unit).resolve(preset)
    }

    fn wanted(&self, hook: Hook) -> bool {
        if !self.requested {
            return false;
        }
        match hook {
            Hook::Tyre => true,
            Hook::Suspension => self.unit_on(Unit::Suspension) && !self.blocked[0],
            Hook::Controls => (self.unit_on(Unit::Aids) || self.unit_on(Unit::Steering)) && !self.blocked[1],
        }
    }

    /// What to do with the host at this frame boundary.
    pub fn plan(&mut self) -> Plan {
        let step = |control: &Control, hook: Hook| match (control.wanted(hook), control.applied.get(hook)) {
            (true, false) => Step::Register,
            (false, true) => Step::Unregister,
            _ => Step::Nothing,
        };
        Plan {
            tyre: step(self, Hook::Tyre),
            suspension: step(self, Hook::Suspension),
            controls: step(self, Hook::Controls),
        }
    }

    /// The host did (or refused) one step of the plan. A refused tyre registration clears the request, so it is not retried every
    /// frame; a refused suspension or controls registration blocks that hook until the player changes a switch.
    pub fn confirm(&mut self, hook: Hook, step: Step, ok: bool) {
        match (step, ok) {
            (Step::Register, true) => {
                self.applied.set(hook, true);
                match hook {
                    Hook::Tyre => {
                        self.needs_spec = true;
                        self.reseed();
                    }
                    Hook::Suspension => {
                        if let Some(model) = self.model.as_mut() {
                            model.axles = [crate::suspension::AxleState::new(), crate::suspension::AxleState::new()];
                        }
                    }
                    Hook::Controls => {
                        if let Some(model) = self.model.as_mut() {
                            model.controls.reset();
                        }
                    }
                }
            }
            (Step::Register, false) => match hook {
                Hook::Tyre => self.requested = false,
                Hook::Suspension => self.blocked[0] = true,
                Hook::Controls => self.blocked[1] = true,
            },
            (Step::Unregister, true) => {
                self.applied.set(hook, false);
                if hook == Hook::Tyre {
                    self.model = None;
                }
            }
            (Step::Unregister, false) => {}
            (Step::Nothing, _) => {}
        }
    }

    /// Whether the glue should read the car's spec now: the model is on and has none, or the car changed.
    pub fn wants_spec(&self) -> bool {
        self.applied.tyre && (self.needs_spec || self.model.is_none())
    }

    /// Start every wheel cold: fresh slip state, new hubs, the aids and steering from nothing, the thermal state reset.
    pub fn reseed(&mut self) {
        let ambient = self.ambient_c;
        let Some(model) = self.model.as_mut() else {
            return;
        };
        model.reseed(ambient);
    }

    /// Build the model of the car `spec` describes: its parameter set, read once per car change. The car's tuning file is read
    /// here, and a car with none is logged once.
    pub fn load_spec(&mut self, spec: CarSpec) {
        self.needs_spec = false;
        let name = spec.key_name();
        let (file, file_note) = self.car_file(&name);
        let options = MapOptions { car: file.as_ref(), forced: self.preset.forced() };
        let params = map_car(&spec, &self.presets, &options);
        if let Some(note) = file_note {
            self.note(Level::Warn, note);
        }
        let low = params.front.inertia_scale.min(params.rear.inertia_scale);
        if low < 0.5 && !self.logged_cars.contains(&name) {
            self.note(Level::Warn, format!("ac-handling: car \"{name}\": inertia_scale {low:.2} is below 0.5: the host's drivetrain may oscillate"));
        }
        if self.logged_cars.insert(name.clone()) {
            let tail = if file.is_some() { format!("tuning/{name}.toml") } else { format!("no tuning/{name}.toml") };
            let [front, rear] = spec.static_corner_loads();
            let loads = format!("{front:.0} N front, {rear:.0} N rear per wheel");
            self.note(Level::Info, format!("ac-handling: car \"{name}\": {}, {loads}, {tail}", params.summary()));
        }
        let mut model = Model::new(&spec, params, name);
        let thermal = self.units.get(Unit::Thermal).resolve(model.params.units.thermal);
        model.set_thermal(thermal, self.warm_start, self.ambient_c);
        self.model = Some(model);
        self.last_spec = Some(spec);
    }

    /// The tuning file of a car: the parsed file, or the one-line reason it was rejected. `spec-invalid` has none.
    fn car_file(&self, name: &str) -> (Option<CarFile>, Option<String>) {
        let Some(dir) = &self.mod_dir else {
            return (None, None);
        };
        if name == "spec-invalid" {
            return (None, None);
        }
        let Ok(text) = std::fs::read_to_string(dir.join("tuning").join(format!("{name}.toml"))) else {
            return (None, None);
        };
        match parse_car_file(&text) {
            Ok(file) => (Some(file), None),
            Err(reason) => {
                (None, Some(format!("ac-handling: tuning/{name}.toml: {reason}; using the automatic mapping")))
            }
        }
    }

    fn rebuild(&mut self) {
        let Some(spec) = self.last_spec.clone() else {
            return;
        };
        self.load_spec(spec);
    }

    /// Choose another compound. The parameter set is rebuilt from the car's last spec and everything starts cold.
    pub fn set_preset(&mut self, preset: PresetChoice) {
        if preset == self.preset {
            return;
        }
        self.preset = preset;
        self.rebuild();
    }

    /// Set the grip gain, kept inside its range.
    pub fn set_grip_gain(&mut self, value: f64) {
        self.grip_gain = clamp_grip_gain(value);
    }

    /// Set the mode of an optional unit. Changing the thermal unit gives new tyres, changing the suspension a new hub state and
    /// changing the aids or the steering a fresh controls state; a hook that was refused is tried again.
    pub fn set_unit(&mut self, unit: Unit, mode: UnitMode) {
        if self.units.get(unit) == mode {
            return;
        }
        self.units.set(unit, mode);
        self.blocked = [false; 2];
        let ambient = self.ambient_c;
        let on = self.unit_on(unit);
        let (warm, ambient_c) = (self.warm_start, ambient);
        let Some(model) = self.model.as_mut() else {
            return;
        };
        match unit {
            Unit::Thermal => model.set_thermal(on, warm, ambient_c),
            Unit::Suspension => {
                model.axles = [crate::suspension::AxleState::new(), crate::suspension::AxleState::new()]
            }
            Unit::Aids | Unit::Steering => model.controls.reset(),
        }
    }

    /// Set one level of the aids (the bias takes clicks, which may be negative).
    pub fn set_level(&mut self, kind: LevelKind, value: i32) {
        let level = value.max(0) as u32;
        match kind {
            LevelKind::Tc => self.levels.tc = Some(level.min(crate::aids::MAX_LEVEL)),
            LevelKind::Abs => self.levels.abs = Some(level.min(crate::aids::MAX_LEVEL)),
            LevelKind::Esc => self.levels.esc = Some(level.min(crate::aids::MAX_ESC_LEVEL)),
            LevelKind::Bias => self.set_bias(value),
        }
    }

    /// Set the brake bias in clicks (the slider and the command carry the sign).
    pub fn set_bias(&mut self, clicks: i32) {
        let limit = crate::aids::MAX_CLICKS;
        self.levels.bias_clicks = Some(clicks.clamp(-limit, limit));
    }

    /// The hotkey: flip the request.
    pub fn toggle(&mut self) {
        self.requested = !self.requested;
        self.blocked = [false; 2];
    }

    /// Switch the master on or off.
    pub fn set_requested(&mut self, on: bool) {
        self.requested = on;
        self.blocked = [false; 2];
    }
}

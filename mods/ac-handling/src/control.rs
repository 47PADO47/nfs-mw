//! The state machine behind the switches: what the player asked for, what is in place, and the model of the current car.
//!
//! Pure (no host calls), so the toggle logic is tested without a game. The glue in `lib.rs` asks [`Control::plan`] at each
//! frame start, does what it says with the host, and tells the result back with [`Control::confirm`]. Settings handlers,
//! the hotkey and the console command only change the request; nothing registers or unregisters inside them, so a switch
//! always lands on a frame boundary.

use std::collections::HashSet;
use std::path::PathBuf;

use blackbox_modding::sdk::{Level, Role, TyreQuery, TyreResult};

use crate::bridge::{to_input, to_result};
use crate::commands::Command;
use crate::grip::{GripInputs, GripScale, NoThermal, sanitize};
use crate::mapping::{CarParams, CarSpec, MapOptions, map_car};
use crate::presets::{CarFile, PresetSet, parse_car_file};
use crate::settings::{PresetChoice, Settings, clamp_grip_gain};
use crate::wheel::{WheelState, wheel_tick};

/// The ambient temperature the host reports when it has no weather, degrees Celsius.
const DEFAULT_AMBIENT_C: f64 = 20.0;

/// What the glue must do with the host at this frame boundary.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Plan {
    /// Nothing.
    Nothing,
    /// Register the tyre hook.
    Register,
    /// Unregister the tyre hook.
    Unregister,
}

/// A line for the game log.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Note {
    /// How loud.
    pub level: Level,
    /// The text, one line.
    pub text: String,
}

/// The model of the car being driven: its tyres, its four wheels' state and the thermal seam.
struct Model {
    params: CarParams,
    wheels: [WheelState; 4],
    thermal: Box<dyn GripScale>,
    name: String,
}

/// The switches, the presets and the model.
pub struct Control {
    /// The player asked for the model (the master toggle, the hotkey or the console).
    pub requested: bool,
    /// The tyre hook is registered.
    pub applied: bool,
    /// The compound setting.
    pub preset: PresetChoice,
    /// The grip gain.
    pub grip_gain: f64,
    /// The substeps setting (0 automatic).
    pub substeps: u32,
    presets: PresetSet,
    mod_dir: Option<PathBuf>,
    model: Option<Model>,
    last_spec: Option<CarSpec>,
    needs_spec: bool,
    ambient_c: f64,
    logged_cars: HashSet<String>,
    notes: Vec<Note>,
}

impl Control {
    /// A control in its starting state: nothing registered, the model of no car built.
    pub fn new(presets: PresetSet, mod_dir: Option<PathBuf>, settings: Settings) -> Control {
        Control {
            requested: settings.enabled,
            applied: false,
            preset: settings.preset,
            grip_gain: settings.grip_gain,
            substeps: settings.substeps,
            presets,
            mod_dir,
            model: None,
            last_spec: None,
            needs_spec: false,
            ambient_c: DEFAULT_AMBIENT_C,
            logged_cars: HashSet::new(),
            notes: Vec::new(),
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

    /// What to do with the host at this frame boundary.
    pub fn plan(&mut self) -> Plan {
        if self.requested && !self.applied {
            return Plan::Register;
        }
        if !self.requested && self.applied {
            return Plan::Unregister;
        }
        Plan::Nothing
    }

    /// The host did (or refused) what [`Control::plan`] said. A refused registration clears the request, so it is not retried
    /// every frame.
    pub fn confirm(&mut self, plan: Plan, ok: bool) {
        match (plan, ok) {
            (Plan::Register, true) => {
                self.applied = true;
                self.needs_spec = true;
                self.reseed();
            }
            (Plan::Register, false) => self.requested = false,
            (Plan::Unregister, true) => {
                self.applied = false;
                self.model = None;
            }
            (Plan::Unregister, false) => self.requested = true,
            (Plan::Nothing, _) => {}
        }
    }

    /// Whether the glue should read the car's spec now: the model is on and has none, or the car changed.
    pub fn wants_spec(&self) -> bool {
        self.applied && (self.needs_spec || self.model.is_none())
    }

    /// Start every wheel cold: fresh slip state and a cold thermal seam at the ambient temperature.
    pub fn reseed(&mut self) {
        let ambient = self.ambient_c;
        let Some(model) = self.model.as_mut() else {
            return;
        };
        model.wheels = [WheelState::fresh(); 4];
        model.thermal.reset(ambient);
    }

    /// Build the model of the car `spec` describes: its parameter set, read once per car change. The car's override file is
    /// read here, and a car with none is logged once.
    pub fn load_spec(&mut self, spec: CarSpec) {
        self.needs_spec = false;
        let name = spec.key_name();
        let (file, file_note) = self.car_file(&name);
        let options = MapOptions { car: file.as_ref(), forced: self.preset.forced() };
        let params = map_car(&spec, &self.presets, &options);
        if let Some(note) = file_note {
            self.note(Level::Warn, note);
        }
        if self.logged_cars.insert(name.clone()) {
            let tail = if file.is_some() { format!("cars/{name}.toml") } else { format!("no cars/{name}.toml") };
            let [front, rear] = spec.static_corner_loads();
            let loads = format!("{front:.0} N front, {rear:.0} N rear per wheel");
            self.note(Level::Info, format!("ac-handling: car \"{name}\": {}, {loads}, {tail}", params.summary()));
        }
        let mut model = Model { params, wheels: [WheelState::fresh(); 4], thermal: Box::new(NoThermal), name };
        model.thermal.reset(self.ambient_c);
        self.model = Some(model);
        self.last_spec = Some(spec);
    }

    /// The override file of a car: the parsed file, or the one-line reason it was rejected.
    fn car_file(&self, name: &str) -> (Option<CarFile>, Option<String>) {
        let Some(dir) = &self.mod_dir else {
            return (None, None);
        };
        let Ok(text) = std::fs::read_to_string(dir.join("cars").join(format!("{name}.toml"))) else {
            return (None, None);
        };
        match parse_car_file(&text) {
            Ok(file) => (Some(file), None),
            Err(reason) => {
                (None, Some(format!("ac-handling: cars/{name}.toml: {reason}; using the automatic mapping")))
            }
        }
    }

    /// Choose another compound. The parameter set is rebuilt from the car's last spec and the wheels start cold.
    pub fn set_preset(&mut self, preset: PresetChoice) {
        if preset == self.preset {
            return;
        }
        self.preset = preset;
        let Some(spec) = self.last_spec.clone() else {
            return;
        };
        self.load_spec(spec);
    }

    /// Set the grip gain, kept inside its range.
    pub fn set_grip_gain(&mut self, value: f64) {
        self.grip_gain = clamp_grip_gain(value);
    }

    /// The hotkey: flip the request.
    pub fn toggle(&mut self) {
        self.requested = !self.requested;
    }

    /// Run a console command and answer with its output.
    pub fn run(&mut self, command: Command) -> String {
        match command {
            Command::On => {
                self.requested = true;
                "ac-handling: on (takes effect at the next frame)".to_owned()
            }
            Command::Off => {
                self.requested = false;
                "ac-handling: off (takes effect at the next frame)".to_owned()
            }
            Command::Status => self.status(),
            Command::Preset(choice) => {
                self.set_preset(choice);
                format!("ac-handling: preset {}", choice.name())
            }
        }
    }

    /// One line: the switch, the preset, the grip gain and the car.
    pub fn status(&self) -> String {
        let car = match &self.model {
            Some(model) => format!("car {} ({})", model.name, model.params.summary()),
            None => "no car read yet".to_owned(),
        };
        format!(
            "ac-handling: {}, hook {}, preset {}, grip {:.2}, {car}",
            if self.requested { "on" } else { "off" },
            if self.applied { "registered" } else { "not registered" },
            self.preset.name(),
            self.grip_gain
        )
    }

    /// The tyre hook: one wheel, one tick. `None` is "not handled" (every other role, no model yet, a refused answer).
    pub fn tyre(&mut self, q: &TyreQuery) -> Option<TyreResult> {
        if q.role != Some(Role::Player) || !self.applied {
            return None;
        }
        let Some(model) = self.model.as_mut() else {
            self.needs_spec = true;
            return None;
        };
        if q.vehicle_revision != model.params.revision {
            // The car changed (or the spec lags the query by a tick): keep the old parameters for this tick, read again.
            self.needs_spec = true;
        }
        let wheel = q.wheel_index as usize;
        if wheel > 3 {
            return None;
        }
        if q.ambient_temperature_c.is_finite() {
            self.ambient_c = f64::from(q.ambient_temperature_c);
        }
        let inputs = GripInputs { t_eff: self.ambient_c, ..GripInputs::default() };
        let thermal = sanitize(model.thermal.grip_scale(wheel, &inputs));
        let condition = f64::from(q.tyre_condition).clamp(0.0, 1.0);
        let input = to_input(q, self.grip_gain * condition * thermal);
        let state = &mut model.wheels[wheel];
        let output = wheel_tick(model.params.axle(wheel), state, &input, self.substeps)?;
        let result = to_result(&output);
        if result.is_none() {
            *state = WheelState::fresh();
        }
        result
    }
}

//! The console command, the status line, the force-feedback accessor and the tyre HUD line of [`Control`].

use crate::commands::{Command, LevelKind};
use crate::control::{Control, HUD_PERIOD};
use crate::settings::Unit;

impl Control {
    /// Run a console command and answer with its output.
    pub fn run(&mut self, command: Command) -> String {
        match command {
            Command::On => {
                self.set_requested(true);
                "ac-handling: on (takes effect at the next frame)".to_owned()
            }
            Command::Off => {
                self.set_requested(false);
                "ac-handling: off (takes effect at the next frame)".to_owned()
            }
            Command::Status => self.status(),
            Command::Ffb => format!(
                "ac-handling: force feedback signal {:+.3} (column torque {:+.2} N m)",
                self.ffb_signal(),
                self.column_torque()
            ),
            Command::Preset(choice) => {
                self.set_preset(choice);
                format!("ac-handling: preset {}", choice.name())
            }
            Command::Unit(unit, mode) => {
                self.set_unit(unit, mode);
                format!("ac-handling: {} {}", unit.name(), mode.name())
            }
            Command::Level(kind, level) => {
                self.set_level(kind, level);
                let name = match kind {
                    LevelKind::Tc => "traction control",
                    LevelKind::Abs => "ABS",
                    LevelKind::Esc => "stability control",
                    LevelKind::Bias => "brake bias (clicks)",
                };
                format!("ac-handling: {name} {level}")
            }
            Command::Device(device) => {
                self.device = device;
                format!("ac-handling: steering device {}", device.name())
            }
            Command::Hud(on) => {
                self.hud = on;
                format!("ac-handling: tyre HUD {}", if on { "on" } else { "off" })
            }
        }
    }

    /// The force-feedback value of the steering unit, -1 to 1 (positive pulls the wheel to the right): an optional output of the
    /// mod itself, computed and dropped (the host has no force-feedback output). 0 with the steering unit off.
    pub fn ffb_signal(&self) -> f64 {
        self.model.as_ref().map_or(0.0, |model| model.controls.ffb)
    }

    /// The column torque behind [`Control::ffb_signal`], N m.
    pub fn column_torque(&self) -> f64 {
        self.model.as_ref().map_or(0.0, |model| model.controls.col_torque)
    }

    /// One line: the switches, the units, the preset, the grip gain and the car.
    pub fn status(&self) -> String {
        let car = match &self.model {
            Some(model) => format!("car {} ({})", model.name, model.params.summary()),
            None => "no car read yet".to_owned(),
        };
        let units: Vec<String> = Unit::ALL
            .iter()
            .map(|unit| format!("{} {}", unit.name(), if self.unit_on(*unit) { "on" } else { "off" }))
            .collect();
        let hooks =
            [("tyre", self.applied.tyre), ("suspension", self.applied.suspension), ("controls", self.applied.controls)]
                .iter()
                .filter(|(_, on)| *on)
                .map(|(name, _)| *name)
                .collect::<Vec<_>>()
                .join("+");
        let registered = if hooks.is_empty() { "not registered".to_owned() } else { format!("registered ({hooks})") };
        format!(
            "ac-handling: {}, hook {registered}, preset {}, grip {:.2}, units: {}, {car}",
            if self.requested { "on" } else { "off" },
            self.preset.name(),
            self.grip_gain,
            units.join(", ")
        )
    }

    /// Count simulated time (the tyre hook calls it once per tick) and, when the HUD is on and it is time, the line to show.
    pub(crate) fn tick_clock(&mut self, dt: f64) {
        self.sim_time += dt;
    }

    /// The tyre temperatures line for the HUD, once every few seconds of driving, when the HUD setting and the thermal unit are
    /// on. `None` otherwise: the host may have no HUD, and the mod never needs one.
    pub fn hud_line(&mut self) -> Option<String> {
        if !self.hud || !self.applied.tyre || self.sim_time < self.hud_due {
            return None;
        }
        let model = self.model.as_ref()?;
        if !model.thermal_on {
            return None;
        }
        self.hud_due = self.sim_time + HUD_PERIOD;
        let values: Vec<_> = (0..4).filter_map(|wheel| model.thermal.hud(wheel, model.camber[wheel])).collect();
        if values.len() < 4 {
            return None;
        }
        let core: Vec<String> = values.iter().map(|v| format!("{:.0}", v.core)).collect();
        let life = values.iter().map(|v| v.life).fold(f64::INFINITY, f64::min);
        Some(format!("Tyres (core C): {}  {:.0} kPa  life {:.0}%", core.join(" "), values[0].pressure, life))
    }
}

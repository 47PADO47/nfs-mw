//! The Speedbreaker tunables, read from `NFSMWSpeedFixerSettings.ini`: the file and keys of the NFSMWSpeedFixer
//! mod, in its units (km/h, seconds, degrees, percent). The reader is `ini-tuning`; this file holds the NFS:MW
//! schema and the vanilla values. The vanilla values are the PC values in `docs/specs/speedbreaker.md`.
//!
//! Rules: a missing file is vanilla throughout; a value outside its range is clamped to the nearest bound; a
//! malformed value is vanilla. Two keys follow the mod's own omission rules: an absent `[Activation] maxDuration`
//! means unlimited, and passive recharge is disabled when both of its keys are absent.

use std::path::Path;

use ini_tuning::{Entry, Range, Read, entries, read_number};

use super::extra_options;

/// The file's name, next to the config file (the mod's own name for it).
pub const FILE_NAME: &str = "NFSMWSpeedFixerSettings.ini";

/// Kilometres per hour in one metre per second.
const KPH_PER_MPS: f32 = 3.6;

/// The vanilla PC values. Each is a physical constant of the original, not a tuning choice.
pub mod vanilla {
    /// Miles per hour, in metres per second.
    pub const MPH: f32 = 0.44704;
    /// Speed (m/s) to activate, and below which an active breaker turns off: 30 mph.
    pub const MINIMUM_SPEED_MPS: f32 = 30.0 * MPH;
    /// Real seconds for a full charge to drain while active.
    pub const DRAIN_SECONDS: f32 = 10.0;
    /// Passive recharge happens strictly above this speed: 100 mph.
    pub const RECHARGE_ABOVE_MPS: f32 = 100.0 * MPH;
    /// Simulation seconds for a passive recharge to fill the meter.
    pub const RECHARGE_SECONDS: f32 = 25.0;
    /// Drift recharge needs a speed strictly above 35 mph (the PC value, as read from the executable).
    pub const DRIFT_MIN_SPEED_MPS: f32 = 15.646_05;
    /// Drift recharge needs a slip angle strictly above 30 degrees (radians, as read from the executable).
    pub const DRIFT_MIN_SLIP_RADIANS: f32 = 0.523_589_97;
    /// The simulation-time multiplier while active: one quarter speed.
    pub const DILATION_TARGET: f32 = 0.25;
    /// Multiplier change per real second on entry, which takes 0.375 s from normal speed.
    pub const ENTER_RATE: f32 = 2.0;
    /// Multiplier change per real second on exit, which takes 1.5 s from quarter speed.
    pub const EXIT_RATE: f32 = 0.5;
    /// Collision mass scale while active.
    pub const CAR_MASS_SCALE: f32 = 2.0;
    /// Gravity scale for the downforce: 3 times gravity, which is two extra g.
    pub const GRAVITY_SCALE: f32 = 3.0;
    /// Front-wheel grip boost per unit of absolute steering input.
    pub const FRONT_GRIP_GAIN: f32 = 0.75;
    /// Steering angle at full lock while active (degrees).
    pub const MAX_STEERING_DEGREES: f32 = 60.0;
    /// Multiplier on aerodynamic drag while active: a 75 percent reduction.
    pub const AERO_DRAG_SCALE: f32 = 0.25;
    /// Tire steering-drag coefficient while active (normal tires use 0.15).
    pub const SLIP_DRAG: f32 = 1.0;
}

/// Passive recharge: the meter fills once the speed is strictly above `min_speed`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Recharge {
    /// Speed in m/s.
    pub min_speed: f32,
    /// Simulation seconds to fill an empty meter.
    pub seconds: f32,
}

/// Drift recharge: fills while drifting faster than `min_speed` at more than `min_slip` radians.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DriftRecharge {
    /// Speed in m/s.
    pub min_speed: f32,
    /// Absolute slip angle in radians.
    pub min_slip: f32,
    /// Effectiveness relative to vanilla (1 is vanilla).
    pub scale: f32,
}

/// Simulation-time multiplier while active, and the rates it moves at.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Dilation {
    /// The multiplier while active (0.25 is a quarter speed).
    pub target: f32,
    /// Change per real second towards `target` on entry.
    pub enter_rate: f32,
    /// Change per real second back to normal speed on exit.
    pub exit_rate: f32,
}

impl Default for Dilation {
    fn default() -> Self {
        Self { target: vanilla::DILATION_TARGET, enter_rate: vanilla::ENTER_RATE, exit_rate: vanilla::EXIT_RATE }
    }
}

/// The vehicle effects while active, applied by the physics step.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PhysicsModifiers {
    /// Collision mass relative to the car's mass.
    pub car_mass_scale: f32,
    /// Gravity scale for the downforce. 1 adds none; the extra downforce is `(gravity_scale - 1)` g.
    pub gravity_scale: f32,
    /// Front tire grip gain per unit of absolute steering input.
    pub front_grip_gain: f32,
    /// Steering angle at full lock, in degrees.
    pub max_steering_degrees: f32,
    /// Multiplier on aerodynamic drag.
    pub aero_drag_scale: f32,
    /// Tire steering-drag coefficient (0.15 normal, 1 at full effect).
    pub slip_drag: f32,
}

impl Default for PhysicsModifiers {
    fn default() -> Self {
        Self {
            car_mass_scale: vanilla::CAR_MASS_SCALE,
            gravity_scale: vanilla::GRAVITY_SCALE,
            front_grip_gain: vanilla::FRONT_GRIP_GAIN,
            max_steering_degrees: vanilla::MAX_STEERING_DEGREES,
            aero_drag_scale: vanilla::AERO_DRAG_SCALE,
            slip_drag: vanilla::SLIP_DRAG,
        }
    }
}

/// Every Speedbreaker tunable, in the game's own units (metres per second, seconds, radians).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SpeedbreakerTuning {
    /// Speed (m/s) to activate; an active breaker turns off below it.
    pub min_speed: f32,
    /// Seconds of a full charge while active; `None` never drains.
    pub max_duration: Option<f32>,
    /// Passive recharge, or `None` when disabled.
    pub passive: Option<Recharge>,
    pub drift: DriftRecharge,
    pub dilation: Dilation,
    pub physics: PhysicsModifiers,
}

impl Default for SpeedbreakerTuning {
    fn default() -> Self {
        Self {
            min_speed: vanilla::MINIMUM_SPEED_MPS,
            max_duration: Some(vanilla::DRAIN_SECONDS),
            passive: Some(Recharge { min_speed: vanilla::RECHARGE_ABOVE_MPS, seconds: vanilla::RECHARGE_SECONDS }),
            drift: DriftRecharge {
                min_speed: vanilla::DRIFT_MIN_SPEED_MPS,
                min_slip: vanilla::DRIFT_MIN_SLIP_RADIANS,
                scale: 1.0,
            },
            dilation: Dilation::default(),
            physics: PhysicsModifiers::default(),
        }
    }
}

/// Values the Extra Options file sets in place of the vanilla ones. They fill in only the keys the SpeedFixer file
/// leaves out: the file's own keys win.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Overrides {
    /// Passive recharge. Set, it stands in for the vanilla recharge, and with no SpeedFixer keys it turns recharge on.
    pub recharge: Option<Recharge>,
    /// Collision mass scale, in place of the vanilla value.
    pub mass_scale: Option<f32>,
}

/// Km/h, the mod's speed unit.
const KPH: Range = Range { min: 0.0, max: 1000.0 };
/// Seconds. The floor keeps a zero duration from dividing by zero.
const SECONDS: Range = Range { min: 0.001, max: 3600.0 };
const DEGREES: Range = Range { min: 0.0, max: 90.0 };
const PERCENT: Range = Range { min: 0.0, max: 500.0 };
const SCALE: Range = Range { min: 0.01, max: 100.0 };

/// The settings file at `path`, read with the rules above, and the Extra Options values as the fallbacks. A missing
/// file is the vanilla tuning, or the Extra Options values where they set one.
pub fn read(path: &Path) -> SpeedbreakerTuning {
    let overrides = extra_options::active().speedbreaker();
    match std::fs::read_to_string(path) {
        Ok(text) => parse_over(&text, &overrides),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => defaults_over(&overrides),
        Err(e) => {
            log::warn!("cannot read {}: {e}", path.display());
            defaults_over(&overrides)
        }
    }
}

/// The tuning from the text of the settings file, with the vanilla values for the keys it leaves out.
#[cfg(test)]
pub fn parse(text: &str) -> SpeedbreakerTuning {
    parse_over(text, &Overrides::default())
}

/// The tuning with no file: the vanilla values, with `overrides` in place of them.
fn defaults_over(overrides: &Overrides) -> SpeedbreakerTuning {
    let d = SpeedbreakerTuning::default();
    SpeedbreakerTuning {
        passive: overrides.recharge.or(d.passive),
        physics: PhysicsModifiers {
            car_mass_scale: overrides.mass_scale.unwrap_or(d.physics.car_mass_scale),
            ..d.physics
        },
        ..d
    }
}

/// The tuning from the text of the settings file, with `overrides` in place of the vanilla values it leaves out.
pub fn parse_over(text: &str, overrides: &Overrides) -> SpeedbreakerTuning {
    let e = entries(text);
    let d = defaults_over(overrides);
    SpeedbreakerTuning {
        min_speed: kph(&e, "Activation", "minCarSpeed", d.min_speed),
        max_duration: max_duration(&e),
        passive: passive(&e, overrides),
        drift: DriftRecharge {
            min_speed: kph(&e, "Recharging", "minDriftSpeed", d.drift.min_speed),
            min_slip: pick(&e, "Recharging", "minSlipAngle", DEGREES, d.drift.min_slip, f32::to_radians),
            scale: pick(&e, "Recharging", "activeScale", SCALE, d.drift.scale, identity),
        },
        dilation: Dilation {
            target: pick(&e, "Physics", "timeScale", Range { min: 1.0, max: 100.0 }, d.dilation.target, recip),
            enter_rate: pick(&e, "Physics", "toDilationScale", SCALE, d.dilation.enter_rate, |s| {
                vanilla::ENTER_RATE / s
            }),
            exit_rate: pick(&e, "Physics", "fromDilationScale", SCALE, d.dilation.exit_rate, |s| {
                vanilla::EXIT_RATE / s
            }),
        },
        physics: PhysicsModifiers {
            car_mass_scale: pick(
                &e,
                "Physics",
                "carMassScale",
                Range { min: 0.0, max: 100.0 },
                d.physics.car_mass_scale,
                identity,
            ),
            gravity_scale: pick(
                &e,
                "Physics",
                "gravityScale",
                Range { min: 1.0, max: 10.0 },
                d.physics.gravity_scale,
                identity,
            ),
            front_grip_gain: pick(&e, "Physics", "frontFrictionBoost", PERCENT, d.physics.front_grip_gain, percent),
            max_steering_degrees: pick(
                &e,
                "Physics",
                "maxSteeringAngle",
                DEGREES,
                d.physics.max_steering_degrees,
                identity,
            ),
            aero_drag_scale: pick(
                &e,
                "Physics",
                "aeroDragReduction",
                Range { min: 0.0, max: 100.0 },
                d.physics.aero_drag_scale,
                |p| 1.0 - percent(p),
            ),
            slip_drag: pick(
                &e,
                "Physics",
                "slipGripReduction",
                Range { min: 15.0, max: 100.0 },
                d.physics.slip_drag,
                percent,
            ),
        },
    }
}

/// The mod's rule: an absent `maxDuration` is unlimited. A malformed one is the vanilla duration.
fn max_duration(e: &[Entry]) -> Option<f32> {
    match read_number(e, "Activation", "maxDuration", SECONDS) {
        Read::Absent => None,
        read => {
            note("Activation", "maxDuration", read);
            Some(read.value().unwrap_or(vanilla::DRAIN_SECONDS))
        }
    }
}

/// The mod's rule: passive recharge is disabled when both of its keys are absent, unless the Extra Options set it.
fn passive(e: &[Entry], overrides: &Overrides) -> Option<Recharge> {
    let speed = read_number(e, "Recharging", "minCarSpeed", KPH);
    let seconds = read_number(e, "Recharging", "passiveTime", SECONDS);
    if speed == Read::Absent && seconds == Read::Absent {
        return overrides.recharge;
    }
    note("Recharging", "minCarSpeed", speed);
    note("Recharging", "passiveTime", seconds);
    let base = overrides
        .recharge
        .unwrap_or(Recharge { min_speed: vanilla::RECHARGE_ABOVE_MPS, seconds: vanilla::RECHARGE_SECONDS });
    Some(Recharge {
        min_speed: speed.value().map_or(base.min_speed, |kph| kph / KPH_PER_MPS),
        seconds: seconds.value().unwrap_or(base.seconds),
    })
}

/// A km/h key, as m/s.
fn kph(e: &[Entry], section: &str, key: &str, default_mps: f32) -> f32 {
    pick(e, section, key, KPH, default_mps, |v| v / KPH_PER_MPS)
}

/// A key read through `range` and converted, or `default` when absent or malformed.
fn pick(e: &[Entry], section: &str, key: &str, range: Range, default: f32, convert: fn(f32) -> f32) -> f32 {
    let read = read_number(e, section, key, range);
    note(section, key, read);
    read.value().map_or(default, convert)
}

/// Logs the lines the file got wrong. Both cases fall back or clamp, so the game still starts.
fn note(section: &str, key: &str, read: Read) {
    match read {
        Read::Clamped(v) => log::warn!("speedbreaker: {section}.{key} is out of range, using {v}"),
        Read::Malformed => log::warn!("speedbreaker: {section}.{key} is not a number, using the vanilla value"),
        Read::Absent | Read::Value(_) => {}
    }
}

fn identity(v: f32) -> f32 {
    v
}

fn recip(v: f32) -> f32 {
    1.0 / v
}

/// A percentage, as a fraction.
fn percent(v: f32) -> f32 {
    v / 100.0
}

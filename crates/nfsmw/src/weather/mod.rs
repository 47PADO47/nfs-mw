//! Rain and wind: the rewrite's weather model. Spec: `docs/specs/weather.md`.
//!
//! The model is pure. Callers pass the frame time, the zone samples and the base roll; nothing here
//! reads the game or the GPU. Values the spec has not measured yet are parameters, not constants.

pub mod drops;
pub mod rain;
#[cfg(test)]
mod tests;

/// A chance below this forces the rain off (the script hook's threshold).
pub const DRY_THRESHOLD: f32 = 0.01;
/// Rate of the rain ease, from the spec's constants.
pub const RAIN_EASE: f32 = 1.0;
/// Rate of the wind-like ease, from the spec's constants.
pub const WIND_EASE: f32 = 10.0;
/// Heading drift in degrees per second.
pub const HEADING_RATE: f32 = 45.0;
/// The heading wraps to zero once it passes this many degrees.
pub const HEADING_WRAP: f32 = 360.0;
/// Frame time used when the frame time is zero.
pub const FALLBACK_DT: f32 = 0.001;

/// The script hook's override. The spec found only the forced-dry value reachable (see `set_chance_of_rain`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Override {
    ForcedDry,
}

/// One zone map entry at a position: the value the base roll is compared with, and the value returned
/// when the roll passes. The spec reads these from two fields; their names are not known yet.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ZoneSample {
    pub compare: f32,
    pub value: f32,
}

/// The weather state that the per-frame update eases.
#[derive(Debug, Clone, PartialEq)]
pub struct Weather {
    override_: Option<Override>,
    /// Eased rain intensity, 0 to 1.
    pub rain: f32,
    /// Eased wind-like value, 0 to 1. Its floor is set at construction.
    pub wind: f32,
    /// Heading in degrees, `0..HEADING_WRAP`.
    pub heading: f32,
    wind_floor: f32,
}

impl Weather {
    /// A dry track with no override. `wind_floor` is the value the wind eases to when its target is zero.
    pub fn new(wind_floor: f32) -> Self {
        Self { override_: None, rain: 0.0, wind: wind_floor, heading: 0.0, wind_floor }
    }

    /// The script hook `SetChanceOfRain`. The chance is clamped to 0..=1. A chance below
    /// `DRY_THRESHOLD` forces the rain off. Any other chance clears the override, so the normal
    /// roll is used. The spec's decompile does not store the chance itself.
    pub fn set_chance_of_rain(&mut self, chance: f32) {
        let chance = chance.clamp(0.0, 1.0);
        self.override_ = (chance < DRY_THRESHOLD).then_some(Override::ForcedDry);
    }

    /// The override in effect, if any.
    pub fn override_(&self) -> Option<Override> {
        self.override_
    }

    /// The rain value at one place (`SampleRainIntensity`). `gate` is the game's flag that squares the
    /// base roll. `zone` is the zone map's sample at the place, or `None` when the map has no entry.
    pub fn sample_rain(&self, base_roll: f32, gate: bool, zone: Option<ZoneSample>) -> f32 {
        if !gate && self.override_ == Some(Override::ForcedDry) {
            return 0.0;
        }
        let roll = if gate { base_roll * base_roll } else { base_roll };
        match zone {
            Some(z) if roll <= z.compare => z.value,
            _ => 0.0,
        }
    }

    /// The per-frame update: ease the rain and the wind toward their targets, then drift the heading.
    pub fn step(&mut self, dt: f32, rain_target: f32, wind_target: f32) {
        let dt = if dt == 0.0 { FALLBACK_DT } else { dt };
        self.rain = ease(self.rain, rain_target, dt, RAIN_EASE);
        if self.rain > 1.0 {
            self.rain = 1.0;
        }
        if rain_target == 0.0 && self.rain < DRY_THRESHOLD {
            self.rain = 0.0;
        }

        self.wind = ease(self.wind, wind_target, dt, WIND_EASE);
        if self.wind > 1.0 {
            self.wind = 1.0;
        }
        if wind_target == 0.0 && self.wind < self.wind_floor + DRY_THRESHOLD {
            self.wind = self.wind_floor;
        }

        self.heading += HEADING_RATE * dt;
        if self.heading > HEADING_WRAP {
            self.heading = 0.0;
        }
    }
}

/// The base roll from the time-like input in seconds (spec: the base roll section). The input is whole seconds, then
/// integer division by 360 gives a 16-bit angle index, and the roll is `1 - |cos(angle)|` clamped at 0.
/// For every input the index is 0 or 1, so the roll is effectively 0: the zone map alone decides the rain.
pub fn base_roll(seconds: f32) -> f32 {
    let whole = seconds.round() as i64;
    let index = ((whole / 360) as u16) as f32;
    let cos = (index * std::f32::consts::TAU / 65536.0).cos();
    (1.0 - cos.abs()).max(0.0)
}

/// One first-order step toward `target`: the lag is `1 / rate` seconds scaled by the frame time.
fn ease(current: f32, target: f32, dt: f32, rate: f32) -> f32 {
    current + (target - current) * dt / rate
}

//! Configurable race staging and launch state. The caller supplies game-specific constants.

#[cfg(test)]
mod tests;

/// Launch rules supplied by the game, with no default activation.
#[derive(Clone, Copy, Debug)]
pub struct LaunchConfig {
    pub band_fraction: f32,
    /// RPM below redline at the top of the launch band.
    pub redline_margin: f32,
    pub duration: f32,
    /// Body speed (m/s) strictly above which launch assistance stops.
    pub speed_cutoff: f32,
    pub traction_gain: f32,
    /// Multiplier applied to all four tires while staging.
    pub staging_traction_scale: f32,
    pub cancel_on_shift: bool,
}

/// Previous completed engine observations and current vehicle context.
pub struct LaunchInput {
    pub dt: f32,
    pub first_gear: bool,
    pub rpm: f32,
    pub idle: f32,
    pub redline: f32,
    pub peak_torque_rpm: f32,
    pub speed: f32,
    pub shifting: bool,
}

#[derive(Clone, Debug, Default)]
pub struct LaunchState {
    config: Option<LaunchConfig>,
    staging: bool,
    remaining: f32,
    strength: f32,
}

impl LaunchState {
    pub fn configure(&mut self, config: Option<LaunchConfig>) {
        self.config = config;
        self.reset();
    }

    pub fn reset(&mut self) {
        self.staging = false;
        self.remaining = 0.0;
        self.strength = 0.0;
    }

    pub fn set_staging(&mut self, staging: bool) {
        self.staging = staging && self.config.is_some();
    }

    pub fn staging(&self) -> bool {
        self.staging
    }

    /// Triggers the release using the most recently staged strength.
    pub fn launch(&mut self) -> bool {
        let Some(config) = self.config else { return false };
        if self.remaining > 0.0 || self.strength <= 0.0 || config.duration <= 0.0 {
            return false;
        }
        self.remaining = config.duration;
        true
    }

    pub fn strength(&self) -> f32 {
        if self.staging || self.remaining <= 0.0 {
            return 0.0;
        }
        self.strength
    }

    pub fn traction_addition(&self) -> f32 {
        self.config.map_or(0.0, |config| self.strength() * config.traction_gain)
    }

    pub fn staging_traction_scale(&self) -> f32 {
        if !self.staging {
            return 1.0;
        }
        self.config.map_or(1.0, |config| config.staging_traction_scale)
    }

    pub fn update(&mut self, input: &LaunchInput) {
        let Some(config) = self.config else { return };
        if self.staging {
            if self.remaining > 0.0 {
                return;
            }
            self.strength = 0.0;
            if !input.first_gear {
                return;
            }
            let width = (input.redline - input.idle) * config.band_fraction;
            let lower = (input.peak_torque_rpm + width).min(input.redline - config.redline_margin) - width;
            let offset = input.rpm - lower;
            if width <= 0.0 || lower <= 0.0 || offset <= 0.0 || offset >= width {
                return;
            }
            self.strength = 0.5 * (1.0 + offset / width);
            return;
        }
        if self.remaining <= 0.0 {
            return;
        }
        if input.speed > config.speed_cutoff || (input.shifting && config.cancel_on_shift) {
            self.remaining = 0.0;
            self.strength = 0.0;
            return;
        }
        if input.shifting {
            return;
        }
        self.remaining = (self.remaining - input.dt).max(0.0);
        if self.remaining == 0.0 {
            self.strength = 0.0;
        }
    }
}

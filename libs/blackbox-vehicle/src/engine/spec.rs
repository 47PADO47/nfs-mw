use crate::math::{FT_LB_TO_NM, rpm_to_rad, table};

/// Engine parameters (the attribute class `engine`). Torque is in ft*lb, speeds in rpm and mph.
#[derive(Clone, Debug)]
pub struct EngineSpec {
    /// Torque samples in ft*lb, evenly spaced from `idle` to `max_rpm`. Fewer than two entries give no torque.
    pub torque: Vec<f32>,
    pub idle: f32,
    /// Rev limiter and last usable curve point.
    pub red_line: f32,
    /// rpm of the last torque sample.
    pub max_rpm: f32,
    /// Flywheel mass as given in the data (unitless); see [`EngineSpec::inertia`].
    pub flywheel_mass: f32,
    /// Closed-throttle drag as a share of the torque: one value, or a curve over `idle..max_rpm`.
    pub engine_braking: Vec<f32>,
    /// `[governor start speed, ramp width]` in mph; 0 disables the speed limiter.
    pub speed_limiter: [f32; 2],
}

impl EngineSpec {
    /// Torque in ft*lb at `rpm`, clamped to `idle..red_line`.
    pub fn torque_ftlb(&self, rpm: f32) -> f32 {
        if self.torque.len() <= 1 {
            return 0.0;
        }
        let rpm = rpm.clamp(self.idle, self.red_line.max(self.idle));
        table(&self.torque, self.idle, self.max_rpm, rpm)
    }

    /// Torque in N m at `rpm`, before induction and nitrous.
    pub fn torque_nm(&self, rpm: f32) -> f32 {
        self.torque_ftlb(rpm) * FT_LB_TO_NM
    }

    /// Closed-throttle drag torque (negative) for the full engine `torque` at `rpm`.
    pub fn braking_torque(&self, torque: f32, rpm: f32) -> f32 {
        match self.engine_braking.as_slice() {
            [] => 0.0,
            [one] => -torque * one,
            curve => {
                let rpm = rpm.clamp(self.idle, self.red_line.max(self.idle));
                -torque * table(curve, self.idle, self.max_rpm, rpm).clamp(0.0, 1.0)
            }
        }
    }

    /// Rotational inertia used to turn torque into angular acceleration (kg m^2, effectively).
    pub fn inertia(&self, in_neutral: bool) -> f32 {
        (self.flywheel_mass * 0.025 + 0.25) * if in_neutral { 0.35 } else { 1.0 }
    }

    pub fn idle_rad(&self) -> f32 {
        rpm_to_rad(self.idle)
    }

    pub fn red_line_rad(&self) -> f32 {
        rpm_to_rad(self.red_line)
    }

    /// Horse power at `rpm` for an engine torque in N m: `T * 0.7376 * rpm / 5252`.
    pub fn horse_power(&self, torque_nm: f32, throttle: f32, rpm: f32) -> f32 {
        torque_nm * throttle * 0.7376 * rpm / crate::math::HP_RPM_CONSTANT
    }
}

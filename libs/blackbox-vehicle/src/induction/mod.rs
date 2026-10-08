//! Forced induction (turbo or supercharger): spool state, boost curve and gauge. Spec section 5 of
//! `docs/specs/vehicle-input-induction-brakes.md`.

use crate::math::ramp;

/// Induction parameters (the attribute class `induction`). All zero means naturally aspirated.
#[derive(Clone, Copy, Debug, Default)]
pub struct InductionSpec {
    /// Boost (torque fraction) at the start of the boosting range.
    pub low_boost: f32,
    /// Boost (torque fraction) at the red line.
    pub high_boost: f32,
    /// Normalised rpm (0..1 over idle..red line) where a turbo starts to work; 0 = supercharger.
    pub spool: f32,
    /// Seconds to spool from 0 to 1 and back.
    pub spool_time_up: f32,
    pub spool_time_down: f32,
    /// Negative torque fraction below the boost threshold.
    pub vacuum: f32,
    /// Gauge value at full boost (display only).
    pub psi: f32,
}

/// What kind of induction a spec describes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum InductionKind {
    None,
    Turbo,
    Supercharger,
}

impl InductionSpec {
    pub fn kind(&self) -> InductionKind {
        if self.high_boost > 0.0 || self.low_boost > 0.0 {
            if self.spool > 0.0 { InductionKind::Turbo } else { InductionKind::Supercharger }
        } else {
            InductionKind::None
        }
    }

    /// The boost with the spool at 1 for a given rpm: used to find the peak torque.
    pub fn full_boost(&self, rpm: f32, idle: f32, red_line: f32) -> f32 {
        if self.kind() == InductionKind::None {
            return 0.0;
        }
        let spool_rpm = self.spool * (red_line - idle) + idle;
        if rpm >= spool_rpm {
            let r = ramp(rpm, spool_rpm, red_line);
            r * self.high_boost + (1.0 - r) * self.low_boost
        } else if self.vacuum < 0.0 {
            ramp(rpm, idle, spool_rpm) * self.vacuum
        } else {
            0.0
        }
    }
}

/// Inputs of one induction update.
#[derive(Clone, Copy, Debug)]
pub struct InductionInput {
    pub dt: f32,
    pub throttle: f32,
    pub rpm: f32,
    pub idle: f32,
    pub red_line: f32,
    pub shifting: bool,
    /// Tuning slider in [-1, 1].
    pub tuning: f32,
}

/// Induction state: spool (0..1), the resulting torque boost and the gauge.
#[derive(Clone, Copy, Debug, Default)]
pub struct Induction {
    pub spool: f32,
    /// Torque multiplier is `1 + boost`.
    pub boost: f32,
    pub psi: f32,
}

impl Induction {
    /// Advances the spool and recomputes the boost for this step.
    pub fn update(&mut self, spec: &InductionSpec, i: &InductionInput) {
        let kind = spec.kind();
        if kind == InductionKind::None {
            *self = Self::default();
            return;
        }
        let t = i.tuning.clamp(-1.0, 1.0);
        let mut spool_n = spec.spool;
        if spec.spool > 0.0 && t != 0.0 {
            let range = if t < 0.0 { spec.spool * 0.25 } else { (1.0 - spec.spool) * 0.25 };
            spool_n = spec.spool + range * t;
        }
        let spool_rpm = spool_n * (i.red_line - i.idle) + i.idle;
        let low = spec.low_boost - spec.low_boost * t * 0.25;
        let high = spec.high_boost + spec.high_boost * t * 0.25;

        let mut desired = ramp(i.throttle, 0.0, 0.5);
        if i.shifting || (kind == InductionKind::Turbo && i.rpm < spool_rpm) {
            desired = 0.0;
        }
        let time = if desired > self.spool { spec.spool_time_up } else { spec.spool_time_down };
        if time <= f32::EPSILON {
            self.spool = desired;
        } else {
            let step = i.dt / time;
            self.spool =
                if desired > self.spool { (self.spool + step).min(desired) } else { (self.spool - step).max(desired) };
        }
        self.spool = self.spool.clamp(0.0, 1.0);

        let peak = high.max(low);
        let (boost, target_psi);
        if i.rpm >= spool_rpm {
            let r = ramp(i.rpm, spool_rpm, i.red_line);
            boost = r * high + (1.0 - r) * low;
            target_psi = self.spool * spec.psi * ramp(boost, 0.0, peak);
        } else if spec.vacuum < 0.0 {
            let d = ramp(i.rpm, i.idle, spool_rpm);
            boost = d * spec.vacuum;
            target_psi = d * -spec.psi * ramp(-boost, 0.0, peak);
        } else {
            boost = 0.0;
            target_psi = 0.0;
        }
        self.boost = boost * self.spool;
        let max_step = 20.0 * i.dt;
        self.psi += (target_psi - self.psi).clamp(-max_step, max_step);
    }
}

#[cfg(test)]
mod tests;

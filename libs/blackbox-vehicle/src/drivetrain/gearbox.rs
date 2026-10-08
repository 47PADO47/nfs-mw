use super::spec::{GEAR_FIRST, TransmissionSpec};
use crate::engine::EngineSpec;
use crate::math::lerp;

/// Up and down shift rpm per gear id (index = gear id). Entries that do not apply are 0.
#[derive(Clone, Debug, Default)]
pub struct ShiftPoints {
    pub up: Vec<f32>,
    pub down: Vec<f32>,
}

/// What the automatic box wants to do.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ShiftPotential {
    None,
    Up,
    Down,
}

impl ShiftPoints {
    /// Computes the shift points from the naturally aspirated torque curve: for each gear, the first rpm
    /// (searched upward from the middle of the range in 50 rpm steps) where the wheel torque after the
    /// shift would exceed the torque before it; the red line minus 100 if there is none.
    pub fn compute(engine: &EngineSpec, trans: &TransmissionSpec) -> Self {
        let n = trans.gear_ratio.len();
        let mut points = Self { up: vec![0.0; n], down: vec![0.0; n] };
        if n <= GEAR_FIRST + 1 || n > 10 {
            return points;
        }
        let top = trans.top_gear();
        let red = engine.red_line;
        for j in GEAR_FIRST..top {
            let (g1, g2) = (trans.ratio(j), trans.ratio(j + 1));
            if g1 < 1e-6 {
                continue;
            }
            let k = g2 / g1;
            let mut rpm = (red + engine.idle) * 0.5;
            while rpm < red {
                let cur = engine.torque_ftlb(rpm);
                let next = engine.torque_ftlb(rpm * k) * k;
                if next > cur {
                    break;
                }
                rpm += 50.0;
            }
            points.up[j] = if rpm >= red { red - 100.0 } else { rpm };
            points.down[j + 1] = points.up[j] * k;
        }
        points.up[top] = red;
        points
    }

    /// The automatic box wish at `rpm` (the rpm the engine would have if locked to the wheels) in `gear`.
    pub fn potential(
        &self,
        engine: &EngineSpec,
        trans: &TransmissionSpec,
        gear: usize,
        rpm: f32,
        throttle: f32,
    ) -> ShiftPotential {
        if gear >= self.up.len() || gear < GEAR_FIRST {
            return ShiftPotential::None;
        }
        let up = self.up[gear];
        let mut down = self.down[gear];
        if gear > GEAR_FIRST && trans.ratio(gear - 1) > 0.0 {
            let lower_up = self.up[gear - 1] * trans.ratio(gear) / trans.ratio(gear - 1) - 200.0;
            let coast = lerp(engine.idle, down, 0.65);
            down = lerp(coast, down, throttle).min(lower_up);
        }
        if rpm >= up && gear < trans.top_gear() {
            ShiftPotential::Up
        } else if rpm <= down && gear > GEAR_FIRST {
            ShiftPotential::Down
        } else {
            ShiftPotential::None
        }
    }

    /// The gear to drop to: step down while the predicted rpm in that gear is still below its own
    /// downshift point.
    pub fn downshift_target(&self, trans: &TransmissionSpec, gear: usize, rpm: f32) -> usize {
        let mut new = gear.saturating_sub(1).max(GEAR_FIRST);
        while new > GEAR_FIRST {
            let predicted = rpm * trans.ratio(gear) / trans.ratio(new).max(1e-6);
            if predicted < self.down[new] {
                new -= 1;
            } else {
                break;
            }
        }
        new
    }
}

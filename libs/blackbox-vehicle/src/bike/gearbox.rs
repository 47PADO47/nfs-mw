//! The automatic gearbox: the change points generated from the torque curve and the gear ratios, and the decision that compares
//! the road speed with them (`docs/specs/ride6-gearbox-auto.md`, sections 1 and 2).

use std::f32::consts::TAU;

use super::curve::curve;
use super::spec::{EngineSpec, TransmissionSpec};

/// The change points of every gear, as shares of the rev limit.
#[derive(Clone, Debug, PartialEq)]
pub struct Gearbox {
    /// `up[n]`: the engine speed in gear `n` (0 for first) at which the next gear gives more force at the wheel.
    up: Vec<f32>,
    /// `down[n]`: the engine speed in gear `n` below which the gear before it gives more.
    down: Vec<f32>,
}

/// The engine speed (rpm) at which the normalised torque curve peaks.
pub fn peak_torque_rpm(engine: &EngineSpec) -> f32 {
    engine
        .torque_curve
        .iter()
        .fold((0.0, f32::MIN), |best, &(rpm, share)| if share > best.1 { (rpm, share) } else { best })
        .0
}

impl Gearbox {
    /// Generate the points: for each gear, sweep the engine speed from the peak of the torque curve to the rev limit and take the
    /// first speed where the next gear wins (up) and the last speed where it loses (down), the tolerances scaling the torque.
    pub fn generate(engine: &EngineSpec, transmission: &TransmissionSpec) -> Self {
        let gears = &transmission.gears;
        let count = gears.len();
        let (mut up, mut down) = (vec![0.0; count], vec![0.0; count]);
        let max = engine.max_rpm.max(1.0);
        let step = ((engine.max_rpm - engine.idle_rpm) * 0.01).max(1.0);
        let start = peak_torque_rpm(engine);
        let tolerance = |list: &[f32], index: usize| list.get(index).copied().unwrap_or(1.0);
        for n in 0..count.saturating_sub(1) {
            let ratio = gears[n + 1] / gears[n].max(1e-6);
            up[n] = 1.0;
            down[n + 1] = 0.0;
            let mut w = start;
            while w < engine.max_rpm {
                let now = curve(&engine.torque_curve, w);
                let next_rpm = w * ratio;
                let after = curve(&engine.torque_curve, next_rpm) * ratio;
                if up[n] == 1.0 && now * tolerance(&transmission.up_tolerance, n) < after {
                    up[n] = w / max;
                }
                if down[n + 1] == 0.0 || after < now * tolerance(&transmission.down_tolerance, n) {
                    down[n + 1] = next_rpm / max;
                }
                w += step;
            }
        }
        Self { up, down }
    }

    pub fn up_point(&self, gear: usize) -> f32 {
        self.up.get(gear).copied().unwrap_or(0.0)
    }

    pub fn down_point(&self, gear: usize) -> f32 {
        self.down.get(gear).copied().unwrap_or(0.0)
    }

    /// The road speed (m/s) of a wheel of effective radius `radius` when the engine turns at `rpm` in gear index `gear`.
    pub fn speed(transmission: &TransmissionSpec, gear: usize, rpm: f32, radius: f32) -> f32 {
        let ratio = transmission.gears.get(gear).copied().unwrap_or(0.0) * transmission.final_ratio;
        if ratio <= 0.0 {
            return 0.0;
        }
        radius.max(1e-4) * rpm * TAU / 60.0 / ratio
    }

    /// Whether the box changes up from gear index `gear` at road speed `speed`. `hold_up` is the factor that lean adds to the
    /// engine speed of the change.
    pub fn wants_up(
        &self,
        engine: &EngineSpec,
        transmission: &TransmissionSpec,
        gear: usize,
        speed: f32,
        radius: f32,
        hold_up: f32,
    ) -> bool {
        if gear + 1 >= transmission.gears.len() {
            return false;
        }
        let a = self.up_point(gear);
        let held = a + (1.0 - a) * hold_up;
        Self::speed(transmission, gear, held * engine.max_rpm, radius) < speed
    }

    /// Whether the box changes down from gear index `gear`. `hold_down` (0 to 1) pushes the change point towards the idle speed.
    pub fn wants_down(
        &self,
        engine: &EngineSpec,
        transmission: &TransmissionSpec,
        gear: usize,
        speed: f32,
        radius: f32,
        hold_down: f32,
    ) -> bool {
        if gear == 0 {
            return false;
        }
        let c = self.down_point(gear);
        let max = engine.max_rpm;
        let rpm = match hold_down >= 0.0 {
            true => max * c + (engine.idle_rpm - max * c) * hold_down,
            false => ((c - 1.0) * hold_down + c) * max,
        };
        Self::speed(transmission, gear - 1, rpm, radius) > speed
    }

    /// The gear index a bike already rolling at `speed` is in: the lowest whose change-up speed it has not passed.
    pub fn gear_for_speed(
        &self,
        engine: &EngineSpec,
        transmission: &TransmissionSpec,
        speed: f32,
        radius: f32,
    ) -> usize {
        let mut gear = 0;
        while self.wants_up(engine, transmission, gear, speed, radius, 0.0) {
            gear += 1;
        }
        gear
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bike::BikeSpec;

    /// The KTM numbers of the spec: ratios 2.0 to 0.954, tolerances up 1 and down 0.4, 0.9, 1, 1, 1, idle 2300, limit 11000,
    /// with a rising-then-falling torque curve.
    fn ktm_like() -> (EngineSpec, TransmissionSpec) {
        let mut spec = BikeSpec::example();
        spec.engine.torque_curve = vec![(1652.0, 0.7375), (7159.0, 1.0), (11000.0, 0.6)];
        spec.engine.idle_rpm = 2300.0;
        spec.engine.max_rpm = 11000.0;
        spec.transmission.gears = vec![2.0, 1.625, 1.333, 1.142, 0.954];
        spec.transmission.up_tolerance = vec![1.0; 5];
        spec.transmission.down_tolerance = vec![0.4, 0.9, 1.0, 1.0, 1.0];
        (spec.engine, spec.transmission)
    }

    #[test]
    fn the_generated_points_sit_below_the_rev_limit_and_the_down_points_below_the_up_points() {
        let (engine, transmission) = ktm_like();
        let box_ = Gearbox::generate(&engine, &transmission);
        for gear in 0..4 {
            let up = box_.up_point(gear);
            assert!(up > 0.65 && up <= 1.0, "gear {gear}: {up}");
            let down = box_.down_point(gear + 1);
            assert!(down > 0.2 && down < up, "gear {gear}: {down} {up}");
        }
        // The top gear has nothing above it, the first nothing below it.
        assert_eq!(box_.up_point(4), 0.0);
        assert_eq!(box_.down_point(0), 0.0);
    }

    #[test]
    fn the_box_changes_up_above_the_point_and_down_below_it() {
        let (engine, transmission) = ktm_like();
        let box_ = Gearbox::generate(&engine, &transmission);
        let radius = 0.32;
        let at = |gear: usize, share: f32| Gearbox::speed(&transmission, gear, share * engine.max_rpm, radius);
        let point = box_.up_point(0);
        assert!(box_.wants_up(&engine, &transmission, 0, at(0, point + 0.02), radius, 0.0));
        assert!(!box_.wants_up(&engine, &transmission, 0, at(0, point - 0.02), radius, 0.0));
        let down = box_.down_point(2);
        assert!(box_.wants_down(&engine, &transmission, 2, at(1, down - 0.02), radius, 0.0));
        assert!(!box_.wants_down(&engine, &transmission, 2, at(1, down + 0.02), radius, 0.0));
        assert!(!box_.wants_down(&engine, &transmission, 0, 0.1, radius, 0.0));
        assert!(!box_.wants_up(&engine, &transmission, 4, 90.0, radius, 0.0));
    }

    #[test]
    fn a_held_down_shift_waits_for_a_lower_speed_and_a_held_up_shift_for_a_higher_one() {
        let (engine, transmission) = ktm_like();
        let box_ = Gearbox::generate(&engine, &transmission);
        let radius = 0.32;
        let speed = Gearbox::speed(&transmission, 1, box_.down_point(2) * engine.max_rpm * 0.99, radius);
        assert!(box_.wants_down(&engine, &transmission, 2, speed, radius, 0.0));
        assert!(!box_.wants_down(&engine, &transmission, 2, speed, radius, 1.0));
        let fast = Gearbox::speed(&transmission, 0, (box_.up_point(0) + 0.01) * engine.max_rpm, radius);
        assert!(box_.wants_up(&engine, &transmission, 0, fast, radius, 0.0));
        assert!(!box_.wants_up(&engine, &transmission, 0, fast, radius, 0.9));
    }

    #[test]
    fn a_bike_already_rolling_starts_in_the_gear_its_speed_calls_for() {
        let (engine, transmission) = ktm_like();
        let box_ = Gearbox::generate(&engine, &transmission);
        assert_eq!(box_.gear_for_speed(&engine, &transmission, 0.0, 0.32), 0);
        let fast = box_.gear_for_speed(&engine, &transmission, 60.0, 0.32);
        assert!(fast >= 3, "{fast}");
    }
}

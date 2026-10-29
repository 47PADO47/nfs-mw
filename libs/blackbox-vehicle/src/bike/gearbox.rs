//! The automatic gearbox: the change points generated from the torque curve and the gear ratios, and the decision that compares
//! the road speed with them (`docs/specs/ride6-gearbox-auto.md`, sections 1 and 2).

use std::f32::consts::TAU;

use super::curve::{curve, ramp};
use super::spec::{EngineSpec, TransmissionSpec};

/// The change points of every gear, as shares of the rev limit.
#[derive(Clone, Debug, PartialEq)]
pub struct Gearbox {
    /// `up[n]`: the engine speed in gear `n` (0 for first) at which the next gear gives more force at the wheel.
    up: Vec<f32>,
    /// `down[n]`: the engine speed in gear `n` below which the gear before it gives more.
    down: Vec<f32>,
}

/// The engine speed (rpm) at which the normalised torque curve peaks: the engine setup of the reference walks the keys of the
/// curve (not the interpolated curve) and keeps the first one whose value is strictly larger than every one before it, starting
/// from 0, so a curve with no positive key gives 0 (`docs/specs/ride6-powertrain-round6.md`, section 4).
pub fn peak_torque_rpm(engine: &EngineSpec) -> f32 {
    engine
        .torque_curve
        .iter()
        .fold((0.0, 0.0), |best, &(rpm, share)| if share > best.1 { (rpm, share) } else { best })
        .0
}

/// The clutch and throttle tuning of the gearbox handler: how a gear change moves the clutch and the throttle, the clutch the
/// handler pulls when the engine would fall below its idle, and the filter of the slipper clutch
/// (`docs/specs/ride6-powertrain-round6.md`). `Default` is the constructor default of the reference, which is what a manifest
/// leaves out.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ClutchControl {
    /// `ClutchTimingOnGearChange`: fractions `(a, b)` of the change. The clutch lever goes from 0 to 1 over `[0, a]` (and the gear
    /// changes at `a`), stays at 1 up to `b` and goes back to 0 over `[b, 1]`.
    pub clutch_timing: [f32; 2],
    /// `ThrottleTimingOnGearChange`: fractions `(c, d)`. The throttle is scaled by 1 at the start, falling to 0 at `c`, 0 up to `d`
    /// and rising to 1 at the end. `(0, 0)` leaves the throttle to the two flags below.
    pub throttle_timing: [f32; 2],
    /// `ReleaseThrottleOnShift`: with no throttle timing, the throttle is shut for the whole change.
    pub release_throttle: bool,
    /// `ModulateThrottleOnUpShift`: with no throttle timing, the throttle follows the clutch (scaled by how far it is back) on a
    /// change up; a change down always does.
    pub modulate_upshift_throttle: bool,
    /// `PressClutchAtMinRPM`: with the throttle shut the clutch is pulled as the road speed falls to the speed at which the engine
    /// turns at its idle speed in the present gear.
    pub press_at_min_rpm: bool,
    /// `MinRPMSpeedScaleRange`: the multiples of that speed at which the idle clutch is fully pulled (low) and fully let go (high).
    pub min_rpm_speed_range: [f32; 2],
    /// `SlipperClutchFilterSpeed`: the speed of the slipper clutch's filter, in the square root of its opening per second; at most
    /// 1e-4 means no filter.
    pub slipper_filter_speed: f32,
    /// The forced downshift under hard braking at speed.
    pub forced_downshift: ForcedDownshift,
}

/// The forced downshift of the road-bike handler (`ForceGearDownChangeBrake*`, `ForceGearDownChangeSpeedInputRange`;
/// `docs/specs/ride6-powertrain-round6.md`, section 8). A brake held above the bottom of `brake_range` builds up a timer; once
/// the timer passes `duration` and the road speed is over the speed range, the change down is made more likely (a negative
/// hold-off) and the change up is held back. `Default` is the constructor default of the reference: a factor of 0, which is off.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ForcedDownshift {
    /// The front lever (0 to 1) at which the strength starts to grow and at which it is full.
    pub brake_range: [f32; 2],
    /// How long the brake has to be held (in seconds weighted by the lever's share of the range) before it acts.
    pub duration: f32,
    pub factor: f32,
    /// The exponent of the lever's share of the range.
    pub linearity: f32,
    /// The road speed range (m/s) over which the effect comes in; a range of zero width is a step.
    pub speed_range: [f32; 2],
}

impl Default for ForcedDownshift {
    fn default() -> Self {
        Self { brake_range: [0.0, 0.0], duration: 0.0, factor: 0.0, linearity: 2.0, speed_range: [0.0, 0.0] }
    }
}

impl ForcedDownshift {
    /// The brake timer after a step: it grows by the lever's share of the range times the step and is 0 when the share is.
    pub fn timer(&self, timer: f32, brake: f32, dt: f32) -> f32 {
        let share = ramp(brake, self.brake_range);
        match share <= 0.0 {
            true => 0.0,
            false => timer + share * dt,
        }
    }

    /// The strength (0 to 1) of the forced downshift: it adds to the hold-off of the change up (largest wins) and subtracts from
    /// that of the change down.
    pub fn strength(&self, timer: f32, brake: f32, speed: f32) -> f32 {
        if self.duration >= timer {
            return 0.0;
        }
        let share = ramp(brake, self.brake_range).powf(self.linearity);
        (ramp(speed, self.speed_range) * share * self.factor).clamp(0.0, 1.0)
    }
}

impl Default for ClutchControl {
    fn default() -> Self {
        Self {
            clutch_timing: [0.0, 0.0],
            throttle_timing: [0.0, 0.0],
            release_throttle: false,
            modulate_upshift_throttle: true,
            press_at_min_rpm: true,
            min_rpm_speed_range: [1.05, 1.1],
            slipper_filter_speed: 0.0,
            forced_downshift: ForcedDownshift::default(),
        }
    }
}

/// The smallest filter speed that filters (the reference compares against it).
const MIN_FILTER_SPEED: f32 = 1e-4;

/// The share `s` (0 to 1) of the ramp a timing pair `(first, second)` gives at the progress `p` of a change: from 1 at the start
/// down to 0 at `first`, then 0 up to `second` and up to 1 at the end. The two ramps are the reference's range translators.
fn timing_ramp(p: f32, [first, second]: [f32; 2]) -> f32 {
    match p < first {
        true => ramp(p, [first, 0.0]),
        false => ramp(p, [second, 1.0]),
    }
}

impl ClutchControl {
    /// The clutch lever (1 pulled) a change asks for at the progress `p` (0 to 1) of the change.
    pub fn shift_clutch(&self, p: f32) -> f32 {
        1.0 - timing_ramp(p, self.clutch_timing)
    }

    /// The factor on the throttle during a change at progress `p`; `up` says whether it is a change up.
    pub fn shift_throttle(&self, p: f32, up: bool) -> f32 {
        if self.throttle_timing != [0.0, 0.0] {
            return timing_ramp(p, self.throttle_timing);
        }
        if self.release_throttle {
            return 0.0;
        }
        match self.modulate_upshift_throttle || !up {
            true => timing_ramp(p, self.clutch_timing),
            false => 1.0,
        }
    }

    /// The idle clutch (1 pulled): `wheel_speed` is the rolling speed of the rear wheel (m/s), `idle_speed` the road speed at
    /// which the engine would turn at its idle speed in the present gear.
    pub fn idle_clutch(&self, wheel_speed: f32, idle_speed: f32) -> f32 {
        if !self.press_at_min_rpm {
            return 0.0;
        }
        let [low, high] = self.min_rpm_speed_range;
        1.0 - ramp(wheel_speed, [idle_speed * low, idle_speed * high])
    }

    /// The slipper clutch's opening after a step: it moves to `target` at once without a filter; with one, the square root of the
    /// gap shrinks by the filter speed each second, so the gap closes in a quadratic ease-out, up or down alike.
    pub fn filter_slipper(&self, current: f32, target: f32, dt: f32) -> f32 {
        if self.slipper_filter_speed <= MIN_FILTER_SPEED {
            return target;
        }
        let gap = current - target;
        let root = gap.abs().sqrt() - self.slipper_filter_speed * dt;
        match (root > 0.0, gap > 0.0) {
            (false, _) => target,
            (true, true) => target + root * root,
            (true, false) => target - root * root,
        }
    }
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

    fn ktm_control() -> ClutchControl {
        ClutchControl {
            clutch_timing: [0.125, 0.2],
            throttle_timing: [0.05, 0.9],
            modulate_upshift_throttle: false,
            ..ClutchControl::default()
        }
    }

    #[test]
    fn the_clutch_goes_out_over_the_first_fraction_stays_out_and_comes_back_over_the_rest() {
        let control = ktm_control();
        assert_eq!(control.shift_clutch(0.0), 0.0);
        assert!((control.shift_clutch(0.0625) - 0.5).abs() < 1e-6);
        assert_eq!(control.shift_clutch(0.125), 1.0);
        assert_eq!(control.shift_clutch(0.2), 1.0);
        assert!((control.shift_clutch(0.6) - 0.5).abs() < 1e-6);
        assert_eq!(control.shift_clutch(1.0), 0.0);
        // The default timing is a plain ramp back over the whole change.
        let plain = ClutchControl::default();
        assert_eq!(plain.shift_clutch(0.0), 1.0);
        assert!((plain.shift_clutch(0.25) - 0.75).abs() < 1e-6);
    }

    #[test]
    fn the_throttle_is_shut_between_its_two_fractions() {
        let control = ktm_control();
        assert_eq!(control.shift_throttle(0.0, true), 1.0);
        assert!((control.shift_throttle(0.025, true) - 0.5).abs() < 1e-6);
        assert_eq!(control.shift_throttle(0.05, false), 0.0);
        assert_eq!(control.shift_throttle(0.5, false), 0.0);
        assert!((control.shift_throttle(0.95, true) - 0.5).abs() < 1e-6);
        assert_eq!(control.shift_throttle(1.0, true), 1.0);
    }

    #[test]
    fn without_a_throttle_timing_the_flags_decide() {
        let mut control = ClutchControl::default();
        // Following the clutch (a plain ramp): half way through the change the throttle is half back.
        assert!((control.shift_throttle(0.5, true) - 0.5).abs() < 1e-6);
        control.modulate_upshift_throttle = false;
        assert_eq!(control.shift_throttle(0.5, true), 1.0);
        assert!((control.shift_throttle(0.5, false) - 0.5).abs() < 1e-6);
        control.release_throttle = true;
        assert_eq!(control.shift_throttle(0.5, true), 0.0);
    }

    #[test]
    fn the_idle_clutch_is_pulled_below_the_idle_speed_and_let_go_above_a_tenth_more() {
        let control = ClutchControl::default();
        assert_eq!(control.idle_clutch(9.0, 10.0), 1.0);
        assert!((control.idle_clutch(10.75, 10.0) - 0.5).abs() < 1e-5);
        assert_eq!(control.idle_clutch(11.5, 10.0), 0.0);
        assert_eq!(ClutchControl { press_at_min_rpm: false, ..control }.idle_clutch(1.0, 10.0), 0.0);
    }

    #[test]
    fn the_slipper_filter_shrinks_the_root_of_the_gap_and_is_off_at_zero() {
        let off = ClutchControl::default();
        assert_eq!(off.filter_slipper(1.0, 0.0, 0.01), 0.0);
        let filter = ClutchControl { slipper_filter_speed: 2.0, ..off };
        // The root of the gap 1 falls by 2 x 0.01 per step: 0.98 squared.
        assert!((filter.filter_slipper(1.0, 0.0, 0.01) - 0.9604).abs() < 1e-6);
        // Rising: the same from below.
        assert!((filter.filter_slipper(0.0, 1.0, 0.01) - (1.0 - 0.9604)).abs() < 1e-6);
        // A small gap closes in one step.
        assert_eq!(filter.filter_slipper(0.0001, 0.0, 0.01), 0.0);
    }

    #[test]
    fn a_held_brake_forces_the_downshift_only_after_its_time_and_above_the_speed() {
        let forced = ForcedDownshift {
            brake_range: [0.5, 1.0],
            duration: 0.3,
            factor: 1.0,
            linearity: 3.0,
            speed_range: [150.0 / 3.6, 150.0 / 3.6],
        };
        // The timer grows with the lever's share of the range and is reset below the range.
        assert_eq!(forced.timer(1.0, 0.4, 0.1), 0.0);
        assert!((forced.timer(0.0, 0.75, 0.1) - 0.05).abs() < 1e-6);
        assert!((forced.timer(0.2, 1.0, 0.1) - 0.3).abs() < 1e-6);
        let fast = 160.0 / 3.6;
        // Not before the duration, not under the speed, then the cube of the share.
        assert_eq!(forced.strength(0.3, 1.0, fast), 0.0);
        assert_eq!(forced.strength(0.31, 1.0, 100.0 / 3.6), 0.0);
        assert_eq!(forced.strength(0.31, 1.0, fast), 1.0);
        assert!((forced.strength(0.31, 0.75, fast) - 0.125).abs() < 1e-6);
        assert_eq!(ForcedDownshift::default().strength(1.0, 1.0, fast), 0.0);
    }

    #[test]
    fn the_peak_torque_speed_is_the_first_key_with_the_largest_value() {
        let (mut engine, _) = ktm_like();
        assert_eq!(peak_torque_rpm(&engine), 7159.0);
        engine.torque_curve = vec![(2000.0, 0.9), (4000.0, 1.0), (6000.0, 1.0)];
        assert_eq!(peak_torque_rpm(&engine), 4000.0);
        engine.torque_curve = vec![(2000.0, 0.0), (4000.0, -1.0)];
        assert_eq!(peak_torque_rpm(&engine), 0.0);
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

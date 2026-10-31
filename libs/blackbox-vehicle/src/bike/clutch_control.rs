//! The clutch and throttle tuning of the gearbox handler and the two rules that act on the gear: the timing of a change, the idle
//! clutch and the slipper filter, the forced downshift under hard braking and the over-limit upshift
//! (`docs/specs/ride6-powertrain-round6.md`, `docs/specs/ride6-powertrain-round7.md`).

use super::curve::ramp;

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
    /// The forced downshift under hard braking.
    pub forced_downshift: ForcedDownshift,
    /// The change up that an engine held near its limit asks for.
    pub over_limit: OverLimit,
}

/// The over-limit upshift (`GearChangeOverLimitEnabled`, `OverLimitMinChangeSpeed`, `OverLimitChangeTime`,
/// `OverLimitChangeTimeThreshold`, `OverLimitRpmPercentage`; `docs/specs/ride6-powertrain-round6.md`, section 12): when the
/// engine has turned over `rpm_share` of its rev limit for `change_time` seconds above `min_speed`, and the last change was more
/// than `threshold` seconds ago, the box changes up whatever the change points say. `Default` is the constructor default.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct OverLimit {
    pub enabled: bool,
    /// The road speed (m/s) above which the timer runs.
    pub min_speed: f32,
    pub change_time: f32,
    pub threshold: f32,
    /// The share of the rev limit the engine has to pass; 0 or less means only the rev limiter itself counts.
    pub rpm_share: f32,
}

impl Default for OverLimit {
    fn default() -> Self {
        Self { enabled: true, min_speed: 10.0 / 3.6, change_time: 1.0, threshold: 0.5, rpm_share: 0.99 }
    }
}

impl OverLimit {
    /// The time spent over the limit after a step: it grows while the engine is over `rpm_share` of its limit (or at the limiter)
    /// above `min_speed` and is 0 otherwise.
    pub fn timer(&self, timer: f32, rpm: f32, max_rpm: f32, speed: f32, dt: f32) -> f32 {
        let over = rpm > max_rpm || (self.rpm_share > 0.0 && self.rpm_share * max_rpm < rpm);
        match over && speed > self.min_speed {
            true => timer + dt,
            false => 0.0,
        }
    }

    /// Whether the box has to change up now, given the time over the limit and the time since the last change.
    pub fn wants_up(&self, timer: f32, since_change: f32) -> bool {
        self.enabled && self.change_time < timer && self.threshold < since_change
    }
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
            over_limit: OverLimit::default(),
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

#[cfg(test)]
mod tests {
    use super::*;

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
    fn an_engine_held_near_its_limit_asks_for_a_change_up_after_its_time() {
        let over =
            OverLimit { enabled: true, min_speed: 60.0 / 3.6, change_time: 0.064, threshold: 1.0, rpm_share: 0.94 };
        let (max, dt) = (11000.0, 0.02);
        // Under the share, or too slow, the timer is 0; over both it grows.
        assert_eq!(over.timer(0.5, 10000.0, max, 30.0, dt), 0.0);
        assert_eq!(over.timer(0.5, 10500.0, max, 10.0, dt), 0.0);
        assert!((over.timer(0.1, 10500.0, max, 30.0, dt) - 0.12).abs() < 1e-6);
        // The rev limiter counts even with a share of 0.
        let limiter = OverLimit { rpm_share: 0.0, ..over };
        assert_eq!(limiter.timer(0.0, 10900.0, max, 30.0, dt), 0.0);
        assert!(limiter.timer(0.0, 11100.0, max, 30.0, dt) > 0.0);
        assert!(over.wants_up(0.1, 1.5));
        assert!(!over.wants_up(0.06, 1.5) && !over.wants_up(0.1, 0.9));
        assert!(!OverLimit { enabled: false, ..over }.wants_up(5.0, 5.0));
    }
}

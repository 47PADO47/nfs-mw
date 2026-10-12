//! High-speed effects as pure curves: how strong the speed streaks, the radial blur, the camera shake and the
//! nitrous field-of-view kick are at a given speed and nitrous level, and how they ease towards that strength.
//!
//! Nothing here draws or reads a camera. The caller feeds in the car's speed (m/s) and nitrous fraction
//! (0..1) each frame and applies the outputs. The thresholds are tuning, not install data: they are chosen
//! so the effects start around the speed where the chase camera's field of view is widening
//! ([`Tuning::default`]), and the caller can replace them.

/// Strength of each effect, every value in `0..=1` except [`Effects::fov_kick_deg`] and [`Effects::shake_m`].
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Effects {
    /// Opacity of the speed streaks drawn over the view.
    pub streaks: f32,
    /// Strength of the radial blur from the screen centre.
    pub blur: f32,
    /// Peak camera displacement, metres, of the shake.
    pub shake_m: f32,
    /// Extra vertical field of view while nitrous burns, degrees.
    pub fov_kick_deg: f32,
}

/// The speed (m/s) ranges and maxima of each effect. A range is `from..to`: no effect below `from`, full
/// strength at `to`. Nitrous adds on top of the speed curves.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Tuning {
    pub streak_from: f32,
    pub streak_to: f32,
    pub blur_from: f32,
    pub blur_to: f32,
    pub shake_from: f32,
    pub shake_to: f32,
    /// Shake displacement at full strength, metres.
    pub shake_max_m: f32,
    /// Field-of-view kick at full nitrous, degrees.
    pub fov_kick_max_deg: f32,
    /// How much of the streaks and blur full nitrous adds (0..1 each).
    pub nitrous_weight: f32,
    /// How fast an effect rises towards its target, per second (the exponential rate).
    pub rise_per_second: f32,
    /// How fast an effect falls back, per second.
    pub fall_per_second: f32,
}

impl Default for Tuning {
    fn default() -> Self {
        Self {
            streak_from: 40.0,
            streak_to: 75.0,
            blur_from: 55.0,
            blur_to: 95.0,
            shake_from: 45.0,
            shake_to: 85.0,
            shake_max_m: 0.04,
            fov_kick_max_deg: 8.0,
            nitrous_weight: 0.4,
            rise_per_second: 6.0,
            fall_per_second: 2.5,
        }
    }
}

/// The linear ramp from 0 at `from` to 1 at `to`, clamped. A `to` at or below `from` is a step at `from`.
pub fn ramp(x: f32, from: f32, to: f32) -> f32 {
    if to <= from {
        return if x >= from { 1.0 } else { 0.0 };
    }
    ((x - from) / (to - from)).clamp(0.0, 1.0)
}

/// The effect strengths the speed and nitrous level call for, without smoothing.
pub fn target(tuning: &Tuning, speed: f32, nitrous: f32) -> Effects {
    let speed = speed.abs();
    let nitrous = nitrous.clamp(0.0, 1.0);
    let streaks = ramp(speed, tuning.streak_from, tuning.streak_to).max(nitrous * tuning.nitrous_weight);
    let blur = (ramp(speed, tuning.blur_from, tuning.blur_to) + nitrous * tuning.nitrous_weight).min(1.0);
    let shake = ramp(speed, tuning.shake_from, tuning.shake_to);
    // The kick follows the nitrous only: the speed already widens the view (`viewer::camera::dynamic_fov`).
    Effects { streaks, blur, shake_m: tuning.shake_max_m * shake, fov_kick_deg: tuning.fov_kick_max_deg * nitrous }
}

/// The effects as they are on screen: they rise and fall towards [`target`] at the tuning's rates, so a
/// gear change or a nitrous press never snaps the view.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SpeedFx {
    tuning: Tuning,
    current: Effects,
}

impl SpeedFx {
    pub fn new(tuning: Tuning) -> Self {
        Self { tuning, current: Effects::default() }
    }

    /// The current strengths.
    pub fn effects(&self) -> Effects {
        self.current
    }

    /// Moves each effect towards its target by the share of the gap that `dt` seconds allow. A non-positive or
    /// non-finite `dt` changes nothing.
    pub fn update(&mut self, speed: f32, nitrous: f32, dt: f32) {
        if !(dt > 0.0 && dt.is_finite()) {
            return;
        }
        let goal = target(&self.tuning, speed, nitrous);
        let now = self.current;
        self.current = Effects {
            streaks: approach(now.streaks, goal.streaks, &self.tuning, dt),
            blur: approach(now.blur, goal.blur, &self.tuning, dt),
            shake_m: approach(now.shake_m, goal.shake_m, &self.tuning, dt),
            fov_kick_deg: approach(now.fov_kick_deg, goal.fov_kick_deg, &self.tuning, dt),
        };
    }
}

/// One step of an exponential approach. Rising uses `rise_per_second`, falling `fall_per_second`.
fn approach(now: f32, goal: f32, tuning: &Tuning, dt: f32) -> f32 {
    let rate = if goal > now { tuning.rise_per_second } else { tuning.fall_per_second };
    let share = 1.0 - (-rate * dt).exp();
    now + (goal - now) * share
}

#[cfg(test)]
mod tests {
    use super::*;

    const T: Tuning = Tuning {
        streak_from: 40.0,
        streak_to: 80.0,
        blur_from: 60.0,
        blur_to: 100.0,
        shake_from: 50.0,
        shake_to: 90.0,
        shake_max_m: 0.1,
        fov_kick_max_deg: 10.0,
        nitrous_weight: 0.5,
        rise_per_second: 4.0,
        fall_per_second: 2.0,
    };

    #[test]
    fn ramp_is_linear_and_clamped() {
        assert_eq!(ramp(0.0, 10.0, 20.0), 0.0);
        assert!((ramp(15.0, 10.0, 20.0) - 0.5).abs() < 1e-6);
        assert_eq!(ramp(99.0, 10.0, 20.0), 1.0);
    }

    #[test]
    fn ramp_with_no_width_is_a_step() {
        assert_eq!(ramp(9.9, 10.0, 10.0), 0.0);
        assert_eq!(ramp(10.0, 10.0, 5.0), 1.0);
    }

    #[test]
    fn nothing_happens_at_rest_without_nitrous() {
        assert_eq!(target(&T, 0.0, 0.0), Effects::default());
    }

    #[test]
    fn speed_curves_start_where_their_ranges_start() {
        assert_eq!(target(&T, 40.0, 0.0).streaks, 0.0);
        assert_eq!(target(&T, 80.0, 0.0).streaks, 1.0);
        assert!((target(&T, 90.0, 0.0).shake_m - 0.1).abs() < 1e-6);
        assert!((target(&T, 99.0, 0.0).blur - ramp(99.0, 60.0, 100.0)).abs() < 1e-6);
    }

    #[test]
    fn reverse_counts_as_speed() {
        assert_eq!(target(&T, -85.0, 0.0), target(&T, 85.0, 0.0));
    }

    #[test]
    fn nitrous_adds_streaks_blur_and_a_fov_kick_at_any_speed() {
        let coasting = target(&T, 10.0, 0.0);
        let boosting = target(&T, 10.0, 1.0);
        assert!((boosting.streaks - 0.5).abs() < 1e-6, "weight 0.5 at full nitrous");
        assert!(boosting.blur > coasting.blur);
        assert!((boosting.fov_kick_deg - 10.0).abs() < 1e-6);
        assert!((target(&T, 10.0, 0.5).fov_kick_deg - 5.0).abs() < 1e-6, "kick is linear in nitrous");
    }

    #[test]
    fn nitrous_is_clamped_to_its_range() {
        assert_eq!(target(&T, 10.0, 5.0), target(&T, 10.0, 1.0));
        assert_eq!(target(&T, 10.0, -1.0), target(&T, 10.0, 0.0));
    }

    #[test]
    fn effects_rise_and_fall_at_their_own_rates() {
        let mut fx = SpeedFx::new(T);
        fx.update(90.0, 0.0, 0.5);
        let risen = fx.effects().streaks;
        assert!(risen > 0.0 && risen < 1.0, "half a second does not reach full strength: {risen}");
        fx.update(0.0, 0.0, 0.5);
        assert!(fx.effects().streaks < risen, "falls back");
    }

    #[test]
    fn long_runs_converge_to_the_target() {
        let mut fx = SpeedFx::new(T);
        for _ in 0..600 {
            fx.update(90.0, 0.0, 1.0 / 60.0);
        }
        let goal = target(&T, 90.0, 0.0);
        assert!((fx.effects().shake_m - goal.shake_m).abs() < 1e-4);
        assert!((fx.effects().streaks - goal.streaks).abs() < 1e-4);
    }

    #[test]
    fn rising_is_faster_than_falling() {
        let mut up = SpeedFx::new(T);
        up.update(90.0, 0.0, 0.2);
        let mut down = SpeedFx::new(T);
        for _ in 0..600 {
            down.update(90.0, 0.0, 1.0 / 60.0);
        }
        let start = down.effects().streaks;
        down.update(0.0, 0.0, 0.2);
        let fallen = start - down.effects().streaks;
        assert!(up.effects().streaks > fallen, "rise at 4/s beats fall at 2/s");
    }

    #[test]
    fn bad_time_steps_change_nothing() {
        let mut fx = SpeedFx::new(T);
        fx.update(90.0, 1.0, 0.0);
        fx.update(90.0, 1.0, f32::NAN);
        fx.update(90.0, 1.0, -1.0);
        assert_eq!(fx.effects(), Effects::default());
    }
}

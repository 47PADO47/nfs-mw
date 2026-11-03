//! The rider's ABS level: a setting from 0 to 1 that scales the anti-lock brake's intensity, switches the rear channel off at
//! the low levels and shrinks the brake relief of the stoppie help (`docs/specs/ride6-tricks.md`, "ABS and a stoppie").
//! The reference's tuning component maps the level through one curve per effect; a bike states those curves in
//! `AbsLevelSpec`, and a bike that states none takes the plain level.

use super::Bike;
use super::curve::curve;
use super::spec::AbsSpec;

/// The levels the setting steps through, from the top: 1, 0.75, 0.5, 0.25, 0.
pub const LEVEL_STEP: f32 = 0.25;

/// How the level reaches the brake. Every curve is `(level, factor)`; an empty one is the default named with it.
#[derive(Clone, Debug, PartialEq)]
pub struct AbsLevelSpec {
    /// The level a bike starts with (the tuning entry's default).
    pub default: f32,
    /// The factor on the front and on the rear ABS intensity. Empty: the level itself.
    pub front: Vec<(f32, f32)>,
    pub rear: Vec<(f32, f32)>,
    /// Whether the rear channel works (on at 0.5 and over). Empty: always on.
    pub rear_enabled: Vec<(f32, f32)>,
    /// The factor on the start of the stoppie help's brake relief range. Empty: 1.
    pub brake_relief: Vec<(f32, f32)>,
}

impl Default for AbsLevelSpec {
    /// The setting's default is the full level (`DefaultValue` 1 of `SETTINGS_ABS`).
    fn default() -> Self {
        Self { default: 1.0, front: Vec::new(), rear: Vec::new(), rear_enabled: Vec::new(), brake_relief: Vec::new() }
    }
}

fn through(points: &[(f32, f32)], level: f32) -> f32 {
    match points.is_empty() {
        true => level,
        false => curve(points, level),
    }
}

impl AbsLevelSpec {
    /// The level clamped to 0..1 and snapped to the setting's steps.
    pub fn snap(level: f32) -> f32 {
        match level.is_finite() {
            true => (level.clamp(0.0, 1.0) / LEVEL_STEP).round() * LEVEL_STEP,
            false => 1.0,
        }
    }

    /// The next level down, and from the lowest back to the highest.
    pub fn cycle(level: f32) -> f32 {
        let next = Self::snap(level) - LEVEL_STEP;
        match next < -1e-6 {
            true => 1.0,
            false => next.max(0.0),
        }
    }

    /// The ABS at `level`: the intensities scaled by the level's factors, the rear off where it is not enabled.
    pub fn apply(&self, base: &AbsSpec, level: f32) -> AbsSpec {
        let rear_on = self.rear_enabled.is_empty() || curve(&self.rear_enabled, level) >= 0.5;
        AbsSpec {
            front_intensity: base.front_intensity * through(&self.front, level),
            rear_intensity: match rear_on {
                true => base.rear_intensity * through(&self.rear, level),
                false => 0.0,
            },
            ..*base
        }
    }

    /// The factor on the start of the stoppie help's brake relief range at `level`.
    pub fn relief(&self, level: f32) -> f32 {
        match self.brake_relief.is_empty() {
            true => 1.0,
            false => curve(&self.brake_relief, level),
        }
    }
}

impl Bike {
    /// The rider's ABS level, 0 (the least) to 1 (the most), in steps of a quarter. It scales the anti-lock brake of both
    /// wheels through the bike's `abs_level` curves and switches the rear channel off at the low levels; the stoppie needs it
    /// low (`docs/specs/ride6-tricks.md`). A new bike starts at the setting's default.
    pub fn set_abs_level(&mut self, level: f32) {
        self.abs_level = AbsLevelSpec::snap(level);
    }

    /// The rider's ABS level now.
    pub fn abs_level(&self) -> f32 {
        self.abs_level
    }

    /// Step the anti-lock with the ABS of the bike at the rider's level (the one place the level reaches the brake).
    pub(super) fn update_antilock(&mut self, speed: f32, normalised: [f32; 2], throttle: f32, dt: f32) {
        let abs = self.spec.abs_level.apply(&self.spec.brakes.abs, self.abs_level);
        self.antilock.update(&abs, speed, normalised[0], normalised[1], dt);
        self.antilock.update_reduction(&abs.reduction, &self.reduction_input(speed, throttle), dt);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bike::BikeSpec;

    /// The curves of the reference's 5-speed supermoto class.
    fn supermoto() -> AbsLevelSpec {
        AbsLevelSpec {
            default: 1.0,
            front: vec![(0.0, 0.1), (0.25, 0.2), (0.5, 0.45), (0.75, 1.0), (1.0, 1.0)],
            rear: Vec::new(),
            rear_enabled: vec![(0.0, 0.0), (0.25, 0.0), (0.5, 1.0), (0.75, 1.0), (1.0, 1.0)],
            brake_relief: vec![(0.0, 1.0), (0.25, 0.75), (0.5, 0.5), (0.75, 0.0), (1.0, 0.0)],
        }
    }

    #[test]
    fn the_level_cycles_down_in_steps_and_wraps_to_the_top() {
        let mut level = 1.0;
        let mut seen = Vec::new();
        for _ in 0..6 {
            seen.push(level);
            level = AbsLevelSpec::cycle(level);
        }
        assert_eq!(seen, vec![1.0, 0.75, 0.5, 0.25, 0.0, 1.0]);
        assert_eq!(AbsLevelSpec::snap(0.6), 0.5);
        assert_eq!(AbsLevelSpec::snap(f32::NAN), 1.0);
        assert_eq!(AbsLevelSpec::snap(7.0), 1.0);
    }

    #[test]
    fn a_level_scales_the_intensities_and_switches_the_rear_off() {
        let base = BikeSpec::example().brakes.abs;
        let spec = supermoto();
        let full = spec.apply(&base, 1.0);
        assert_eq!((full.front_intensity, full.rear_intensity), (base.front_intensity, base.rear_intensity));
        let half = spec.apply(&base, 0.5);
        assert!((half.front_intensity - 0.45).abs() < 1e-6 && (half.rear_intensity - 0.5).abs() < 1e-6);
        let off = spec.apply(&base, 0.0);
        assert!((off.front_intensity - 0.1).abs() < 1e-6 && off.rear_intensity == 0.0);
        assert_eq!(off.front_slip, base.front_slip);
    }

    #[test]
    fn without_curves_the_level_is_the_factor_and_the_rear_stays_on() {
        let base = BikeSpec::example().brakes.abs;
        let plain = AbsLevelSpec::default();
        assert_eq!(plain.default, 1.0);
        let low = plain.apply(&base, 0.25);
        assert_eq!(
            (low.front_intensity, low.rear_intensity),
            (0.25 * base.front_intensity, 0.25 * base.rear_intensity)
        );
        assert_eq!(plain.relief(0.0), 1.0);
        assert_eq!(supermoto().relief(0.75), 0.0);
    }
}

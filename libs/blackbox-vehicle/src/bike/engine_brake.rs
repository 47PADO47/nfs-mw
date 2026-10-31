//! The engine-brake reduction: the share of the closed-throttle drag the engine drops when the rear tyre slides, so a locked or
//! sliding rear wheel is not held back by the engine as well (`docs/specs/ride6-powertrain-round6.md`, section 11). It is the
//! reference's `bUseDeprecatedEngineBrakeReduction` path, which every bike keeps.

use super::brake_reduction::RangeMap;

/// The tunings of the reduction. `Default` is the constructor default of the reference: a factor of 0 (no reduction beyond the
/// part of the friction curve over its limit), and translators that output 1.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct EngineBrakeReduction {
    /// `EngineBrakeReductionFactor`: the weight of the load and slip terms (0 for none).
    pub factor: f32,
    /// `FrictionPeakTorqueLimit`: the friction curve value above which the drag is dropped outright (constructor default 1; no
    /// bike sets it, so only the part of the curve over 1 counts, which the KTM's curve has past its rev limit).
    pub friction_limit: f32,
    /// Of the normalised lean (absolute value), the rear tyre's load over its load at rest, and the rear slip ratio. The lean one is
    /// left at its default of 1 by every bike.
    pub lean: RangeMap,
    pub load: RangeMap,
    pub slip: RangeMap,
}

impl Default for EngineBrakeReduction {
    fn default() -> Self {
        Self {
            factor: 0.0,
            friction_limit: 1.0,
            lean: RangeMap::constant(1.0),
            load: RangeMap::constant(1.0),
            slip: RangeMap::constant(1.0),
        }
    }
}

impl EngineBrakeReduction {
    /// The reduction `r` (0 to 1) of the drag at the friction curve value `friction` (1 at the rev limit): the part over the limit
    /// plus the lean, load and slip terms times the factor. `lean` is the lean as a share of the lean limit, `load` the rear
    /// tyre's load over its load at rest and `slip` the rear wheel's slip ratio ((rim speed - road speed) over the road speed,
    /// negative when the wheel is held back).
    pub fn reduction(&self, friction: f32, lean: f32, load: f32, slip: f32) -> f32 {
        let over = (friction - self.friction_limit).max(0.0);
        (over + self.lean.map(lean.abs()) * self.load.map(load) * self.factor * self.slip.map(slip)).clamp(0.0, 1.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The KTM: factor 1, load 1 to 0 giving 1 to 0.9, slip 0 to -0.33 giving 0 to 1.
    fn ktm() -> EngineBrakeReduction {
        EngineBrakeReduction {
            factor: 1.0,
            load: RangeMap { input: [1.0, 0.0], output: [1.0, 0.9] },
            slip: RangeMap { input: [0.0, -0.33], output: [0.0, 1.0] },
            ..EngineBrakeReduction::default()
        }
    }

    #[test]
    fn the_engine_keeps_its_drag_while_the_rear_grips_and_drops_it_as_the_rear_slides() {
        let r = ktm();
        assert_eq!(r.reduction(0.5, 0.3, 1.0, 0.0), 0.0);
        assert!((r.reduction(0.5, 0.3, 1.0, -0.165) - 0.5).abs() < 1e-6);
        assert_eq!(r.reduction(0.5, 0.3, 1.0, -0.5), 1.0);
        // An unloaded rear (load 0) takes slightly less of the drag away: the load term is 0.9.
        assert!((r.reduction(0.5, 0.0, 0.0, -0.33) - 0.9).abs() < 1e-6);
    }

    #[test]
    fn the_part_of_the_curve_over_its_limit_is_dropped_whatever_the_slip() {
        let r = ktm();
        assert!((r.reduction(1.5, 0.0, 1.0, 0.0) - 0.5).abs() < 1e-6);
        assert_eq!(r.reduction(2.5, 0.0, 1.0, 0.0), 1.0);
    }

    #[test]
    fn the_default_reduces_nothing_below_the_limit() {
        let r = EngineBrakeReduction::default();
        assert_eq!(r.reduction(1.0, 1.0, 0.0, -1.0), 0.0);
    }
}

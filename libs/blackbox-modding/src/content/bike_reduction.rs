//! The brake reduction of the anti-lock element (`[bike.brakes.abs.reduction]`, `docs/specs/ride6-powertrain-round6.md`, section 9).
//! Every translator is `[input low, input high, output at low, output at high]`; all zeros (the default) is a term that is off.

use blackbox_vehicle::bike::{BrakeReduction, RangeMap, WheelReduction as WheelSpec};
use serde::Deserialize;

use super::ContentError;
use super::bike_check::{all_ranged, ranged};

fn map(values: [f32; 4]) -> RangeMap {
    RangeMap { input: [values[0], values[1]], output: [values[2], values[3]] }
}

/// One wheel's tunings: the intensity and the four terms. The lean (a share of the lean limit) and the drift angle (deg) are
/// read by their absolute value; the load is the wheel's load over its load at rest; the throttle is the rider's.
#[derive(Debug, Clone, Copy, Default, PartialEq, Deserialize)]
#[serde(deny_unknown_fields, default)]
pub struct WheelReduction {
    pub intensity: f32,
    pub lean: [f32; 4],
    pub drift: [f32; 4],
    pub load: [f32; 4],
    pub throttle: [f32; 4],
}

impl WheelReduction {
    fn to_spec(self) -> WheelSpec {
        WheelSpec {
            intensity: self.intensity,
            lean: map(self.lean),
            drift: map(self.drift),
            load: map(self.load),
            throttle: map(self.throttle),
        }
    }

    fn check(&self, name: &'static str) -> Result<(), ContentError> {
        ranged(self.intensity, name, 0.0, 10.0)?;
        [self.lean, self.drift, self.load, self.throttle].iter().try_for_each(|m| all_ranged(m, name, -1000.0, 1000.0))
    }
}

/// `[bike.brakes.abs.reduction]`: the speed term (km/h in, a share out) and the two wheels.
#[derive(Debug, Clone, Copy, Default, PartialEq, Deserialize)]
#[serde(deny_unknown_fields, default)]
pub struct Reduction {
    pub speed_kmh: [f32; 4],
    pub front: WheelReduction,
    pub rear: WheelReduction,
}

impl Reduction {
    pub(super) fn to_spec(self) -> BrakeReduction {
        BrakeReduction { speed: map(self.speed_kmh), front: self.front.to_spec(), rear: self.rear.to_spec() }
    }

    pub(super) fn check(&self) -> Result<(), ContentError> {
        all_ranged(&self.speed_kmh, "bike.brakes.abs.reduction.speed_kmh", -1000.0, 1000.0)?;
        self.front.check("bike.brakes.abs.reduction.front")?;
        self.rear.check("bike.brakes.abs.reduction.rear")
    }
}

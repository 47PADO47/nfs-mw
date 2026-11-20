//! Grip level, compound class and car class (parameter-mapping.md sections 4.1 and 4.2).

use super::spec::{CarSpec, Drive};
use crate::constants::{DEFAULT_RHO, REFERENCE_PEAK_FRICTION};
use crate::presets::{CarClass, Compound};

/// The grip level `L` of an axle: its `peak_friction` over the game's reference tyre. 1.0 when the figure is unusable.
pub fn grip_level(peak_friction: f64) -> f64 {
    if !peak_friction.is_finite() || peak_friction <= 0.0 {
        return 1.0;
    }
    peak_friction / REFERENCE_PEAK_FRICTION
}

/// The sliding over peak ratio `rho` of an axle. 0.89 when either figure is unusable.
pub fn sliding_ratio(peak_friction: f64, sliding_friction: f64) -> f64 {
    let usable = |value: f64| value.is_finite() && value > 0.0;
    if !usable(peak_friction) || !usable(sliding_friction) {
        return DEFAULT_RHO;
    }
    sliding_friction / peak_friction
}

/// The compound class the front axle's grip level picks.
pub fn compound_for(level: f64) -> Compound {
    match level {
        l if l < 0.70 => Compound::Economy,
        l if l < 0.95 => Compound::Road,
        l if l < 1.15 => Compound::Sport,
        l if l < 1.40 => Compound::Semislick,
        _ => Compound::Slick,
    }
}

/// The car class from the size of the car and its tyres.
pub fn class_for(spec: &CarSpec) -> CarClass {
    let power_to_mass = spec.max_power_w / spec.mass_kg;
    if spec.tyre_width_m[1] >= 0.285 || power_to_mass >= 250.0 {
        return CarClass::Super;
    }
    if spec.mass_kg >= 1800.0 {
        return CarClass::Heavy;
    }
    if spec.mass_kg < 1250.0 {
        return CarClass::Compact;
    }
    CarClass::Sport
}

/// The front minus rear peak angle of the layout, degrees: a front-driven car is set up to understeer more.
pub fn default_bias(drive: Drive) -> f64 {
    match drive {
        Drive::Front => 3.0,
        Drive::Rear => 2.0,
        Drive::All => 2.5,
    }
}

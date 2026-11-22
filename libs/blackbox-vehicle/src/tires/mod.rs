//! The tire model: wheel spin, slip, load-sensitive lateral force, friction ellipse and surface grip.
//! Spec section 3 of `docs/specs/vehicle-suspension-tires.md`.

mod adopt;
mod angle;
mod coasting;
mod damage;
mod integration;
mod lateral;
mod loaded;
mod model;
mod rolling;
mod scales;
mod simple;
mod spec;
pub mod spike;
mod state;

pub use coasting::CoastingTireResponse;
pub use damage::{BLOWN_TRACTION, TireDamage, TireState};
pub use integration::TireIntegration;
pub use lateral::lateral_force;
pub use model::{
    AMBIENT_TEMPERATURE_C, MAX_ALIGNING_TORQUE_NM, MAX_FORCE_N, MAX_SLIP_ANGLE_TURNS, MAX_SLIP_MS, MAX_WHEEL_SPIN,
    ROAD_TEMPERATURE_C, TyreHook, TyreModel, TyreQuery, TyreReject, TyreResult,
};
pub use rolling::RollingTireResponse;
pub use scales::StepScales;
pub use spec::TireSpec;
pub use state::{LoadedInput, ROLLING_FRICTION, Tire, TireParams, WHEEL_INERTIA};

#[cfg(test)]
mod capture_tests;
#[cfg(test)]
mod integration_tests;
#[cfg(test)]
mod tests;

//! The tire model: wheel spin, slip, load-sensitive lateral force, friction ellipse and surface grip.
//! Spec section 3 of `docs/specs/vehicle-suspension-tires.md`.

mod damage;
mod lateral;
mod loaded;
mod scales;
mod spec;
pub mod spike;
mod state;

pub use damage::{BLOWN_TRACTION, TireDamage, TireState};
pub use lateral::lateral_force;
pub use scales::StepScales;
pub use spec::TireSpec;
pub use state::{LoadedInput, ROLLING_FRICTION, Tire, TireParams, WHEEL_INERTIA};

#[cfg(test)]
mod tests;

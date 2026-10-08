//! Engine data and pieces: torque curve, engine braking, inertia, clutch. The torque loop that couples
//! the engine to the wheels lives in [`crate::drivetrain`]. Spec: `docs/specs/vehicle-engine-drivetrain.md`.

mod clutch;
mod spec;

pub use clutch::{Clutch, ClutchState};
pub use spec::EngineSpec;

#[cfg(test)]
mod tests;

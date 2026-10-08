//! Transmission data, gear ids, shift logic, differentials and the torque loop that couples the engine to
//! the wheels. Spec: `docs/specs/vehicle-engine-drivetrain.md`.

mod spec;

pub use spec::{GEAR_FIRST, GEAR_NEUTRAL, GEAR_REVERSE, TransmissionSpec};

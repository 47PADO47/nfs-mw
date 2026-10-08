//! Transmission data, gear ids, shift logic, differentials and the torque loop that couples the engine to
//! the wheels. Spec: `docs/specs/vehicle-engine-drivetrain.md`.

mod diff;
mod gearbox;
mod powertrain;
mod spec;
mod split;
mod torque_loop;
mod wheels;

pub use diff::calc_split;
pub use gearbox::{ShiftPoints, ShiftPotential};
pub use powertrain::{Powertrain, TickInput};
pub use spec::{GEAR_FIRST, GEAR_NEUTRAL, GEAR_REVERSE, TransmissionSpec};
pub use split::split_drive_torque;
pub use wheels::{DrivenAxles, driven_speeds, write_back};

#[cfg(test)]
mod tests;

//! Controllers that turn "go to that point at that speed" into the gas, brake, handbrake and steering
//! a human would produce, for computer-driven cars of EA Black Box games.
//! Spec: `docs/specs/ai-driver-control.md`, `docs/specs/ai-driver-control-pid.md`.
//!
//! - [`Driver`]: reverse logic, steering and pedals, for the simple traffic controller or the PID one.
//! - [`StuckDetector`]: a car that presses the gas and does not move.
//!
//! Pure maths on plain numbers: no I/O, no vehicle or road dependency. Physics space (x right, y up,
//! z forward), metres, seconds, radians; steering is signed with positive to the right.

mod adaptive;
mod controls;
mod driver;
mod lookup;
mod pid_error;
mod simple;
mod steering_pid;
mod stuck;
mod throttle_pid;

#[cfg(test)]
mod tests;

pub use adaptive::{AdaptivePid, AdaptiveSettings};
pub use controls::{AiControls, DriveFlags, GearRequest};
pub use driver::{ControllerKind, DriveRequest, Driver, VehicleView};
pub use lookup::{Graph, Table, ramp};
pub use pid_error::PidError;
pub use simple::{heading_to, simple_pedals, simple_steering};
pub use steering_pid::SteeringPid;
pub use stuck::StuckDetector;
pub use throttle_pid::{Pedals, ThrottleInput, ThrottlePid};

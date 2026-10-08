//! Steering: the speed-sensitive angle limit and rate of a driver-steered car, counter-steer help, and
//! the Ackermann split into left and right wheel angles. Spec section 8.5 of
//! `docs/specs/vehicle-input-induction-brakes.md` and section 6 of
//! `docs/specs/vehicle-steering-assists-aero.md`.

mod ackermann;
mod human;
mod tables;
mod window;

pub use ackermann::{WheelAngles, ackermann};
pub use human::{Steering, SteeringDevice, SteeringInput};
pub use tables::ABSOLUTE_MAX_STEERING;

#[cfg(test)]
mod tests;

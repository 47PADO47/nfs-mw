//! Four spring/damper corners: chassis data, wheel geometry, the ground probe, compression and the
//! spring, damper and anti-roll force. Spec section 1 and 2 of `docs/specs/vehicle-suspension-tires.md`.

mod geometry;
mod probe;
mod spec;
mod spring;

pub use geometry::{Geometry, center_of_gravity};
pub use probe::{Compression, PROBE_LIFT, WheelContact, compression, probe};
pub use spec::{AxleConstants, ChassisSpec};
pub use spring::{Corner, Side, SpringInput, spring_force};

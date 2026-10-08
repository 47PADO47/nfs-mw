//! A car's physics data: the AttribSys classes `pvehicle` links to, read into the plain parameter
//! structs of `blackbox-vehicle`. Field meanings and units are in `docs/specs/vehicle-*.md`.

mod bounds;
mod fields;
mod powertrain;

pub use bounds::{CarBounds, car_bounds, read_car_bounds};
pub use fields::Fields;
pub use powertrain::{engine, induction, nos, transmission};

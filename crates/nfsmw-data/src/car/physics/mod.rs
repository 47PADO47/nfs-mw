//! A car's physics data: the AttribSys classes `pvehicle` links to, read into the plain parameter
//! structs of `blackbox-vehicle`. Field meanings and units are in `docs/specs/vehicle-*.md`.

mod body;
mod bounds;
mod chassis;
mod fields;
mod powertrain;
mod running_gear;
mod surface;

pub use body::{BodyData, body, rigid_body_spec};
pub use bounds::{CarBounds, car_bounds, read_car_bounds};
pub use chassis::{aero, chassis};
pub use fields::Fields;
pub use powertrain::{engine, induction, nos, transmission};
pub use running_gear::{brakes, tires};
pub use surface::SurfaceTable;

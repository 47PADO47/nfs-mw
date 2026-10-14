//! A car's physics data: the AttribSys classes `pvehicle` links to, read into the plain parameter
//! structs of `blackbox-vehicle`. Field meanings and units are in `docs/specs/vehicle-*.md`.

mod body;
mod bounds;
mod chassis;
mod data;
mod events;
mod fields;
mod geometry;
mod jump;
pub mod performance;
mod powertrain;
mod reactions;
mod running_gear;
mod spec;
mod surface;
pub mod tuning;
mod wall_steering;

pub use body::{BodyData, WallSpec, body, rigid_body_spec, wall_spec};
pub use bounds::{CarBounds, car_bounds, read_car_bounds};
pub use chassis::{aero, chassis};
pub use data::PhysicsData;
pub use events::{CarEventSpec, DAMAGE_ZONES};
pub use fields::Fields;
pub use geometry::CarGeometry;
pub use jump::jump_config;
pub use performance::{PerformanceCatalog, PerformanceCategory, PerformancePackage};
pub use powertrain::{engine, induction, no_engine, no_transmission, nos, transmission};
pub use reactions::{AiCollisionReactions, CollisionReaction, CollisionReactions, GoalReaction};
pub use running_gear::{brakes, tires};
pub use spec::{CarPhysics, car_physics};
pub use surface::{SurfaceAudio, SurfaceTable};
pub use wall_steering::wall_steering_config;

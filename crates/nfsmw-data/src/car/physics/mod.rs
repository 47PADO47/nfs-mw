//! A car's physics data: the AttribSys classes `pvehicle` links to, read into the plain parameter
//! structs of `blackbox-vehicle`. Field meanings and units are in `docs/specs/vehicle-*.md`.

mod body;
mod bounds;
mod chassis;
mod data;
mod events;
mod fields;
mod from_stats;
mod geometry;
mod jump;
pub mod measured;
pub mod performance;
mod powertrain;
pub mod ratings;
mod reactions;
mod rolling_chassis;
mod running_gear;
mod simple_chassis;
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
pub use from_stats::{ai_vehicle_from_stats, physics_from_stats};
pub use geometry::CarGeometry;
pub use jump::jump_config;
pub use performance::{PerformanceCatalog, PerformanceCategory, PerformancePackage};
pub use powertrain::{engine, induction, no_engine, no_transmission, nos, transmission};
pub use reactions::{AiCollisionReactions, CollisionReaction, CollisionReactions, GoalReaction};
pub use rolling_chassis::{traffic_chassis_config, trailer_chassis_config, trailer_cog};
pub use running_gear::{brakes, tires};
pub use simple_chassis::{destroyed_coasting_tires, simple_chassis_config};
pub use spec::{CarPhysics, car_physics};
pub use surface::{SurfaceAudio, SurfaceTable};
pub use wall_steering::wall_steering_config;

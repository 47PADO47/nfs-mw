//! The cop goals of a pursuit as pure rules: the slot records of the pit, box-in and rolling block (where a cop chases
//! the target and where it drives once the finisher is armed), the shared ram approach (aim, lead, speed ceiling and
//! limiter) and the pull-over (the collapse: separation, arrival and the detour). Nothing here reads a world or a clock.
//! The caller passes positions, the limiter's last value and `dt`, and feeds the [`GoalCommand`] to the car driver.
//! Spec: `docs/specs/ai-pursuit-goals.md`.
//!
//! Frame: a [`glam::Vec2`] is the map plane `(x, z)`. In the target's frame `x` is to its right and `y` is forward, as
//! the spec writes `(x, z)`.

pub mod approach;
pub mod box_in;
pub mod pit;
pub mod pull_over;
pub mod record;
pub mod rolling_block;
pub mod tuning;

pub use approach::{AiVehicle, Ceiling, CopState, GoalCommand, Step, TargetState};
pub use record::{GoalKind, Record};
pub use tuning::Tuning;

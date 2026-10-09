//! The vehicle as one rigid body: fixed-step integration, box inertia, drag, sleep and a plane-contact
//! impulse routine. Spec: `docs/specs/vehicle-rigid-body.md`.

mod bodies;
mod body;
mod contact;
mod inertia;
mod joint;
mod obb;
mod spec;

pub use bodies::{BodyContact, BodyHit, react_bodies, separate};
pub use body::{BodyState, MAX_ANGULAR_SPEED, MAX_LINEAR_SPEED, RigidBody};
pub use contact::{ContactParams, FrictionState, PlaneContact, Reaction};
pub use inertia::{box_inertia, inverse_diagonal};
pub use joint::{BAUMGARTE, BallJoint, MAX_CORRECTION_SPEED};
pub use obb::{Obb, ObbContact, obb_contact};
pub use spec::RigidBodySpec;

#[cfg(test)]
mod joint_tests;
#[cfg(test)]
mod tests;

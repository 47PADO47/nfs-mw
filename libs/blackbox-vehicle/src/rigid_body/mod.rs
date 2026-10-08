//! The vehicle as one rigid body: fixed-step integration, box inertia, drag, sleep and a plane-contact
//! impulse routine. Spec: `docs/specs/vehicle-rigid-body.md`.

mod body;
mod contact;
mod inertia;
mod spec;

pub use body::{BodyState, MAX_ANGULAR_SPEED, MAX_LINEAR_SPEED, RigidBody};
pub use contact::{ContactParams, FrictionState, PlaneContact, Reaction};
pub use inertia::{box_inertia, inverse_diagonal};
pub use spec::RigidBodySpec;

#[cfg(test)]
mod tests;

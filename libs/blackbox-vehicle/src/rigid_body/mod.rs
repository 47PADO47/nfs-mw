//! The vehicle as one rigid body: fixed-step integration, box inertia, drag, sleep and a plane-contact
//! impulse routine. Spec: `docs/specs/vehicle-rigid-body.md`.

mod bodies;
mod body;
mod contact;
mod inertia;
mod joint;
mod material;
mod moment;
mod obb;
mod separation;
mod spec;
mod sphere;

pub use bodies::{BodyContact, BodyHit, BodyReaction, Separation, react_bodies, resolve_bodies, separate};
pub use body::{BodyState, MAX_ANGULAR_SPEED, MAX_LINEAR_SPEED, RigidBody};
pub use contact::{ContactParams, FrictionState, PlaneContact, Reaction};
pub use inertia::{box_inertia, inverse_diagonal};
pub use joint::{BAUMGARTE, BallJoint, MAX_CORRECTION_SPEED};
pub use material::ObjectMaterial;
pub use moment::{ContactMoment, react_moments};
pub use obb::{Obb, ObbContact, box_contact, obb_contact};
pub use spec::RigidBodySpec;
pub use sphere::{Sphere, sphere_box_contact, sphere_contact};

#[cfg(test)]
mod joint_tests;
#[cfg(test)]
mod moment_tests;
#[cfg(test)]
mod tests;

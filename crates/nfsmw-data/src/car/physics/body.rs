//! The rigid body of a car: `pvehicle`'s mass and tensor scale, and the `rigidbodyspecs` record it links
//! to (`docs/specs/vehicle-rigid-body.md` §9).

use blackbox_vehicle::rigid_body::RigidBodySpec;
use glam::Vec3;

use super::fields::Fields;

/// What the rigid body is built from besides the size of the car's box.
#[derive(Debug, Clone)]
pub struct BodyData {
    /// Kilograms.
    pub mass: f32,
    /// Per-axis multiplier of the box's inertia tensor.
    pub tensor_scale: Vec3,
    pub spec: RigidBodySpec,
}

/// The body of `pvehicle` collection `pvehicle`.
pub fn body(pvehicle: Fields<'_>) -> BodyData {
    let spec = pvehicle.link("rigidbodyspecs").map_or_else(RigidBodySpec::default, rigid_body_spec);
    let tensor = pvehicle.vec3("TENSOR_SCALE");
    BodyData { mass: pvehicle.f32("MASS"), tensor_scale: if tensor == Vec3::ZERO { Vec3::ONE } else { tensor }, spec }
}

pub fn rigid_body_spec(c: Fields<'_>) -> RigidBodySpec {
    RigidBodySpec {
        gravity: c.f32("GRAVITY"),
        cg: c.vec3("CG"),
        drag: c.vec3("DRAG"),
        drag_angular: c.vec3("DRAG_ANGULAR"),
        sleep_velocity: c.f32("SLEEP_VELOCITY"),
        collision_box_pad: c.vec3("COLLISION_BOX_PAD"),
        ground_friction: c.pair("GROUND_FRICTION"),
        ground_elasticity: c.vec3("GROUND_ELASTICITY"),
        ground_moment_scale: c.vec3("GROUND_MOMENT_SCALE"),
    }
}

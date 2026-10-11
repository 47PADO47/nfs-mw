//! The rigid body of a car: `pvehicle`'s mass and tensor scale, and the `rigidbodyspecs` record it links
//! to (`docs/specs/vehicle-rigid-body.md` §9).

use blackbox_vehicle::rigid_body::{ObjectMaterial, RigidBodySpec};
use glam::Vec3;

use super::fields::Fields;

/// How the car reacts to walls and barriers (`rigidbodyspecs`: `WALL_*`, `WORLD_MOMENT_SCALE`). The
/// vehicle library only resolves ground contacts itself; the game applies these in its wall hits.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct WallSpec {
    /// Static and kinetic friction coefficients.
    pub friction: [f32; 2],
    /// Restitution per body axis (right, up, forward).
    pub elasticity: Vec3,
    /// Inertia multiplier per body axis in world hits.
    pub moment_scale: Vec3,
}

impl Default for WallSpec {
    fn default() -> Self {
        Self { friction: [0.5, 0.3], elasticity: Vec3::splat(0.2), moment_scale: Vec3::ONE }
    }
}

/// What the rigid body is built from besides the size of the car's box.
#[derive(Debug, Clone)]
pub struct BodyData {
    /// Kilograms.
    pub mass: f32,
    /// Per-axis multiplier of the box's inertia tensor.
    pub tensor_scale: Vec3,
    pub spec: RigidBodySpec,
    pub walls: WallSpec,
}

/// The body of `pvehicle` collection `pvehicle`.
pub fn body(pvehicle: Fields<'_>) -> BodyData {
    let record = pvehicle.link("rigidbodyspecs");
    let spec = record.map_or_else(RigidBodySpec::default, rigid_body_spec);
    let walls = record.map_or_else(WallSpec::default, wall_spec);
    let tensor = pvehicle.vec3("TENSOR_SCALE");
    BodyData {
        mass: pvehicle.f32("MASS"),
        tensor_scale: if tensor == Vec3::ZERO { Vec3::ONE } else { tensor },
        spec,
        walls,
    }
}

pub fn wall_spec(c: Fields<'_>) -> WallSpec {
    WallSpec {
        friction: c.pair("WALL_FRICTION"),
        elasticity: c.vec3("WALL_ELASTICITY"),
        moment_scale: c.vec3("WORLD_MOMENT_SCALE"),
    }
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
        no_ground_collisions: c.bool("NO_GROUND_COLLISIONS"),
        objects: object_material(c),
    }
}

/// Per-body object parameters. A nonpositive fourth moment component leaves mass unchanged.
pub fn object_material(c: Fields<'_>) -> ObjectMaterial {
    let scale = c.0.get_vector4("OBJ_MOMENT_SCALE").unwrap_or([0.0; 4]);
    let mass_scale = match scale[3] > 0.0 {
        true => scale[3],
        false => 1.0,
    };
    ObjectMaterial {
        friction: c.pair("OBJ_FRICTION"),
        elasticity: c.vec3("OBJ_ELASTICITY"),
        inertia_scale: Vec3::new(scale[0], scale[1], scale[2]),
        mass_scale,
    }
}

//! A hinged part that has left the car: the body it becomes, where it starts and how fast it flies.
//!
//! The game's detached vehicle parts are fragments of the prop solver: a rigid body built from a box of the car's bounds,
//! with the dynamics of a `smackable` record, that falls, hits the ground and the world, spins, comes to rest and is
//! released after a while (`docs/specs/world-sequencers.md`, `prop-lifetime.md`). A mod car's part reuses that solver. It
//! has no bounds node or smackable record of the 2005 game to take its shape from, so its box is that of its own solid
//! ([`debris_bounds`]), its record is built from its own mass ([`debris_dynamics`]), and it starts at the pose of the car
//! with the car's velocity and a kick ([`part_frame`], [`debris_motion`]).
//!
//! Physics space is x right, y up, z forward; the car model's space is x forward, y left, z up (`scenes/world/space.rs`).

use blackbox_collision::{Bounds, BoundsSet, Shape, bounds_flags as flags};
use blackbox_vehicle::rigid_body::RigidBodySpec;
use glam::{Mat3, Mat4, Quat, Vec3};

use crate::car::physics::WallSpec;
use crate::world::PropDynamics;

/// The collision sounds of a part that hits the road are those of this `smackable` record of the game: the metal panels
/// of a prop. The record only names a sound; the body's physics are the part's own.
pub const DEBRIS_SOUND: &str = "largemetalpost";
/// The road surface key of `carbody`, the material a part is made of.
const CAR_BODY_SURFACE: u32 = 0x99DA_6531;
/// The smallest half size of a part's box, in metres (the solver's own floor).
const MIN_HALF: f32 = 0.03;
/// What the part is thrown with, relative to the car: outwards along the ground, up, and a spin about its hinge.
const KICK_OUT: f32 = 2.0;
const KICK_UP: f32 = 1.0;
const KICK_SPIN: f32 = 3.0;
/// Seconds out of the camera's sight before the part is released, and the sink time and speed after it rests unseen.
const KILL_OFF_SCREEN: f32 = 5.0;
const DROPOUT: [f32; 2] = [2.0, 0.25];

/// How a part moves the moment it leaves the car.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Motion {
    /// Linear velocity of the part's centre, physics space, m/s.
    pub velocity: Vec3,
    /// Angular velocity, physics space, rad/s.
    pub spin: Vec3,
}

/// Physics space from model space.
fn to_physics(p: Vec3) -> Vec3 {
    Vec3::new(-p.y, p.z, p.x)
}

/// The box of a part with half sizes `half` in the car model's space, as the bounds of its own body: one box that
/// collides with the world, the ground and other bodies, centred on the body's origin (physics axes: width, height,
/// length).
pub fn debris_bounds(half: Vec3) -> BoundsSet {
    let half = Vec3::new(half.y, half.z, half.x).max(Vec3::splat(MIN_HALF));
    let node = Bounds {
        orientation: [0.0, 0.0, 0.0, 1.0],
        position: [0.0; 3],
        flags: flags::BOX | flags::PRIM_VS_WORLD | flags::PRIM_VS_OBJECTS | flags::PRIM_VS_GROUND,
        half_dimensions: half.to_array(),
        child_count: 0,
        point_cloud: None,
        pivot: [0.0; 3],
        first_child: 0,
        radius: half.length(),
        surface: CAR_BODY_SURFACE,
        name_hash: 0,
    };
    BoundsSet { name_hash: 0, nodes: vec![node], point_clouds: Vec::new() }
}

/// The dynamics of a part of `mass_kg`: gravity, ordinary friction and bounce, a collision box like any prop's, no damage
/// to the cars it hits, and a short life once it is out of sight (`kill_off_screen`, then a sink).
pub fn debris_dynamics(mass_kg: f32, sound_profile: u32) -> PropDynamics {
    PropDynamics {
        sound_profile,
        mass: mass_kg.max(0.1),
        tensor_scale: Vec3::splat(2.0),
        body: RigidBodySpec::default(),
        walls: WallSpec::default(),
        no_world_collisions: false,
        no_object_collisions: false,
        fallback_shape: Shape::Box,
        base_surface: CAR_BODY_SURFACE,
        detach_impulse: 0.0,
        no_car_effect: true,
        camera_avoidable: false,
        immobile: false,
        respawn_time: 0.0,
        kill_off_screen: KILL_OFF_SCREEN,
        dropout: DROPOUT,
        allow_off_world: false,
        event_sequencer: 0,
        start_sequencer: false,
        no_trigger: true,
        explosion_effect: 0.0,
        cost_to_state: 0,
    }
}

/// Where the body of a part starts, from the car's `model` matrix (render space, as `CarPose::transform` gives it) and the
/// centre of the part's box in the car model's space: the centre in physics space and the orientation of the car as the
/// solver holds an orientation (its columns are the body's width, height and length axes).
pub fn part_frame(model: Mat4, centre: Vec3) -> (Vec3, Quat) {
    let axis = |v: Vec3| to_physics(model.transform_vector3(v).normalize_or_zero());
    let rotation = Mat3::from_cols(-axis(Vec3::Y), axis(Vec3::Z), axis(Vec3::X));
    let orientation = Quat::from_mat3(&rotation).normalize();
    (to_physics(model.transform_point3(centre)), orientation)
}

/// A hinge `axis` of the car's model space as a unit direction in physics space, for the car at `model`.
pub fn part_axis(model: Mat4, axis: Vec3) -> Vec3 {
    to_physics(model.transform_vector3(axis)).normalize_or_zero()
}

/// The state of the car a part leaves, physics space.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Parent {
    pub position: Vec3,
    pub velocity: Vec3,
    pub angular_velocity: Vec3,
}

/// The motion of a part whose centre is at `centre` (physics space) when it leaves `parent`. It keeps the velocity of the
/// point of the car it sat on (the car's velocity plus the car's spin about its position), and is thrown outwards from
/// the car along the ground and upwards; it spins about its hinge `axis` (a unit vector, physics space) the way it had
/// turned (`turned` degrees, signed).
pub fn debris_motion(parent: &Parent, centre: Vec3, axis: Vec3, turned: f32) -> Motion {
    let arm = centre - parent.position;
    let outward = Vec3::new(arm.x, 0.0, arm.z).normalize_or_zero();
    let velocity = parent.velocity + parent.angular_velocity.cross(arm) + outward * KICK_OUT + Vec3::Y * KICK_UP;
    let sign = if turned < 0.0 { -1.0 } else { 1.0 };
    Motion { velocity, spin: parent.angular_velocity + axis.normalize_or_zero() * KICK_SPIN * sign }
}

#[cfg(test)]
mod tests {
    use super::*;
    use blackbox_collision::{BodyGeometry, GeometryOptions};

    fn parent() -> Parent {
        Parent {
            position: Vec3::new(10.0, 0.5, 20.0),
            velocity: Vec3::new(0.0, 0.0, 30.0),
            angular_velocity: Vec3::ZERO,
        }
    }

    #[test]
    fn the_box_of_a_part_is_one_primitive_in_physics_axes() {
        // A part 1.4 m long, 1.0 wide and 0.1 high in the model's space (x forward, y left, z up).
        let bounds = debris_bounds(Vec3::new(0.7, 0.5, 0.05));
        let options = GeometryOptions {
            fallback_half: Vec3::splat(0.1),
            fallback_shape: Shape::Box,
            box_padding: Vec3::ZERO,
            surface: 0,
        };
        let geometry = BodyGeometry::from_bounds(&bounds, 0, options).unwrap();
        assert!(!geometry.used_fallback);
        assert_eq!(geometry.primitives.len(), 1);
        let primitive = &geometry.primitives[0];
        assert_eq!(primitive.half, Vec3::new(0.5, 0.05, 0.7), "width, height, length");
        assert!(primitive.collides(flags::PRIM_VS_GROUND) && primitive.collides(flags::PRIM_VS_WORLD));
        assert_eq!(primitive.centre, Vec3::ZERO);
        assert_eq!(debris_bounds(Vec3::ZERO).nodes[0].half_dimensions, [MIN_HALF; 3], "never a flat box");
    }

    #[test]
    fn the_dynamics_are_a_light_prop_that_does_not_hurt_cars() {
        let dynamics = debris_dynamics(30.0, 7);
        assert_eq!((dynamics.mass, dynamics.sound_profile), (30.0, 7));
        assert!(dynamics.body.gravity < 0.0 && !dynamics.body.no_ground_collisions);
        assert!(dynamics.no_car_effect && !dynamics.no_world_collisions && !dynamics.immobile);
        assert!(dynamics.kill_off_screen > 0.0 && dynamics.dropout[0] > 0.0, "it is released after a while");
        assert_eq!(dynamics.detach_impulse, 0.0, "it starts free, not attached");
        assert!(debris_dynamics(0.0, 0).mass > 0.0);
    }

    #[test]
    fn a_part_starts_where_the_car_is_and_faces_as_the_car_does() {
        let identity = part_frame(Mat4::IDENTITY, Vec3::new(1.5, 0.2, 0.8));
        // Forward (model +x) is physics +z, left (+y) is -x, up is up.
        assert!((identity.0 - Vec3::new(-0.2, 0.8, 1.5)).length() < 1e-6);
        assert!(identity.1.angle_between(Quat::IDENTITY) < 1e-5);
        // A car turned a quarter to the left (about the model's up axis), 10 m ahead: its forward axis is the model +y.
        let model = Mat4::from_rotation_translation(Quat::from_rotation_z(std::f32::consts::FRAC_PI_2), Vec3::X * 10.0);
        let (centre, orientation) = part_frame(model, Vec3::new(1.0, 0.0, 0.0));
        assert!((centre - to_physics(Vec3::new(10.0, 1.0, 0.0))).length() < 1e-5);
        let forward = orientation * Vec3::Z;
        assert!((forward - to_physics(Vec3::Y)).length() < 1e-5, "the body's length axis follows the car");
    }

    #[test]
    fn a_part_keeps_the_motion_of_the_car_and_is_thrown_clear() {
        let still = Parent { velocity: Vec3::ZERO, ..parent() };
        let front = still.position + Vec3::new(0.0, 0.3, 2.0);
        let motion = debris_motion(&still, front, Vec3::X, -15.0);
        assert!(motion.velocity.z > 1.5 && motion.velocity.y > 0.5, "forward and up from a front part: {motion:?}");
        assert!(motion.spin.x < 0.0, "it spins the way it was turning");
        assert_eq!(debris_motion(&still, front, Vec3::X, 15.0).spin.x, -motion.spin.x);

        let moving = debris_motion(&parent(), front, Vec3::X, -15.0);
        assert!((moving.velocity.z - motion.velocity.z - 30.0).abs() < 1e-4, "it keeps the car's 30 m/s");

        // A car spinning about its up axis carries a part on its side along the circle.
        let turning = Parent { angular_velocity: Vec3::Y * 2.0, velocity: Vec3::ZERO, ..parent() };
        let side = turning.position + Vec3::X * 1.0;
        let thrown = debris_motion(&turning, side, Vec3::Z, 1.0);
        assert!((thrown.velocity.z - (-2.0)).abs() < 1e-4, "v = w x r: {thrown:?}");
        assert!(thrown.spin.y == 2.0, "the car's spin is kept");
    }

    #[test]
    fn a_hinge_axis_follows_the_car() {
        assert!((part_axis(Mat4::IDENTITY, Vec3::Y) - Vec3::NEG_X).length() < 1e-6, "left is -x in physics");
        let model = Mat4::from_rotation_z(std::f32::consts::FRAC_PI_2);
        assert!((part_axis(model, Vec3::X) - Vec3::NEG_X).length() < 1e-6, "forward turned to the left");
        assert_eq!(part_axis(Mat4::IDENTITY, Vec3::ZERO), Vec3::ZERO);
    }

    #[test]
    fn a_part_at_the_cars_centre_is_only_lifted() {
        let motion = debris_motion(&Parent { velocity: Vec3::ZERO, ..parent() }, parent().position, Vec3::ZERO, 0.0);
        assert_eq!(motion.velocity, Vec3::Y * KICK_UP);
        assert_eq!(motion.spin, Vec3::ZERO);
    }
}

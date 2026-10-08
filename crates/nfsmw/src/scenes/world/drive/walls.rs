//! Walls and barriers against the car's body (`docs/specs/vehicle-rigid-body.md` §4.3, with the data
//! in `docs/formats/collision.md`).
//!
//! The vehicle library resolves the road under the tyres and the body box against the ground. Walls
//! are the game's job: probes on the body's mid-height outline are cast from its centre against the
//! resident collision packs (barriers and steep faces), and every one that sticks into something is
//! pushed out and reacts through the rigid body's impulse routine with the car's wall friction and
//! restitution.

use blackbox_collision::{CollisionWorld, RayOptions};
use blackbox_vehicle::Vehicle;
use blackbox_vehicle::rigid_body::{ContactParams, PlaneContact};
use glam::{Mat3, Vec3};
use nfsmw_data::car::physics::WallSpec;

/// A surface whose normal points up more than this is a floor, not a wall.
const FLOOR_NORMAL: f32 = 0.7;
/// Hits deeper than this are ignored (a wall we are already far inside is not one we just hit).
const MAX_DEPTH: f32 = 1.5;

/// A probe point sticking into a wall.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct WallContact {
    /// Where the wall is, world space.
    pub point: Vec3,
    /// Unit normal facing the car.
    pub normal: Vec3,
    /// How far the probe is past the wall, metres.
    pub depth: f32,
}

/// The probe points on the body's outline at mid-height, in the body frame: the four corners and the
/// middle of each side.
fn probes(half: Vec3) -> [Vec3; 8] {
    [
        Vec3::new(half.x, 0.0, half.z),
        Vec3::new(-half.x, 0.0, half.z),
        Vec3::new(half.x, 0.0, -half.z),
        Vec3::new(-half.x, 0.0, -half.z),
        Vec3::new(0.0, 0.0, half.z),
        Vec3::new(0.0, 0.0, -half.z),
        Vec3::new(half.x, 0.0, 0.0),
        Vec3::new(-half.x, 0.0, 0.0),
    ]
}

/// The first thing a segment meets: where, and the surface normal facing the segment's start.
pub type Cast<'a> = &'a dyn Fn(Vec3, Vec3) -> Option<(Vec3, Vec3)>;

/// Ray casts against the resident collision packs: faces and barriers.
pub fn world_cast(collision: &CollisionWorld) -> impl Fn(Vec3, Vec3) -> Option<(Vec3, Vec3)> + '_ {
    move |from, to| {
        let hit = collision.ray_cast(from.to_array(), to.to_array(), &RayOptions::default())?;
        Some((Vec3::from(hit.point), Vec3::from(hit.normal)))
    }
}

/// The probes of a body at `position` with orientation `rot` and half extents `half` that are in a
/// wall, deepest first. Only barriers and steep faces count; the ground is the tyres' business.
pub fn find(cast: Cast<'_>, position: Vec3, rot: Mat3, half: Vec3) -> Vec<WallContact> {
    let mut found: Vec<WallContact> = probes(half)
        .iter()
        .filter_map(|&local| {
            let probe = position + rot * local;
            let (point, normal) = cast(position, probe)?;
            if normal.y > FLOOR_NORMAL {
                return None;
            }
            let depth = (point - probe).dot(normal);
            (depth > 1e-4 && depth < MAX_DEPTH).then_some(WallContact { point, normal, depth })
        })
        .collect();
    found.sort_by(|a, b| b.depth.total_cmp(&a.depth));
    found
}

/// Resolve wall contacts on `vehicle`: each reacts with the wall friction and restitution, and the
/// deepest pushes the body out. Returns the strongest impulse (N s), 0 without a hit.
pub fn resolve(vehicle: &mut Vehicle, cast: Cast<'_>, walls: &WallSpec) -> f32 {
    let body = vehicle.body();
    let half = body.dimension() + body.spec().collision_box_pad;
    let (position, rot) = (body.position, body.rotation());
    let contacts = find(cast, position, rot, half);
    let Some(deepest) = contacts.first().copied() else { return 0.0 };
    let mut strongest = 0.0f32;
    for c in &contacts {
        let n = c.normal;
        let e = Vec3::new(n.dot(rot.x_axis), n.dot(rot.y_axis), n.dot(rot.z_axis)) * walls.elasticity;
        let plane = PlaneContact {
            point: c.point,
            normal: n,
            friction_static: walls.friction[0],
            friction_kinetic: walls.friction[1].min(walls.friction[0]),
        };
        let params = ContactParams { restitution: e.length(), inertia_scale: walls.moment_scale };
        if let Some(reaction) = vehicle.body_mut().react_plane(&plane, &params) {
            strongest = strongest.max(reaction.impulse);
            // One reaction per step, as the ground code does: the deepest contact decides.
            break;
        }
    }
    vehicle.body_mut().position += deepest.normal * deepest.depth;
    if strongest > 0.0 {
        vehicle.notify_collision(strongest);
    }
    strongest
}

#[cfg(test)]
mod tests {
    use blackbox_vehicle::{FIXED_STEP, FlatGround, InputState, VehicleSpec};

    use super::*;

    /// A wall: the plane z = `wall_z` (physics space), facing -z, like a barrier across the road.
    fn wall(wall_z: f32) -> impl Fn(Vec3, Vec3) -> Option<(Vec3, Vec3)> {
        move |from, to| {
            let crosses = from.z < wall_z && to.z >= wall_z;
            let t = (wall_z - from.z) / (to.z - from.z);
            crosses.then(|| (from + (to - from) * t, Vec3::NEG_Z))
        }
    }

    fn car() -> Vehicle {
        let mut v = Vehicle::new(VehicleSpec::example());
        assert!(v.place_on_ground(&FlatGround::new(0.0), 0.0, 0.0, 2.0, 0.0));
        v
    }

    #[test]
    fn a_probe_past_the_wall_is_a_contact() {
        let half = Vec3::new(0.9, 0.6, 2.2);
        let cast = wall(2.0);
        let contacts = find(&cast, Vec3::ZERO, Mat3::IDENTITY, half);
        // The front corners and the front middle poke 0.2 m through.
        assert_eq!(contacts.len(), 3);
        assert!(contacts.iter().all(|c| (c.depth - 0.2).abs() < 1e-4 && c.normal == Vec3::NEG_Z));
        assert!(find(&wall(3.0), Vec3::ZERO, Mat3::IDENTITY, half).is_empty());
    }

    #[test]
    fn floors_are_not_walls() {
        let floor = |from: Vec3, to: Vec3| Some((from.lerp(to, 0.5), Vec3::Y));
        assert!(find(&floor, Vec3::ZERO, Mat3::IDENTITY, Vec3::ONE).is_empty());
    }

    #[test]
    fn driving_into_a_wall_stops_the_car_and_keeps_it_out() {
        let ground = FlatGround::new(0.0);
        let mut v = car();
        let wall_z = 30.0;
        let cast = wall(wall_z);
        let mut worst_front = f32::MIN;
        let mut hit = 0.0f32;
        for _ in 0..600 {
            v.step(FIXED_STEP, &InputState { throttle: 1.0, ..InputState::default() }, &ground);
            hit = hit.max(resolve(&mut v, &cast, &WallSpec::default()));
            let front = v.position().z + v.body().dimension().z;
            worst_front = worst_front.max(front);
        }
        assert!(hit > 1000.0, "an impulse was felt: {hit}");
        assert!(worst_front < wall_z + 0.5, "the nose went {} m into the wall", worst_front - wall_z);
        assert!(v.forward_speed() < 8.0, "still going {} m/s after ten seconds against a wall", v.forward_speed());
    }

    #[test]
    fn no_wall_no_impulse() {
        let mut v = car();
        assert_eq!(resolve(&mut v, &wall(1000.0), &WallSpec::default()), 0.0);
    }
}

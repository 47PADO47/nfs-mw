//! Walls and barriers against the car's body (`docs/specs/vehicle-rigid-body.md` §4.3, with the data
//! in `docs/formats/collision.md`).
//!
//! The vehicle library resolves the road under the tyres and the body box against the ground. Walls
//! are the game's job: probes on the body's mid-height outline are cast from its centre against the
//! resident collision packs (barriers and steep faces), and every one that sticks into something is
//! pushed out and reacts through the rigid body's impulse routine with the car's wall friction and
//! restitution.

use blackbox_collision::{BARRIER_TWO_SIDED, CollisionWorld, GROUP_EXCLUSION, HitKind, RayOptions};
use blackbox_vehicle::Vehicle;
use blackbox_vehicle::rigid_body::{ContactParams, PlaneContact};
use glam::{Mat3, Vec3};
use nfsmw_data::car::physics::WallSpec;
use nfsmw_data::world::PropKind;

use crate::scenes::world::props::{Obb, PropWorld};
use crate::scenes::world::space;

/// Shallow faces belong to ground handling, including origin-facing downward normals.
const FLOOR_NORMAL: f32 = 0.7;
/// Hits deeper than this are ignored (a wall we are already far inside is not one we just hit).
const MAX_DEPTH: f32 = 1.5;
/// Visual telemetry budget, deepest first; the physics contacts remain unrestricted.
const MAX_VISUAL_CONTACTS: usize = 16;

/// Where a wall contact came from in the collision data, for the log.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct HitInfo {
    pub barrier: bool,
    /// The `simsurface` hash.
    pub surface: u32,
    pub section: u32,
    pub instance: usize,
}

/// What a wall contact is, for drawing it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ContactKind {
    /// A barrier of the collision packs (guard rails, concrete barriers, fences).
    Barrier,
    /// A steep face of the packs.
    Face,
    /// A rigid prop.
    PropRigid,
    /// A light prop the car pushes aside.
    PropLight,
}

/// Where the car met something this step, as the debug view draws it.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ContactPoint {
    pub point: Vec3,
    pub normal: Vec3,
    pub kind: ContactKind,
}

/// Inputs for collision visuals, captured before the wall response (physics space).
/// Selection and emission are specified in `docs/specs/vehicle-visual-effects.md`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct VisualContact {
    pub point: Vec3,
    pub normal: Vec3,
    /// Velocity of this body point, including rotation, before any wall/prop response.
    pub velocity: Vec3,
    /// This contact's actual normal impulse divided by car mass, m/s; zero without a reaction.
    pub impulse_delta_v: f32,
    /// World material hash, including 0; prop materials have not yet been retained by the host.
    pub surface: Option<u32>,
    pub kind: ContactKind,
    pub info: Option<HitInfo>,
    pub prop: Option<u32>,
}

/// A probe point sticking into a wall.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct WallContact {
    /// Where the wall is, world space.
    pub point: Vec3,
    /// Unit normal facing the car.
    pub normal: Vec3,
    /// How far the probe is past the wall, metres.
    pub depth: f32,
    /// The prop this is, if it is one (its id, and its mass when it is a light one).
    pub prop: Option<(u32, Option<f32>)>,
    /// The collision data it is, unless it is a prop.
    pub info: Option<HitInfo>,
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

/// What a segment meets: where, the surface normal facing the segment's start, and the prop it is.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Hit {
    pub point: Vec3,
    pub normal: Vec3,
    pub prop: Option<(u32, Option<f32>)>,
    pub info: Option<HitInfo>,
}

pub type Cast<'a> = &'a dyn Fn(Vec3, Vec3) -> Option<Hit>;

/// Ray casts against the resident collision packs: faces and barriers.
///
/// Scenery-group geometry is left out: the original excludes it for vehicles (the road blocks and gates of
/// races and pursuits, which are off in free roam), and it is 18 % of all the barriers, many of them
/// across open road. A barrier is a wall from its front side only unless it is marked two-sided; a ray that
/// reaches it from behind passes (`docs/formats/collision.md`, "Query semantics" 6).
pub fn world_cast(collision: &CollisionWorld) -> impl Fn(Vec3, Vec3) -> Option<Hit> + '_ {
    let options = RayOptions { exclude: u32::from(GROUP_EXCLUSION), ..RayOptions::default() };
    move |from, to| {
        let hit = collision.ray_cast_filtered(from.to_array(), to.to_array(), &options, eligible)?;
        let info = HitInfo {
            barrier: hit.kind == HitKind::Barrier,
            surface: hit.surface_hash,
            section: hit.section,
            instance: hit.instance,
        };
        Some(Hit { point: Vec3::from(hit.point), normal: Vec3::from(hit.normal), prop: None, info: Some(info) })
    }
}

/// Reject floors and rear-facing one-way barriers before they can hide a valid wall farther along
/// the probe. A ray normal faces its origin, so a floor can have either sign of y.
fn eligible(hit: &blackbox_collision::Hit) -> bool {
    if hit.kind == HitKind::Face && hit.normal[1].abs() > FLOOR_NORMAL {
        return false;
    }
    if hit.kind == HitKind::Barrier && hit.surface_flags & BARRIER_TWO_SIDED == 0 && !hit.front_facing {
        return false;
    }
    true
}

/// The props the body's box overlaps, as wall contacts (physics space): the world's props are found by
/// box overlap, not by probe lines, so a thin post or a cone between two probes is not missed.
pub type PropQuery<'a> = &'a dyn Fn(Vec3, Mat3, Vec3) -> Vec<WallContact>;

pub fn world_props(props: &PropWorld) -> impl Fn(Vec3, Mat3, Vec3) -> Vec<WallContact> + '_ {
    move |centre, rot, half| {
        // Physics axes to render axes: the same map as for points.
        let to_render = |v: Vec3| space::to_render(v.to_array());
        let car = Obb { centre: to_render(centre), axes: [rot.x_axis, rot.y_axis, rot.z_axis].map(to_render), half };
        props
            .overlaps(&car)
            .into_iter()
            .map(|o| {
                let normal = Vec3::from(space::to_physics(o.normal));
                let mass = match o.kind {
                    PropKind::Light { mass } => Some(mass),
                    PropKind::Rigid => None,
                };
                // Where the car's box touches: its extreme point towards the prop.
                let reach =
                    (0..3).map(|i| half[i] * [rot.x_axis, rot.y_axis, rot.z_axis][i].dot(normal).abs()).sum::<f32>();
                WallContact {
                    point: centre - normal * reach,
                    normal,
                    depth: o.depth,
                    prop: Some((o.id, mass)),
                    info: None,
                }
            })
            .collect()
    }
}

/// The probes of a body at `position` with orientation `rot` and half extents `half` that are in a
/// wall, deepest first. Only barriers, props and steep faces count; the ground is the tyres' business.
pub fn find(cast: Cast<'_>, props: PropQuery<'_>, position: Vec3, rot: Mat3, half: Vec3) -> Vec<WallContact> {
    let mut found: Vec<WallContact> = probes(half)
        .iter()
        .filter_map(|&local| {
            let probe = position + rot * local;
            let hit = cast(position, probe)?;
            if hit.normal.y.abs() > FLOOR_NORMAL && hit.prop.is_none() {
                return None;
            }
            let depth = (hit.point - probe).dot(hit.normal);
            (depth > 1e-4 && depth < MAX_DEPTH).then_some(WallContact {
                point: hit.point,
                normal: hit.normal,
                depth,
                prop: hit.prop,
                info: hit.info,
            })
        })
        .collect();
    found.extend(props(position, rot, half).into_iter().filter(|c| c.depth < MAX_DEPTH));
    found.sort_by(|a, b| b.depth.total_cmp(&a.depth));
    found
}

/// What a step against the world did.
#[derive(Debug, Default, Clone, PartialEq)]
pub struct Impact {
    /// The strongest wall impulse, N s (0 without a hit).
    pub impulse: f32,
    /// Light props the car ran into, with their masses: the caller knocks them over.
    pub knocked: Vec<(u32, f32)>,
    /// How many rigid contacts the car has with the world this step (touching, not only hitting).
    pub rigid: usize,
    /// The deepest contact is on the car's front or back rather than its side.
    pub front: bool,
    /// Where the deepest contact is (physics space) and what it is.
    pub deepest: Option<(Vec3, Option<HitInfo>)>,
    /// Every contact of the step (physics space), deepest first: what `debug collisions` draws.
    pub contacts: Vec<ContactPoint>,
    /// At most sixteen rigid contacts, deepest first, retaining their pre-response velocities.
    pub visuals: Vec<VisualContact>,
}

impl From<&WallContact> for ContactPoint {
    fn from(c: &WallContact) -> Self {
        let kind = match (c.prop, c.info) {
            (Some((_, Some(_))), _) => ContactKind::PropLight,
            (Some((_, None)), _) => ContactKind::PropRigid,
            (None, Some(HitInfo { barrier: true, .. })) => ContactKind::Barrier,
            (None, _) => ContactKind::Face,
        };
        Self { point: c.point, normal: c.normal, kind }
    }
}

fn is_light(c: &WallContact) -> bool {
    matches!(c.prop, Some((_, Some(_))))
}

/// Resolve contacts on `vehicle`. Rigid contacts react with the wall friction and restitution and the
/// deepest pushes the body out; a light prop only costs the car the momentum it takes to push it away.
pub fn resolve(vehicle: &mut Vehicle, cast: Cast<'_>, props: PropQuery<'_>, walls: &WallSpec) -> Impact {
    let body = vehicle.body();
    let half = body.dimension() + body.spec().collision_box_pad;
    let (position, rot, car_mass) = (body.position, body.rotation(), body.mass());
    let contacts = find(cast, props, position, rot, half);
    let visuals = contacts
        .iter()
        .filter(|c| !is_light(c))
        .take(MAX_VISUAL_CONTACTS)
        .map(|c| VisualContact {
            point: c.point,
            normal: c.normal,
            velocity: body.point_velocity(c.point),
            impulse_delta_v: 0.0,
            surface: c.info.map(|i| i.surface),
            kind: ContactPoint::from(c).kind,
            info: c.info,
            prop: c.prop.map(|(id, _)| id),
        })
        .collect();
    let mut impact =
        Impact { contacts: contacts.iter().map(ContactPoint::from).collect(), visuals, ..Impact::default() };

    for c in contacts.iter().filter(|c| is_light(c)) {
        if let Some((id, Some(mass))) = c.prop
            && !impact.knocked.iter().any(|&(k, _)| k == id)
        {
            impact.knocked.push((id, mass));
            // Pushing the prop away takes a share of the momentum: m_car / (m_car + m_prop).
            vehicle.body_mut().linear_velocity *= car_mass / (car_mass + mass);
        }
    }

    let rigid: Vec<&WallContact> = contacts.iter().filter(|c| !is_light(c)).collect();
    let Some(deepest) = rigid.first().map(|c| **c) else { return impact };
    impact.rigid = rigid.len();
    impact.deepest = Some((deepest.point, deepest.info));
    impact.front = deepest.normal.dot(rot.z_axis).abs() > deepest.normal.dot(rot.x_axis).abs();
    for (index, c) in rigid.iter().enumerate() {
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
            impact.impulse = impact.impulse.max(reaction.impulse);
            if let Some(visual) = impact.visuals.get_mut(index) {
                visual.impulse_delta_v = reaction.impulse / car_mass;
            }
            // One reaction per step, as the ground code does: the deepest contact decides.
            break;
        }
    }
    vehicle.body_mut().position += deepest.normal * deepest.depth;
    if impact.impulse > 0.0 {
        vehicle.notify_collision(impact.impulse);
    }
    impact
}

#[cfg(test)]
mod regression_tests;
#[cfg(test)]
mod visual_tests;

#[cfg(test)]
mod tests {
    use blackbox_vehicle::{FIXED_STEP, FlatGround, InputState, VehicleSpec};

    use super::*;

    fn no_props(_: Vec3, _: Mat3, _: Vec3) -> Vec<WallContact> {
        vec![]
    }

    /// A wall: the plane z = `wall_z` (physics space), facing -z, like a barrier across the road.
    fn wall(wall_z: f32) -> impl Fn(Vec3, Vec3) -> Option<Hit> {
        move |from, to| {
            let crosses = from.z < wall_z && to.z >= wall_z;
            let t = (wall_z - from.z) / (to.z - from.z);
            crosses.then(|| Hit { point: from + (to - from) * t, normal: Vec3::NEG_Z, prop: None, info: None })
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
        let contacts = find(&cast, &no_props, Vec3::ZERO, Mat3::IDENTITY, half);
        // The front corners and the front middle poke 0.2 m through.
        assert_eq!(contacts.len(), 3);
        assert!(contacts.iter().all(|c| (c.depth - 0.2).abs() < 1e-4 && c.normal == Vec3::NEG_Z));
        assert!(find(&wall(3.0), &no_props, Vec3::ZERO, Mat3::IDENTITY, half).is_empty());
    }

    #[test]
    fn floors_are_not_walls() {
        let floor =
            |from: Vec3, to: Vec3| Some(Hit { point: from.lerp(to, 0.5), normal: Vec3::Y, prop: None, info: None });
        assert!(find(&floor, &no_props, Vec3::ZERO, Mat3::IDENTITY, Vec3::ONE).is_empty());
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
            hit = hit.max(resolve(&mut v, &cast, &no_props, &WallSpec::default()).impulse);
            let front = v.position().z + v.body().dimension().z;
            worst_front = worst_front.max(front);
        }
        assert!(hit > 1000.0, "an impulse was felt: {hit}");
        assert!(worst_front < wall_z + 0.5, "the nose went {} m into the wall", worst_front - wall_z);
        assert!(v.forward_speed() < 8.0, "still going {} m/s after ten seconds against a wall", v.forward_speed());
    }

    #[test]
    fn every_contact_of_the_step_is_reported_with_its_kind() {
        let mut v = car();
        let impact = resolve(&mut v, &wall(2.0), &no_props, &WallSpec::default());
        // Three probes poke through the plane; with no collision data behind it the wall is a face.
        assert_eq!(impact.contacts.len(), 3);
        assert!(impact.contacts.iter().all(|c| c.kind == ContactKind::Face && c.normal == Vec3::NEG_Z));
        let barrier = WallContact {
            point: Vec3::ZERO,
            normal: Vec3::X,
            depth: 0.1,
            prop: None,
            info: Some(HitInfo { barrier: true, surface: 0, section: 0, instance: 0 }),
        };
        let prop = |mass| WallContact { prop: Some((1, mass)), info: None, ..barrier };
        assert_eq!(ContactPoint::from(&barrier).kind, ContactKind::Barrier);
        assert_eq!(ContactPoint::from(&prop(None)).kind, ContactKind::PropRigid);
        assert_eq!(ContactPoint::from(&prop(Some(80.0))).kind, ContactKind::PropLight);
    }

    #[test]
    fn no_wall_no_impulse() {
        let mut v = car();
        assert_eq!(resolve(&mut v, &wall(1000.0), &no_props, &WallSpec::default()), Impact::default());
    }

    /// A prop standing across the road at `z`: a contact whenever the front of the box reaches it.
    /// `mass` of `None` is a rigid one.
    fn prop(z: f32, mass: Option<f32>) -> impl Fn(Vec3, Mat3, Vec3) -> Vec<WallContact> {
        move |centre, _, half| {
            let depth = centre.z + half.z - z;
            if depth > 0.0 && centre.z < z {
                vec![WallContact {
                    point: Vec3::new(centre.x, centre.y, z),
                    normal: Vec3::NEG_Z,
                    depth,
                    prop: Some((7, mass)),
                    info: None,
                }]
            } else {
                vec![]
            }
        }
    }

    #[test]
    fn a_light_prop_is_pushed_aside_at_the_cost_of_some_speed() {
        let ground = FlatGround::new(0.0);
        let mut v = car();
        v.place_moving(Vec3::new(0.0, v.position().y, 0.0), glam::Quat::IDENTITY, 20.0);
        // The prop is gone once it has been knocked, as in the game.
        let gone = std::cell::Cell::new(false);
        let inner = prop(2.0, Some(100.0));
        let props = |c: Vec3, r: Mat3, h: Vec3| if gone.get() { vec![] } else { inner(c, r, h) };
        let before = v.forward_speed();
        let mut knocked = vec![];
        for _ in 0..3 {
            v.step(FIXED_STEP, &InputState { throttle: 1.0, ..InputState::default() }, &ground);
            let hit = resolve(&mut v, &no_cast, &props, &WallSpec::default()).knocked;
            gone.set(gone.get() || !hit.is_empty());
            knocked.extend(hit);
        }
        assert_eq!(knocked, [(7, 100.0)]);
        // The car keeps most of its speed: it lost about m_prop / (m_car + m_prop) of it, once.
        assert!(v.forward_speed() > before * 0.85, "{} -> {}", before, v.forward_speed());
        assert!(v.position().z > 0.5, "the car went through");
    }

    #[test]
    fn a_rigid_prop_is_a_wall() {
        let ground = FlatGround::new(0.0);
        let mut v = car();
        v.place_moving(Vec3::new(0.0, v.position().y, 0.0), glam::Quat::IDENTITY, 20.0);
        let props = prop(10.0, None);
        let mut knocked = 0;
        for _ in 0..240 {
            v.step(FIXED_STEP, &InputState { throttle: 1.0, ..InputState::default() }, &ground);
            knocked += resolve(&mut v, &no_cast, &props, &WallSpec::default()).knocked.len();
        }
        assert_eq!(knocked, 0);
        assert!(v.position().z + v.body().dimension().z < 10.6, "stopped at the prop: z {}", v.position().z);
    }

    fn no_cast(_: Vec3, _: Vec3) -> Option<Hit> {
        None
    }
}

//! Walls and barriers against the car's body (`docs/specs/vehicle-rigid-body.md` §4.3, with the data
//! in `docs/formats/collision.md`).
//!
//! The vehicle library resolves the road under the tyres and the body box against the ground. Walls
//! are the game's job: probes on the body's mid-height outline are cast from its centre against the
//! resident collision packs (barriers and steep faces), and every one that sticks into something is
//! pushed out and reacts through the rigid body's impulse routine with the car's wall friction and
//! restitution.

use blackbox_collision::{BARRIER_TWO_SIDED, CollisionWorld, HitKind, RayOptions};
use blackbox_vehicle::Vehicle;
use blackbox_vehicle::rigid_body::{ContactParams, PlaneContact};
use glam::{Mat3, Vec3};
use nfsmw_data::car::physics::WallSpec;
use nfsmw_data::world::PropKind;

use crate::audio::{CarContact, ContactSource};
use crate::scenes::world::events::{DamageHit, OtherBody};
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
    /// A movable prop; its mass comes from the installed attributes.
    PropMovable,
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
/// The caller supplies the body's driver mask; scenery groups are independently inactive in free roam.
/// A barrier is a wall from its front side only unless it is marked two-sided; a ray that
/// reaches it from behind passes (`docs/formats/collision.md`, "Query semantics" 6).
pub fn world_cast_with_mask(collision: &CollisionWorld, exclusion: u16) -> impl Fn(Vec3, Vec3) -> Option<Hit> + '_ {
    let options = RayOptions { exclude: u32::from(exclusion), skip_groups: true, ..RayOptions::default() };
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
                    PropKind::Movable { mass } => Some(mass),
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
#[cfg(test)]
pub fn find(cast: Cast<'_>, props: PropQuery<'_>, position: Vec3, rot: Mat3, half: Vec3) -> Vec<WallContact> {
    let mut found = Vec::new();
    find_into(cast, props, position, rot, half, &mut found);
    found
}

/// As `find`, but fills `found` (cleared first) so the caller's buffer is reused from step to step.
pub fn find_into(
    cast: Cast<'_>,
    props: PropQuery<'_>,
    position: Vec3,
    rot: Mat3,
    half: Vec3,
    found: &mut Vec<WallContact>,
) {
    found.clear();
    found.extend(probes(half).iter().filter_map(|&local| {
        let probe = position + rot * local;
        let hit = cast(position, probe)?;
        if hit.normal.y.abs() > FLOOR_NORMAL && hit.prop.is_none() {
            return None;
        }
        let depth = (hit.point - probe).dot(hit.normal);
        if depth >= MAX_DEPTH {
            log::debug!("ignored, too deep ({depth:.2} m): prop {:?} info {:?}", hit.prop, hit.info);
        }
        (depth > 1e-4 && depth < MAX_DEPTH).then_some(WallContact {
            point: hit.point,
            normal: hit.normal,
            depth,
            prop: hit.prop,
            info: hit.info,
        })
    }));
    for c in props(position, rot, half) {
        if c.depth >= MAX_DEPTH {
            log::debug!("ignored, too deep ({:.2} m): prop {:?}", c.depth, c.prop);
            continue;
        }
        found.push(c);
    }
    found.sort_by(|a, b| b.depth.total_cmp(&a.depth));
}

/// What a step against the world did.
#[derive(Debug, Default, Clone, PartialEq)]
pub struct Impact {
    /// Unbounded contact metadata for sound, independent of the visual telemetry budget.
    pub sounds: Vec<CarContact>,
    /// Damage callback data captured before each successful response.
    pub damage: Vec<DamageHit>,
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
            (Some((_, Some(_))), _) => ContactKind::PropMovable,
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

/// What the car does about a contact, for the log.
fn action(c: &WallContact) -> &'static str {
    match ContactPoint::from(c).kind {
        ContactKind::PropMovable => "knocked over (prop hidden for a while, car loses momentum)",
        ContactKind::PropRigid => "wall response (prop stays)",
        ContactKind::Barrier => "wall response (barrier)",
        ContactKind::Face => "wall response (steep face)",
    }
}

/// Resolve contacts on `vehicle`. Rigid contacts react with the wall friction and restitution and the
/// deepest pushes the body out; a light prop only costs the car the momentum it takes to push it away.
#[cfg(test)]
pub fn resolve(vehicle: &mut Vehicle, cast: Cast<'_>, props: PropQuery<'_>, walls: &WallSpec) -> Impact {
    resolve_into(vehicle, cast, props, walls, &mut Vec::new())
}

/// As `resolve`, with the contact list built in `scratch` (cleared first) so a caller that keeps the buffer does not
/// allocate per step.
pub fn resolve_into(
    vehicle: &mut Vehicle,
    cast: Cast<'_>,
    props: PropQuery<'_>,
    walls: &WallSpec,
    scratch: &mut Vec<WallContact>,
) -> Impact {
    let body = vehicle.body();
    let half = body.dimension() + body.spec().collision_box_pad;
    let (position, rot, car_mass) = (body.position, body.rotation(), body.mass());
    find_into(cast, props, position, rot, half, scratch);
    let contacts = scratch.as_slice();
    for c in contacts {
        log::debug!(
            "collision {:?} prop {:?} info {:?} at {:?}: {}",
            ContactPoint::from(c).kind,
            c.prop.map(|(id, _)| id),
            c.info,
            c.point,
            action(c),
        );
    }
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
    impact.sounds = contacts
        .iter()
        .map(|c| CarContact {
            sound_profile: 0,
            detached: false,
            source: if c.prop.is_some() { ContactSource::Object } else { ContactSource::World },
            identity: c.prop.map_or_else(
                || c.info.map_or(0, |i| (u64::from(i.section) << 32) | i.instance as u64),
                |(id, _)| u64::from(id),
            ),
            surface: c.info.map_or(0, |i| i.surface),
            point: c.point,
            normal: c.normal,
            velocity: body.point_velocity(c.point),
            delta_v: 0.0,
            front: c.normal.dot(rot.z_axis).abs() > c.normal.dot(rot.x_axis).abs(),
            bottom: false,
        })
        .collect();

    for c in contacts.iter().filter(|c| is_light(c)) {
        if let Some((id, Some(mass))) = c.prop
            && !impact.knocked.iter().any(|&(k, _)| k == id)
        {
            impact.knocked.push((id, mass));
            // Pushing the prop away takes a share of the momentum: m_car / (m_car + m_prop).
            vehicle.body_mut().linear_velocity *= car_mass / (car_mass + mass);
            if let Some(sound) = impact.sounds.iter_mut().find(|s| s.identity == u64::from(id)) {
                sound.delta_v = sound.closing_speed() * mass / (car_mass + mass);
            }
        }
    }

    let Some(deepest) = contacts.iter().find(|c| !is_light(c)).copied() else {
        impact.sounds.retain(|c| c.delta_v > 0.0);
        return impact;
    };
    impact.rigid = contacts.iter().filter(|c| !is_light(c)).count();
    impact.deepest = Some((deepest.point, deepest.info));
    impact.front = deepest.normal.dot(rot.z_axis).abs() > deepest.normal.dot(rot.x_axis).abs();
    for (index, c) in contacts.iter().filter(|c| !is_light(c)).enumerate() {
        let n = c.normal;
        let e = Vec3::new(n.dot(rot.x_axis), n.dot(rot.y_axis), n.dot(rot.z_axis)) * walls.elasticity;
        let plane = PlaneContact {
            point: c.point,
            normal: n,
            friction_static: walls.friction[0],
            friction_kinetic: walls.friction[1].min(walls.friction[0]),
        };
        let params = ContactParams { restitution: e.length(), inertia_scale: walls.moment_scale };
        let damage = DamageHit::capture(
            vehicle.body(),
            c.point,
            n,
            vehicle.body().point_velocity(c.point).length(),
            OtherBody::default(),
        );
        let body_velocity = vehicle.body().linear_velocity;
        let closing_velocity = vehicle.body().point_velocity(c.point);
        let (centre, params) = vehicle.wall_reaction(n, params);
        if let Some(reaction) = vehicle.body_mut().react_plane_at(&plane, &params, Some(centre)) {
            if c.prop.is_none() {
                vehicle.notify_wall_contact(c.point, n, closing_velocity, body_velocity);
            }
            impact.damage.push(damage.resolved(vehicle.body(), false));
            impact.impulse = impact.impulse.max(reaction.impulse);
            if let Some(sound) = impact.sounds.iter_mut().find(|s| s.point == c.point && s.normal == n) {
                sound.delta_v = reaction.impulse / car_mass;
            }
            if let Some(visual) = impact.visuals.get_mut(index) {
                visual.impulse_delta_v = reaction.impulse / car_mass;
            }
            // One reaction per step, as the ground code does: the deepest contact decides.
            break;
        }
    }
    vehicle.body_mut().position += deepest.normal * deepest.depth;
    if impact.impulse > 0.0 {
        vehicle.notify_collision(impact.impulse / car_mass);
    }
    impact.sounds.retain(|c| c.delta_v > 0.0);
    impact
}

#[cfg(test)]
mod regression_tests;
#[cfg(test)]
mod visual_tests;

#[cfg(test)]
mod steering_tests;
#[cfg(test)]
mod tests;

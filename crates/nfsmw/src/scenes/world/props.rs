//! Resident scenery with persistent, fixed-step rigid bodies for original physics placements.

mod body;
mod contact;
mod fragments;
mod ground;
mod lifetime;
mod obb;
mod original_camera;
mod placed;
pub(super) mod scenery;
#[cfg(test)]
mod tests;

use body::PropBody;
pub use fragments::FragmentSpawn;
use obb::obb_overlap;
pub use obb::{Obb, PropOverlap};
pub use placed::PlacedSpawn;

use super::drive::walls::{ContactKind, ContactPoint, Impact};
use super::events::{DamageHit, OtherBody};
use super::object_geometry::{BodyGeometryFrame, primitive_at};
use super::{resident::Placed, space};
use crate::audio::{CarContact, ContactSource};
use blackbox_scene::Aabb;
use blackbox_vehicle::{
    Ground, Vehicle,
    rigid_body::{BodyState, GroundContact, Separation},
};
use glam::{Mat4, Vec3};
use nfsmw_data::world::{PropKind, PropShape};
use std::collections::HashMap;
use std::sync::Arc;

const CELL: f32 = 16.0;
type Cell = (i32, i32);
/// Id and cell maps. Every iteration over them is either order-independent or sorted before it affects results.
type FastMap<K, V> = HashMap<K, V, foldhash::fast::FixedState>;

struct Collider {
    collidable: bool,
    shape: Arc<PropShape>,
    transform: Mat4,
    inverse_transform: Mat4,
    original_bounds: Aabb,
    body: Option<PropBody>,
    cells: Vec<Cell>,
}

impl Collider {
    /// Broad phase covers the collision geometry even when visual bounds are smaller.
    fn physics_bounds(&self) -> Aabb {
        let Some(body) = &self.body else {
            return self.shape.boxes.iter().fold(Aabb::EMPTY, |bounds, b| {
                let m = self.transform * Mat4::from_rotation_translation(b.rotation, b.centre);
                bounds.union(&Aabb::new(-b.half, b.half).transformed(&m))
            });
        };
        let mut bounds = Aabb::EMPTY;
        for primitive in &body.geometry.primitives {
            for b in [body.obb(primitive), primitive_at(primitive, body.previous)] {
                let m = Mat4::from_mat3_translation(b.axes, b.centre);
                let physics = Aabb::new(-b.half, b.half).transformed(&m);
                bounds.extend(space::to_render(physics.min.to_array()));
                bounds.extend(space::to_render(physics.max.to_array()));
            }
        }
        for point in body.geometry.ground_meshes.iter().flat_map(|m| &m.points) {
            bounds.extend(space::to_render((body.rigid.position + body.rigid.rotation() * point).to_array()));
        }
        bounds
    }

    fn pose(&self, alpha: f32) -> (Mat4, Aabb) {
        let Some(body) = &self.body else {
            return (self.transform, self.original_bounds);
        };
        let transform = body.transform(alpha);
        (transform, self.original_bounds.transformed(&(transform * self.inverse_transform)))
    }
}

/// Seconds a knocked-over prop stays gone (hidden) after the car hits it.
pub const REGENERATE_AFTER: f32 = 6.0;

#[derive(Default)]
pub struct PropWorld {
    colliders: FastMap<u32, Collider>,
    /// Seconds left before a knocked-over prop is drawn again.
    knocked: FastMap<u32, f32>,
    grid: FastMap<Cell, Vec<u32>>,
    tiles: HashMap<usize, Vec<u32>>,
    next_id: u32,
    alpha: f32,
    fragments: std::collections::BTreeMap<u32, fragments::FragmentVisual>,
    scenery: scenery::SceneryEvents,
    /// Ids whose lifetime may be active, ascending and unique; `update_lifetimes` prunes the ones that are not.
    live_lifetimes: Vec<u32>,
    /// Per-tick scratch lists, kept across ticks so a steady world tick does not allocate.
    active_scratch: Vec<u32>,
    pair_scratch: Vec<(u32, u32)>,
    lifetime_scratch: Vec<u32>,
    candidate_scratch: Vec<u32>,
    contact_scratch: Vec<GroundContact>,
}

/// The grid cells under `bounds`, x-major, without allocating.
fn cells(bounds: &Aabb) -> impl Iterator<Item = Cell> {
    let cell = |v: f32| (v / CELL).floor() as i32;
    let (x0, x1) = (cell(bounds.min.x), cell(bounds.max.x));
    let (y0, y1) = (cell(bounds.min.y), cell(bounds.max.y));
    (x0..=x1).flat_map(move |x| (y0..=y1).map(move |y| (x, y)))
}

impl PropWorld {
    pub fn add_tile(&mut self, tile: usize, placed: &mut [Placed]) {
        self.remove_tile(tile);
        let mut ids = Vec::new();
        for p in placed {
            let Some(shape) = p.prop.clone() else { continue };
            self.next_id += 1;
            // Stable original IDs fix contact order independently of tile completion order.
            let id = p.spawner.map_or(self.next_id | 0x8000_0000, |s| s.unique_id + 1);
            if self.colliders.contains_key(&id) {
                continue;
            }
            let body = match (&p.spawner, &shape.bounds, &shape.dynamics) {
                (Some(spawn), Some(bounds), Some(spec)) => PropBody::new(spawn, bounds, spec, p.transform),
                _ => None,
            };
            if p.spawner.is_some() && body.is_none() {
                continue;
            }
            p.prop_id = id;
            if let (Some(bounds), Some(spec)) = (&shape.bounds, &shape.dynamics) {
                self.scenery.models.insert(id, scenery::SceneryModel::placed(p.parts.clone(), bounds.clone(), spec));
            }
            self.colliders.insert(
                id,
                Collider {
                    collidable: true,
                    shape,
                    transform: p.transform,
                    inverse_transform: p.transform.inverse(),
                    original_bounds: p.bounds,
                    body,
                    cells: Vec::new(),
                },
            );
            self.regrid(id);
            ids.push(id);
        }
        self.tiles.insert(tile, ids);
    }

    pub fn remove_tile(&mut self, tile: usize) {
        let Some(ids) = self.tiles.remove(&tile) else { return };
        for id in ids {
            self.scenery.models.remove(&id);
            let Some(c) = self.colliders.remove(&id) else { continue };
            self.remove_cells(id, &c.cells);
        }
        self.remove_orphaned_fragments();
    }

    fn remove_cells(&mut self, id: u32, old: &[Cell]) {
        for cell in old {
            if let Some(ids) = self.grid.get_mut(cell) {
                ids.retain(|&n| n != id);
            }
            if self.grid.get(cell).is_some_and(Vec::is_empty) {
                self.grid.remove(cell);
            }
        }
    }

    fn regrid(&mut self, id: u32) {
        let Some(c) = self.colliders.get(&id) else { return };
        let bounds = match !c.collidable || c.body.as_ref().is_some_and(|b| b.lifetime.sinking) {
            true => None,
            false => Some(c.physics_bounds()),
        };
        // An unchanged footprint is compared in place, so a body that stays in its cells does not allocate.
        let unchanged = match &bounds {
            Some(bounds) => cells(bounds).eq(c.cells.iter().copied()),
            None => c.cells.is_empty(),
        };
        if unchanged {
            return;
        }
        let new: Vec<Cell> = bounds.as_ref().map_or_else(Vec::new, |b| cells(b).collect());
        let Some(c) = self.colliders.get_mut(&id) else { return };
        let old = std::mem::replace(&mut c.cells, new);
        self.remove_cells(id, &old);
        for &cell in &self.colliders[&id].cells {
            self.grid.entry(cell).or_default().push(id);
        }
    }

    /// Fills `out` with the ids in the cells under `bounds`: ascending and unique, like the set it replaces.
    fn candidates(&self, bounds: &Aabb, out: &mut Vec<u32>) {
        out.clear();
        // A knocked-over prop is hidden, so nothing hits it until it is back.
        for cell in cells(bounds) {
            let Some(ids) = self.grid.get(&cell) else { continue };
            out.extend(ids.iter().copied().filter(|id| !self.hidden(*id)));
        }
        out.sort_unstable();
        out.dedup();
    }

    pub fn len(&self) -> usize {
        self.colliders.len()
    }
    /// Whether the prop is knocked over and neither drawn nor hit.
    pub fn hidden(&self, id: u32) -> bool {
        self.knocked.contains_key(&id)
    }

    pub fn knocked_count(&self) -> usize {
        self.colliders.values().filter(|c| c.body.as_ref().is_some_and(|b| b.moved)).count()
    }
    pub fn set_alpha(&mut self, alpha: f32) {
        self.alpha = alpha;
    }

    /// The scenery name of prop `id`, for the log.
    pub fn name(&self, id: u32) -> Option<&str> {
        self.colliders.get(&id).map(|c| c.shape.name.as_str())
    }

    /// Knock a prop over; it comes back after [`REGENERATE_AFTER`] seconds.
    pub fn knock(&mut self, id: u32) {
        self.knocked.insert(id, REGENERATE_AFTER);
    }

    /// Dynamic visual pose and bounds, interpolated on the same clock as cars.
    pub fn pose(&self, id: u32) -> Option<(Mat4, Aabb)> {
        Some(self.colliders.get(&id)?.pose(self.alpha))
    }

    pub fn near(&self, at: Vec3, radius: f32) -> Vec<(String, PropKind, Vec3, f32)> {
        let mut out: Vec<_> = self
            .colliders
            .values()
            .map(|c| {
                let p = c.pose(1.0).0.transform_point3(Vec3::ZERO);
                (c.shape.name.clone(), c.shape.kind, p, (p.x - at.x).hypot(p.y - at.y))
            })
            .filter(|r| r.3 <= radius)
            .collect();
        out.sort_by(|a, b| a.3.total_cmp(&b.3));
        out
    }

    /// Only scenery without a physics spawner participates in the legacy static-wall path.
    pub fn overlaps(&self, car: &Obb) -> Vec<PropOverlap> {
        let reach = car.half.length();
        let bounds = Aabb::new((car.centre - reach).to_array(), (car.centre + reach).to_array());
        // `&self` leaves no scratch to fill, so an id in several cells is tested once per cell and the
        // copies are dropped after sorting by id; each copy is identical, so the result is the same.
        let mut out: Vec<PropOverlap> = Vec::new();
        for cell in cells(&bounds) {
            let Some(ids) = self.grid.get(&cell) else { continue };
            for &id in ids {
                // A knocked-over prop is hidden, so nothing hits it until it is back.
                if self.hidden(id) {
                    continue;
                }
                out.extend(self.overlap(id, car));
            }
        }
        out.sort_unstable_by_key(|o| o.id);
        out.dedup_by_key(|o| o.id);
        out
    }

    /// The overlap of the car's box with scenery prop `id`, if it is static and the box really touches it.
    fn overlap(&self, id: u32, car: &Obb) -> Option<PropOverlap> {
        let c = self.colliders.get(&id)?;
        if c.body.is_some() {
            return None;
        }
        let deepest = c
            .shape
            .boxes
            .iter()
            .filter_map(|b| {
                let m = c.transform * Mat4::from_rotation_translation(b.rotation, b.centre);
                obb_overlap(car, &Obb::from_matrix(&m, b.half))
            })
            .max_by(|a, b| a.1.total_cmp(&b.1));
        let (normal, depth) = deepest?;
        Some(PropOverlap { id, kind: c.shape.kind, normal, depth })
    }

    /// One world tick; fallen props keep their render/collision identity.
    pub fn step(&mut self, ground: &dyn Ground, dt: f32) {
        let _span = bevy_log::info_span!("world::props_step").entered();
        self.knocked.retain(|_, left| {
            *left -= dt;
            *left > 0.0
        });
        self.remove_orphaned_fragments();
        self.advance_explosions(dt);
        let mut active = std::mem::take(&mut self.active_scratch);
        active.clear();
        for (&id, c) in &mut self.colliders {
            let Some(b) = c.body.as_mut() else { continue };
            b.previous = (b.rigid.position, b.rigid.orientation);
            b.object_contact = false;
            if b.rigid.state != BodyState::Awake || b.attached || b.lifetime.sinking {
                continue;
            }
            ground::step(b, ground, dt, &mut self.contact_scratch);
            active.push(id);
        }
        active.sort_unstable();
        for &id in &active {
            self.regrid(id);
        }
        self.collide_props(&active);
        self.active_scratch = active;
    }

    fn collide_props(&mut self, active: &[u32]) {
        // Sorted and deduplicated, so the pair order is the same as the BTreeSet it replaces.
        let mut pairs = std::mem::take(&mut self.pair_scratch);
        let mut others = std::mem::take(&mut self.candidate_scratch);
        pairs.clear();
        for &id in active {
            let bounds = self.colliders[&id].physics_bounds();
            self.candidates(&bounds, &mut others);
            for &other in &others {
                if id == other {
                    continue;
                }
                pairs.push((id.min(other), id.max(other)));
            }
        }
        self.candidate_scratch = others;
        pairs.sort_unstable();
        pairs.dedup();
        for &(a, b) in &pairs {
            let (Some(pa), Some(pb)) = (&self.colliders[&a].body, &self.colliders[&b].body) else { continue };
            let counts =
                [contact::shape_count(Some(pa.geometry_frame())), contact::shape_count(Some(pb.geometry_frame()))];
            for i in 0..counts[0] {
                for j in 0..counts[1] {
                    let [Some(ca), Some(cb)] = self.colliders.get_disjoint_mut([&a, &b]) else { continue };
                    let (Some(pa), Some(pb)) = (&mut ca.body, &mut cb.body) else { continue };
                    let (Some(sa), Some(sb)) = (&ca.shape.dynamics, &cb.shape.dynamics) else { continue };
                    let first = contact::shape(&pa.rigid, Some(pa.geometry_frame()), i).unwrap();
                    let second = contact::shape(&pb.rigid, Some(pb.geometry_frame()), j).unwrap();
                    let Some(c) = first.contact(&second) else { continue };
                    let attached = [pa.attached, pb.attached];
                    let closing = (pa.rigid.point_velocity(c.point) - pb.rigid.point_velocity(c.point)).length();
                    let ha = contact::parameters(&pa.rigid, c.normal, sa, pa.attached);
                    let hb = contact::parameters(&pb.rigid, -c.normal, sb, pb.attached);
                    let response = contact::resolve(&mut pa.rigid, &mut pb.rigid, c, ha, hb, Separation::NormalSpeed);
                    let Some(response) = response else { continue };
                    pa.lifetime.activate(sa);
                    pb.lifetime.activate(sb);
                    lifetime::track_lifetime(&mut self.live_lifetimes, a);
                    lifetime::track_lifetime(&mut self.live_lifetimes, b);
                    pa.object_contact = true;
                    pb.object_contact = true;
                    let [ia, ib] = response.immobile;
                    pa.attached &= ia;
                    pb.attached &= ib;
                    pa.moved |= !ia;
                    pb.moved |= !ib;
                    self.contact_event(a, attached[0], closing);
                    self.contact_event(b, attached[1], closing);
                }
            }
            self.regrid(a);
            self.regrid(b);
        }
        self.pair_scratch = pairs;
    }

    /// A vehicle transfers impulses to resident bodies, including previously fallen ones.
    pub fn collide_vehicle(
        &mut self,
        vehicle: &mut Vehicle,
        geometry: Option<BodyGeometryFrame<'_>>,
        impact: &mut Impact,
    ) {
        let centre = space::to_render(vehicle.position().to_array());
        let radius = geometry.map_or(vehicle.body().radius(), |g| {
            g.geometry.primitives.iter().map(|p| p.centre.length() + p.half.length()).fold(0.0_f32, f32::max)
        });
        let previous = geometry.map_or(centre, |g| space::to_render(g.previous.0.to_array()));
        let bounds = Aabb::new(centre.min(previous) - radius, centre.max(previous) + radius);
        let mut ids = std::mem::take(&mut self.candidate_scratch);
        self.candidates(&bounds, &mut ids);
        for &id in &ids {
            let Some(p) = self.colliders[&id].body.as_ref() else { continue };
            let counts = [contact::shape_count(geometry), contact::shape_count(Some(p.geometry_frame()))];
            for i in 0..counts[0] {
                for j in 0..counts[1] {
                    let c = self.colliders.get_mut(&id).unwrap();
                    let Some(p) = c.body.as_mut() else { continue };
                    let Some(spec) = &c.shape.dynamics else { continue };
                    // Separation updates both prepared centres before the next primitive pair.
                    let first = contact::shape(vehicle.body(), geometry, i).unwrap();
                    let second = contact::shape(&p.rigid, Some(p.geometry_frame()), j).unwrap();
                    let Some(hit) = first.contact(&second) else { continue };
                    let _surface = second.surface;
                    let velocity = vehicle.body().point_velocity(hit.point);
                    let was_attached = p.attached;
                    let closing = (velocity - p.rigid.point_velocity(hit.point)).length();
                    let damage = DamageHit::capture(
                        vehicle.body(),
                        hit.point,
                        hit.normal,
                        closing,
                        OtherBody { no_car_effect: spec.no_car_effect, ..Default::default() },
                    );
                    let mut car = contact::material(vehicle.body(), hit.normal);
                    car.immobile = spec.no_car_effect && !p.attached && !spec.immobile;
                    vehicle.modify_object_hit(&mut car);
                    let prop = contact::parameters(&p.rigid, -hit.normal, spec, p.attached);
                    let relative = vehicle.body().point_velocity(hit.point) - p.rigid.point_velocity(hit.point);
                    let rot = vehicle.body().rotation();
                    let sound = CarContact {
                        sound_profile: 0,
                        detached: false,
                        source: ContactSource::Object,
                        identity: u64::from(id),
                        surface: second.surface,
                        point: hit.point,
                        normal: hit.normal,
                        velocity: relative,
                        delta_v: 0.0,
                        front: hit.normal.dot(rot.z_axis).abs() > hit.normal.dot(rot.x_axis).abs(),
                        bottom: false,
                    };
                    let response = contact::resolve(vehicle.body_mut(), &mut p.rigid, hit, car, prop, Separation::Mass);
                    let Some(response) = response else { continue };
                    p.lifetime.activate(spec);
                    lifetime::track_lifetime(&mut self.live_lifetimes, id);
                    p.object_contact = true;
                    let prop_immobile = response.immobile[1];
                    impact.damage.push(damage.resolved(vehicle.body(), true));
                    impact.smacked.push((id, spec.cost_to_state));
                    p.attached &= prop_immobile;
                    p.moved |= !prop_immobile;
                    let delta_v = response.impulse_delta_v[0];
                    impact.sounds.push(CarContact { delta_v, ..sound });
                    if spec.sound_profile != 0 {
                        impact.sounds.push(CarContact {
                            sound_profile: spec.sound_profile,
                            detached: was_attached && !p.attached,
                            surface: first.surface,
                            normal: -hit.normal,
                            velocity: -relative,
                            delta_v: response.impulse_delta_v[1],
                            front: false,
                            ..sound
                        });
                    }
                    let kind = match prop_immobile {
                        true => ContactKind::PropRigid,
                        false => ContactKind::PropMovable,
                    };
                    impact.impulse = impact.impulse.max(delta_v * vehicle.body().mass());
                    impact.contacts.push(ContactPoint { point: hit.point, normal: hit.normal, kind });
                    if delta_v > 0.0 {
                        vehicle.notify_collision(delta_v);
                    }
                    self.contact_event(id, was_attached, closing);
                }
            }
            self.regrid(id);
        }
        self.candidate_scratch = ids;
    }
}

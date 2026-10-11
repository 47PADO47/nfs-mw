//! Resident scenery with persistent, fixed-step rigid bodies for original physics placements.

mod body;
mod contact;
mod ground;
mod obb;
#[cfg(test)]
mod tests;

use body::PropBody;
use obb::obb_overlap;
pub use obb::{Obb, PropOverlap};

use super::drive::walls::{ContactKind, ContactPoint, Impact};
use super::{resident::Placed, space};
use blackbox_scene::Aabb;
use blackbox_vehicle::{Ground, Vehicle, rigid_body::BodyState};
use glam::{Mat4, Vec3};
use nfsmw_data::world::{PropKind, PropShape};
use std::collections::{BTreeSet, HashMap};
use std::sync::Arc;

const CELL: f32 = 16.0;
type Cell = (i32, i32);

struct Collider {
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
        for primitive in &body.primitives {
            let b = body.obb(primitive);
            let m = Mat4::from_mat3_translation(b.axes, b.centre);
            let physics = Aabb::new(-b.half, b.half).transformed(&m);
            bounds.extend(space::to_render(physics.min.to_array()));
            bounds.extend(space::to_render(physics.max.to_array()));
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

#[derive(Default)]
pub struct PropWorld {
    colliders: HashMap<u32, Collider>,
    grid: HashMap<Cell, Vec<u32>>,
    tiles: HashMap<usize, Vec<u32>>,
    next_id: u32,
    alpha: f32,
}

fn cells(bounds: &Aabb) -> Vec<Cell> {
    let cell = |v: f32| (v / CELL).floor() as i32;
    let mut out = Vec::new();
    for x in cell(bounds.min.x)..=cell(bounds.max.x) {
        for y in cell(bounds.min.y)..=cell(bounds.max.y) {
            out.push((x, y));
        }
    }
    out
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
            self.colliders.insert(
                id,
                Collider {
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
            let Some(c) = self.colliders.remove(&id) else { continue };
            self.remove_cells(id, &c.cells);
        }
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
        let Some(c) = self.colliders.get_mut(&id) else { return };
        let new = cells(&c.physics_bounds());
        if new == c.cells {
            return;
        }
        let old = std::mem::replace(&mut c.cells, new.clone());
        self.remove_cells(id, &old);
        for cell in new {
            self.grid.entry(cell).or_default().push(id);
        }
    }

    fn candidates(&self, bounds: &Aabb) -> BTreeSet<u32> {
        cells(bounds).into_iter().filter_map(|cell| self.grid.get(&cell)).flatten().copied().collect()
    }

    pub fn len(&self) -> usize {
        self.colliders.len()
    }
    pub fn knocked_count(&self) -> usize {
        self.colliders.values().filter(|c| c.body.as_ref().is_some_and(|b| b.moved)).count()
    }
    pub fn set_alpha(&mut self, alpha: f32) {
        self.alpha = alpha;
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
        self.candidates(&bounds)
            .into_iter()
            .filter_map(|id| {
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
            })
            .collect()
    }

    /// One world tick; fallen props keep their render/collision identity.
    pub fn step(&mut self, ground: &dyn Ground, dt: f32) {
        let mut active = Vec::new();
        for (&id, c) in &mut self.colliders {
            let Some(b) = c.body.as_mut() else { continue };
            b.previous = (b.rigid.position, b.rigid.orientation);
            if b.rigid.state != BodyState::Awake || b.attached {
                continue;
            }
            ground::step(b, ground, dt);
            active.push(id);
        }
        active.sort_unstable();
        for &id in &active {
            self.regrid(id);
        }
        self.collide_props(&active);
    }

    fn collide_props(&mut self, active: &[u32]) {
        let mut pairs = BTreeSet::new();
        for &id in active {
            let bounds = self.colliders[&id].physics_bounds();
            for other in self.candidates(&bounds) {
                if id == other {
                    continue;
                }
                pairs.insert((id.min(other), id.max(other)));
            }
        }
        for (a, b) in pairs {
            let [Some(ca), Some(cb)] = self.colliders.get_disjoint_mut([&a, &b]) else { continue };
            let (Some(pa), Some(pb)) = (&mut ca.body, &mut cb.body) else { continue };
            let (Some(sa), Some(sb)) = (&ca.shape.dynamics, &cb.shape.dynamics) else { continue };
            let Some(c) = contact::prop_contact(pa, pb) else { continue };
            let ha = contact::parameters(&pa.rigid, c.normal, sa, pa.attached);
            let hb = contact::parameters(&pb.rigid, -c.normal, sb, pb.attached);
            if let Some((_, ia, ib)) = contact::resolve(&mut pa.rigid, &mut pb.rigid, c, ha, hb) {
                pa.attached &= ia;
                pb.attached &= ib;
                pa.moved |= !ia;
                pb.moved |= !ib;
            }
            self.regrid(a);
            self.regrid(b);
        }
    }

    /// A vehicle transfers impulses to resident bodies, including previously fallen ones.
    pub fn collide_vehicle(&mut self, vehicle: &mut Vehicle, impact: &mut Impact) {
        let centre = space::to_render(vehicle.position().to_array());
        let radius = vehicle.body().radius();
        let bounds = Aabb::new((centre - radius).to_array(), (centre + radius).to_array());
        for id in self.candidates(&bounds) {
            let c = self.colliders.get_mut(&id).unwrap();
            let Some(p) = c.body.as_mut() else { continue };
            let Some(spec) = &c.shape.dynamics else { continue };
            let Some((hit, _surface)) = contact::car_contact(vehicle.body(), p) else { continue };
            let mut car = contact::material(vehicle.body(), hit.normal);
            car.immobile = spec.no_car_effect && !p.attached && !spec.immobile;
            let prop = contact::parameters(&p.rigid, -hit.normal, spec, p.attached);
            let response = contact::resolve(vehicle.body_mut(), &mut p.rigid, hit, car, prop);
            if let Some((impulse, car_immobile, prop_immobile)) = response {
                p.attached &= prop_immobile;
                p.moved |= !prop_immobile;
                let delta_v = match car_immobile {
                    true => 0.0,
                    false => impulse / vehicle.body().mass(),
                };
                impact.impulse = impact.impulse.max(delta_v * vehicle.body().mass());
                impact.contacts.push(ContactPoint {
                    point: hit.point,
                    normal: hit.normal,
                    kind: ContactKind::PropLight,
                });
                if delta_v > 0.0 {
                    vehicle.notify_collision(impulse);
                }
            }
            self.regrid(id);
        }
    }
}

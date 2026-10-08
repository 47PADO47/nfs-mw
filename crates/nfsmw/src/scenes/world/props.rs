//! Props that are in the way: the collision boxes of scenery objects (cones, bins, benches, poles,
//! fences) placed with their instances, with the loose ones knocked aside for a few seconds when a car
//! hits them. All in render space.

use std::collections::HashMap;
use std::sync::Arc;

use glam::{Mat4, Vec3};
use nfsmw_data::world::{PropKind, PropShape};

use super::resident::Placed;

/// Side of a cell of the lookup grid, metres.
const CELL: f32 = 16.0;
/// Seconds a knocked-over prop stays gone.
pub const REGENERATE_AFTER: f32 = 6.0;

struct Collider {
    shape: Arc<PropShape>,
    /// Model space to the world.
    transform: Mat4,
}

/// An oriented box: centre, three orthonormal axes and the half extent along each.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Obb {
    pub centre: Vec3,
    pub axes: [Vec3; 3],
    pub half: Vec3,
}

impl Obb {
    /// The box with local half extents `half` placed by `to_world` (its axes may be scaled).
    fn from_matrix(to_world: &Mat4, half: Vec3) -> Self {
        let col = |m: glam::Vec4| m.truncate();
        let axes = [col(to_world.x_axis), col(to_world.y_axis), col(to_world.z_axis)];
        let lengths = Vec3::new(axes[0].length(), axes[1].length(), axes[2].length());
        Self { centre: col(to_world.w_axis), axes: axes.map(|a| a.normalize_or_zero()), half: half * lengths }
    }

    /// How far the box reaches along the unit vector `l` from its centre.
    pub fn support(&self, l: Vec3) -> f32 {
        (0..3).map(|i| self.half[i] * self.axes[i].dot(l).abs()).sum()
    }
}

/// A prop the car's box overlaps.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PropOverlap {
    pub id: u32,
    pub kind: PropKind,
    /// Unit axis of least overlap, from the prop towards the car.
    pub normal: Vec3,
    pub depth: f32,
}

/// Whether two oriented boxes overlap (separating axis test): the axis of least overlap, pointing
/// from `b` to `a`, and the overlap along it.
fn obb_overlap(a: &Obb, b: &Obb) -> Option<(Vec3, f32)> {
    let mut best: Option<(Vec3, f32)> = None;
    let offset = a.centre - b.centre;
    let mut test = |axis: Vec3| -> bool {
        let len = axis.length();
        if len < 1e-5 {
            return true; // parallel edges: no axis
        }
        let l = axis / len;
        let overlap = a.support(l) + b.support(l) - offset.dot(l).abs();
        if overlap < 0.0 {
            return false;
        }
        if best.is_none_or(|(_, d)| overlap < d) {
            best = Some((if offset.dot(l) < 0.0 { -l } else { l }, overlap));
        }
        true
    };
    for i in 0..3 {
        if !test(a.axes[i]) || !test(b.axes[i]) {
            return None;
        }
    }
    for i in 0..3 {
        for j in 0..3 {
            if !test(a.axes[i].cross(b.axes[j])) {
                return None;
            }
        }
    }
    best
}

#[derive(Default)]
pub struct PropWorld {
    colliders: HashMap<u32, Collider>,
    grid: HashMap<(i32, i32), Vec<u32>>,
    /// The props each tile put in, to take out again when the tile goes.
    tiles: HashMap<usize, Vec<u32>>,
    /// Seconds left before a knocked-over prop is back.
    knocked: HashMap<u32, f32>,
    next_id: u32,
}

fn cell_of(p: Vec3) -> (i32, i32) {
    ((p.x / CELL).floor() as i32, (p.y / CELL).floor() as i32)
}

impl PropWorld {
    /// Register the props of `placed` (a tile's instances), giving each an id.
    pub fn add_tile(&mut self, tile: usize, placed: &mut [Placed]) {
        let mut ids = Vec::new();
        for p in placed.iter_mut() {
            let Some(shape) = p.prop.clone() else { continue };
            self.next_id += 1;
            p.prop_id = self.next_id;
            // The cells the instance's box touches.
            let (lo, hi) = (cell_of(p.bounds.min - 1.5), cell_of(p.bounds.max + 1.5));
            for cx in lo.0..=hi.0 {
                for cy in lo.1..=hi.1 {
                    self.grid.entry((cx, cy)).or_default().push(self.next_id);
                }
            }
            self.colliders.insert(self.next_id, Collider { shape, transform: p.transform });
            ids.push(self.next_id);
        }
        if !ids.is_empty() {
            log::debug!("tile {tile}: {} props", ids.len());
            self.tiles.insert(tile, ids);
        }
    }

    pub fn remove_tile(&mut self, tile: usize) {
        let Some(ids) = self.tiles.remove(&tile) else { return };
        for id in &ids {
            self.colliders.remove(id);
            self.knocked.remove(id);
        }
        // Grid entries of removed props are skipped when looked up; drop them when a cell is walked.
    }

    pub fn len(&self) -> usize {
        self.colliders.len()
    }

    /// Whether the prop is knocked over and not drawn or hit.
    pub fn hidden(&self, id: u32) -> bool {
        self.knocked.contains_key(&id)
    }

    /// Knock a prop over; it comes back after [`REGENERATE_AFTER`] seconds.
    pub fn knock(&mut self, id: u32) {
        self.knocked.insert(id, REGENERATE_AFTER);
    }

    /// Let time pass: knocked props that have waited long enough come back.
    pub fn tick(&mut self, dt: f32) {
        self.knocked.retain(|_, left| {
            *left -= dt;
            *left > 0.0
        });
    }

    /// The props within `radius` metres of `at` (map x, y), nearest first: scenery name, kind, position
    /// and distance. For the `props` console command.
    pub fn near(&self, at: Vec3, radius: f32) -> Vec<(String, PropKind, Vec3, f32)> {
        let mut out: Vec<_> = self
            .colliders
            .iter()
            .filter(|(id, _)| !self.knocked.contains_key(id))
            .map(|(_, c)| (c.shape.name.clone(), c.shape.kind, c.transform.transform_point3(Vec3::ZERO)))
            .map(|(name, kind, p)| (name, kind, p, (p.x - at.x).hypot(p.y - at.y)))
            .filter(|t| t.3 <= radius)
            .collect();
        out.sort_by(|a, b| a.3.total_cmp(&b.3));
        out
    }

    /// The props whose boxes overlap the oriented box `car` (render space), not counting knocked ones,
    /// with the way out of each: the axis of least overlap, pointing from the prop towards the car.
    pub fn overlaps(&self, car: &Obb) -> Vec<PropOverlap> {
        let reach = car.half.length();
        let (lo, hi) = (cell_of(car.centre - reach), cell_of(car.centre + reach));
        let mut found: Vec<PropOverlap> = Vec::new();
        for cx in lo.0..=hi.0 {
            for cy in lo.1..=hi.1 {
                let Some(ids) = self.grid.get(&(cx, cy)) else { continue };
                for &id in ids {
                    let Some(c) = self.colliders.get(&id) else { continue };
                    if self.knocked.contains_key(&id) || found.iter().any(|f| f.id == id) {
                        continue;
                    }
                    let deepest = c
                        .shape
                        .boxes
                        .iter()
                        .filter_map(|b| {
                            let w = c.transform * Mat4::from_rotation_translation(b.rotation, b.centre);
                            obb_overlap(car, &Obb::from_matrix(&w, b.half))
                        })
                        .max_by(|a, b| a.1.total_cmp(&b.1));
                    if let Some((normal, depth)) = deepest {
                        found.push(PropOverlap { id, kind: c.shape.kind, normal, depth });
                    }
                }
            }
        }
        found
    }

    pub fn knocked_count(&self) -> usize {
        self.knocked.len()
    }
}

#[cfg(test)]
mod tests {
    use glam::Quat;
    use nfsmw_data::world::LocalBox;

    use super::*;

    fn shape(kind: PropKind) -> Arc<PropShape> {
        Arc::new(PropShape {
            name: String::new(),
            boxes: vec![LocalBox {
                centre: Vec3::new(0.0, 0.0, 0.5),
                rotation: Quat::IDENTITY,
                half: Vec3::new(0.5, 0.5, 0.5),
            }],
            kind,
        })
    }

    fn placed(at: Vec3, kind: PropKind) -> Placed {
        Placed::prop_only(Mat4::from_translation(at), shape(kind))
    }

    fn car_box(centre: Vec3) -> Obb {
        Obb { centre, axes: [Vec3::X, Vec3::Y, Vec3::Z], half: Vec3::new(2.2, 0.9, 0.6) }
    }

    #[test]
    fn a_car_box_overlapping_a_prop_gets_the_way_out() {
        let mut world = PropWorld::default();
        let mut tile = [placed(Vec3::new(10.0, 0.0, 0.0), PropKind::Rigid)];
        world.add_tile(0, &mut tile);
        // The prop is a 1 m cube from x = 9.5 to 10.5. A car 4.4 m long centred at 8 reaches 10.2.
        let hits = world.overlaps(&car_box(Vec3::new(8.0, 0.0, 0.7)));
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].normal, Vec3::NEG_X, "pushed back the way it came");
        assert!((hits[0].depth - 0.7).abs() < 1e-4, "{}", hits[0].depth);
        // Not touching, passing beside it, or knocked over: nothing.
        assert!(world.overlaps(&car_box(Vec3::new(6.0, 0.0, 0.7))).is_empty());
        assert!(world.overlaps(&car_box(Vec3::new(10.0, 3.0, 0.7))).is_empty());
        world.knock(tile[0].prop_id);
        assert!(world.overlaps(&car_box(Vec3::new(8.0, 0.0, 0.7))).is_empty());
    }

    #[test]
    fn a_thin_prop_between_two_probe_lines_is_still_found() {
        // The reason for box tests: a 0.2 m post between the middle and the corner of the car's side.
        let mut world = PropWorld::default();
        let post = Arc::new(PropShape {
            name: String::new(),
            boxes: vec![LocalBox { centre: Vec3::ZERO, rotation: Quat::IDENTITY, half: Vec3::new(0.1, 0.1, 1.0) }],
            kind: PropKind::Light { mass: 50.0 },
        });
        let mut tile = [Placed::prop_only(Mat4::from_translation(Vec3::new(0.0, 0.45, 0.0)), post)];
        world.add_tile(0, &mut tile);
        assert_eq!(world.overlaps(&car_box(Vec3::new(0.0, 0.0, 0.7))).len(), 1);
    }

    #[test]
    fn knocked_props_are_gone_for_a_while_and_come_back() {
        let mut world = PropWorld::default();
        let mut tile = [placed(Vec3::ZERO, PropKind::Light { mass: 100.0 })];
        world.add_tile(0, &mut tile);
        let id = tile[0].prop_id;
        let car = car_box(Vec3::new(0.0, 0.0, 0.7));
        assert_eq!(world.overlaps(&car).len(), 1);
        world.knock(id);
        assert!(world.hidden(id) && world.overlaps(&car).is_empty());
        world.tick(REGENERATE_AFTER - 1.0);
        assert!(world.hidden(id));
        world.tick(1.5);
        assert!(!world.hidden(id) && world.overlaps(&car).len() == 1);
    }

    #[test]
    fn a_tile_takes_its_props_away() {
        let mut world = PropWorld::default();
        let mut a = [placed(Vec3::new(0.0, 0.0, 0.0), PropKind::Rigid)];
        let mut b = [placed(Vec3::new(40.0, 0.0, 0.0), PropKind::Rigid)];
        world.add_tile(1, &mut a);
        world.add_tile(2, &mut b);
        assert_eq!(world.len(), 2);
        world.remove_tile(1);
        assert_eq!(world.len(), 1);
        assert!(world.overlaps(&car_box(Vec3::new(0.0, 0.0, 0.7))).is_empty());
        assert_eq!(world.overlaps(&car_box(Vec3::new(40.0, 0.0, 0.7))).len(), 1);
    }

    #[test]
    fn turned_instances_turn_their_boxes() {
        let mut world = PropWorld::default();
        // A long thin box (half 2 x 0.1) turned a quarter turn about z lies along y.
        let long = Arc::new(PropShape {
            name: String::new(),
            boxes: vec![LocalBox { centre: Vec3::ZERO, rotation: Quat::IDENTITY, half: Vec3::new(2.0, 0.1, 0.5) }],
            kind: PropKind::Rigid,
        });
        let mut tile = [Placed::prop_only(Mat4::from_rotation_z(std::f32::consts::FRAC_PI_2), long)];
        world.add_tile(0, &mut tile);
        let small = |centre: Vec3| Obb { centre, axes: [Vec3::X, Vec3::Y, Vec3::Z], half: Vec3::splat(0.3) };
        // Lying along y: something 1.5 m along y touches it, 1.5 m along x does not.
        assert_eq!(world.overlaps(&small(Vec3::new(0.0, 1.5, 0.0))).len(), 1);
        assert!(world.overlaps(&small(Vec3::new(1.5, 0.0, 0.0))).is_empty());
    }
}

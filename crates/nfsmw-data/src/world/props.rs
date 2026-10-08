//! Props: scenery objects (cones, bins, benches, poles, fences) that have collision bounds.
//!
//! The track file holds a `BoundsPack` of 405 sets keyed by an object name hash (`docs/formats/collision.md`,
//! "Bounds"). This module matches them to scenery names and turns each set into oriented boxes in the
//! scenery model's own space (x forward, y left, z up), ready to be placed with an instance's matrix.
//!
//! Whether a prop gives way is not in the bounds. The attribute class `smackable` has a mass per kind of
//! object, but which scenery object is which kind is not in any file we have read, so the kind is guessed
//! from the scenery name ([`smackable_class`]); everything else a car can hit is rigid.

use std::cell::RefCell;
use std::collections::HashMap;
use std::sync::Arc;

use blackbox_attrib::{Database, vlt_hash};
use blackbox_collision::{BoundsSet, Shape, bounds_flags};
use glam::{Mat3, Quat, Vec3};

/// A scenery name in the data is cut to 23 characters, but the bounds are keyed by the full name: the
/// ends the full names seem to have, tried in this order after the name itself.
const NAME_ENDINGS: [&str; 6] = ["0", "00", "_DE", "_DE0", "1", "A"];

/// Light objects weigh at most this (kg): a car pushes them out of the way. Heavier ones are walls.
pub const LIGHT_MASS: f32 = 150.0;

/// An oriented box in a scenery model's space (x forward, y left, z up).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LocalBox {
    pub centre: Vec3,
    pub rotation: Quat,
    pub half: Vec3,
}

/// Whether a prop stands its ground.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PropKind {
    /// A wall for the car.
    Rigid,
    /// Knocked aside by a car: its mass in kilograms.
    Light { mass: f32 },
}

/// The collision of one kind of scenery object.
#[derive(Debug, Clone, PartialEq)]
pub struct PropShape {
    pub boxes: Vec<LocalBox>,
    pub kind: PropKind,
}

/// A bounds-space (physics axes) vector in the model's axes: `(x, y, z) -> (z, -x, y)`.
fn to_model(v: [f32; 3]) -> Vec3 {
    Vec3::new(v[2], -v[0], v[1])
}

/// Bounds space to model space as a matrix: a reflection, since physics space is left-handed.
fn model_from_bounds() -> Mat3 {
    Mat3::from_cols(Vec3::new(0.0, -1.0, 0.0), Vec3::new(0.0, 0.0, 1.0), Vec3::new(1.0, 0.0, 0.0))
}

/// The boxes of a bounds set in model space. Nodes that collide with the world count; a set with none
/// uses its root. Spheres become the cube around them.
pub fn boxes(set: &BoundsSet) -> Vec<LocalBox> {
    let usable = |n: &&blackbox_collision::Bounds| {
        n.flags & bounds_flags::DISABLED == 0 && matches!(n.shape(), Shape::Box | Shape::Sphere)
    };
    let mut nodes: Vec<_> =
        set.nodes.iter().filter(usable).filter(|n| n.flags & bounds_flags::PRIM_VS_WORLD != 0).collect();
    if nodes.is_empty() {
        nodes = set.nodes.iter().take(1).filter(usable).collect();
    }
    let m = model_from_bounds();
    nodes
        .into_iter()
        .map(|n| {
            let [x, y, z, w] = n.orientation;
            let q = Quat::from_xyzw(x, y, z, w);
            let q = if q.length_squared() > 1e-6 { q.normalize() } else { Quat::IDENTITY };
            let rotation = Quat::from_mat3(&(m * Mat3::from_quat(q) * m.transpose()));
            let half = n.half_dimensions;
            let half = if n.shape() == Shape::Sphere { [n.radius; 3] } else { half };
            LocalBox { centre: to_model(n.pivot), rotation, half: Vec3::new(half[2], half[0], half[1]) }
        })
        .collect()
}

/// The `smackable` class a scenery name most likely is (`docs` above): by what the name says.
pub fn smackable_class(name: &str) -> &'static str {
    let n = name.to_ascii_lowercase();
    let has = |words: &[&str]| words.iter().any(|w| n.contains(w));
    match () {
        _ if has(&["cone"]) => "cone",
        _ if has(&["hydrant"]) => "firehydrant",
        _ if has(&["dumpster"]) => "dumpster",
        _ if has(&["garbage", "trash", "recyc"]) => "garbg_can",
        _ if has(&["crashbarrel"]) => "crsh_barrel",
        _ if has(&["barrel", "drum"]) => "oil_drum",
        _ if has(&["bench"]) => "bench",
        _ if has(&["newspaper", "mailbox"]) => "news_box",
        _ if has(&["crate", "boxes", "box"]) => "crate",
        _ if has(&["chair"]) => "chair",
        _ if has(&["table"]) => "table",
        _ if has(&["parkingmeter", "parking_meter"]) => "parking_mtr",
        _ if has(&["trafficlight"]) => "trafficlight",
        _ if has(&["tollbooth", "booth"]) => "tollbooth",
        _ if has(&["popmachine", "vendmach", "machine"]) => "pop_mach",
        _ if has(&["lightpole", "medianpole", "pole", "post"]) => "mediummetalpost",
        _ if has(&["gazebo", "scaffold", "shelter", "kiosk", "wall", "gate"]) => "largemetalobject",
        _ => "default",
    }
}

/// Every prop the track has bounds for.
pub struct PropCatalog {
    sets: HashMap<u32, Arc<BoundsSet>>,
    /// `smackable` class name -> mass.
    masses: HashMap<String, f32>,
    cache: RefCell<HashMap<String, Option<Arc<PropShape>>>>,
}

impl PropCatalog {
    /// `sets` are the track's bounds sets and `db` the gameplay database (for the masses).
    pub fn new(sets: Vec<BoundsSet>, db: &Database) -> Self {
        let masses =
            db.collections_of("smackable").filter_map(|c| Some((c.name()?.to_owned(), c.get_f32("MASS")?))).collect();
        Self { sets: sets.into_iter().map(|s| (s.name_hash, Arc::new(s))).collect(), masses, cache: RefCell::default() }
    }

    pub fn len(&self) -> usize {
        self.sets.len()
    }

    pub fn is_empty(&self) -> bool {
        self.sets.is_empty()
    }

    fn find_set(&self, name: &str) -> Option<&Arc<BoundsSet>> {
        std::iter::once("").chain(NAME_ENDINGS).find_map(|ending| self.sets.get(&vlt_hash(&format!("{name}{ending}"))))
    }

    /// The collision of the scenery object `name`, if it has any. Objects whose names start with `XO_`
    /// are the loose ones; everything else (walls, fences, buildings) is rigid.
    pub fn shape(&self, name: &str) -> Option<Arc<PropShape>> {
        if let Some(known) = self.cache.borrow().get(name) {
            return known.clone();
        }
        let shape = self.find_set(name).map(|set| {
            let boxes = boxes(set);
            let kind = if name.starts_with("XO_") {
                match self.masses.get(smackable_class(name)).copied() {
                    Some(mass) if mass <= LIGHT_MASS => PropKind::Light { mass },
                    _ => PropKind::Rigid,
                }
            } else {
                PropKind::Rigid
            };
            Arc::new(PropShape { boxes, kind })
        });
        let shape = shape.filter(|s| !s.boxes.is_empty());
        self.cache.borrow_mut().insert(name.to_owned(), shape.clone());
        shape
    }
}

#[cfg(test)]
mod tests {
    use blackbox_collision::Bounds;

    use super::*;

    fn node(flags: u16, pivot: [f32; 3], half: [f32; 3]) -> Bounds {
        Bounds {
            orientation: [0.0, 0.0, 0.0, 1.0],
            position: [0.0; 3],
            flags,
            half_dimensions: half,
            child_count: 0,
            point_cloud: None,
            pivot,
            first_child: 0,
            radius: half[0],
            surface: 0,
            name_hash: 0,
        }
    }

    #[test]
    fn bounds_axes_become_model_axes() {
        // Physics axes: 0.3 wide (x), 0.7 tall (y), 0.5 long (z), centred 0.7 up and 0.1 ahead.
        let set = BoundsSet {
            name_hash: 1,
            nodes: vec![node(bounds_flags::BOX | bounds_flags::PRIM_VS_WORLD, [0.0, 0.7, 0.1], [0.3, 0.7, 0.5])],
            point_clouds: vec![],
        };
        let b = boxes(&set);
        assert_eq!(b.len(), 1);
        // Model axes: forward = length, left = width, up = height; the centre 0.1 ahead and 0.7 up.
        assert_eq!(b[0].half, Vec3::new(0.5, 0.3, 0.7));
        assert!((b[0].centre - Vec3::new(0.1, 0.0, 0.7)).length() < 1e-6);
        assert!(b[0].rotation.angle_between(Quat::IDENTITY) < 1e-5);
    }

    #[test]
    fn a_turned_box_turns_the_same_way_in_model_space() {
        // A quarter turn about the physics up axis (y) swaps the box's width and length in the world, and
        // about the model's up axis (z) too, with the opposite sense because the spaces mirror each other.
        let mut n = node(bounds_flags::BOX | bounds_flags::PRIM_VS_WORLD, [0.0; 3], [1.0, 1.0, 1.0]);
        let q = Quat::from_rotation_y(std::f32::consts::FRAC_PI_2);
        n.orientation = [q.x, q.y, q.z, q.w];
        let set = BoundsSet { name_hash: 1, nodes: vec![n], point_clouds: vec![] };
        let r = boxes(&set)[0].rotation;
        // Physics +z (forward) turned about +y by 90 degrees points along +x (right); in model axes that
        // is forward turned to the -y (right) side: the model's forward axis goes to its right.
        let turned_forward = r * Vec3::X;
        assert!((turned_forward - Vec3::NEG_Y).length() < 1e-4, "{turned_forward:?}");
        assert!((r * Vec3::Z - Vec3::Z).length() < 1e-4, "up stays up");
    }

    #[test]
    fn only_world_colliding_nodes_count_else_the_root() {
        let set = BoundsSet {
            name_hash: 1,
            nodes: vec![
                node(bounds_flags::BOX, [0.0; 3], [1.0; 3]),
                node(bounds_flags::BOX | bounds_flags::PRIM_VS_WORLD, [1.0, 0.0, 0.0], [0.5; 3]),
                node(
                    bounds_flags::BOX | bounds_flags::PRIM_VS_WORLD | bounds_flags::DISABLED,
                    [2.0, 0.0, 0.0],
                    [0.5; 3],
                ),
            ],
            point_clouds: vec![],
        };
        let b = boxes(&set);
        assert_eq!(b.len(), 1);
        assert_eq!(b[0].half, Vec3::splat(0.5));
        let only_root =
            BoundsSet { name_hash: 1, nodes: vec![node(bounds_flags::BOX, [0.0; 3], [1.0; 3])], point_clouds: vec![] };
        assert_eq!(boxes(&only_root).len(), 1);
    }

    #[test]
    fn names_say_what_a_thing_is() {
        assert_eq!(smackable_class("XO_TrafficConeA_1b_00"), "cone");
        assert_eq!(smackable_class("XO_CrashBarrelA_1b_00"), "crsh_barrel");
        assert_eq!(smackable_class("XO_OilBarrelA_1b_00"), "oil_drum");
        assert_eq!(smackable_class("XO_ParkBenchA_1b_00"), "bench");
        assert_eq!(smackable_class("XO_LightPoleA_1b_00"), "mediummetalpost");
        assert_eq!(smackable_class("XO_Whatever_1b_00"), "default");
    }
}

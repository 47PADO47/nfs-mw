//! The objects placed on the track (barriers, buildings, props) in the cells: each placement goes to the cell whose anchor is
//! nearest, the cell's section holds the models its placements use, and a solid model also puts a box of its size into the cell's
//! collision, so a bike or a car meets the wall.

use std::collections::{BTreeMap, HashMap};

use blackbox_collision::SoupTriangle;
use blackbox_solid::SolidSource;
use blackbox_tpk::TextureSource;

use crate::cells::{Cell, parts_of};
use crate::collision::{surface_hash, to_physics};
use crate::model::model_solid;
use crate::pack::{Instance, Pack};
use crate::textures::Pictures;

/// The largest component of a placement's rows the game's scenery format holds.
const MAX_COMPONENT: f32 = 4.0;
/// The surface of a wall.
const WALL_SURFACE: &str = "concrete";
/// A box lower than this is not a wall: a cone, a kerb stone (metres).
const MIN_WALL_HEIGHT: f32 = 0.3;

/// The low and high corners of a box.
type Aabb = ([f32; 3], [f32; 3]);

/// One placement of a solid in a cell's section: the solid's name, its position and its rows.
#[derive(Debug, Clone, PartialEq)]
pub struct Placed {
    pub solid: String,
    pub pos: [f32; 3],
    pub rows: [[f32; 3]; 3],
}

/// What a cell holds of the objects.
#[derive(Debug, Default)]
pub struct CellObjects {
    pub solids: Vec<SolidSource>,
    pub textures: Vec<TextureSource>,
    pub places: Vec<Placed>,
    /// Boxes of the solid models, as collision triangles in physics space.
    pub walls: Vec<SoupTriangle>,
    /// Placements left out: a scale beyond what the scenery format holds.
    pub skipped: usize,
}

/// For each cell of `cells`, the indices of the placements of `pack` that belong to it: those nearest to its anchor.
pub fn assign(pack: &Pack, cells: &[Cell]) -> Vec<Vec<usize>> {
    let mut out = vec![Vec::new(); cells.len()];
    if cells.is_empty() {
        return out;
    }
    let centres: Vec<[f32; 2]> = cells
        .iter()
        .map(|cell| {
            let [x, y, _, _] = cell.anchor();
            [x, y]
        })
        .collect();
    for (at, instance) in pack.instances.iter().enumerate() {
        let near = (0..cells.len())
            .min_by(|&a, &b| distance(centres[a], instance.pos).total_cmp(&distance(centres[b], instance.pos)))
            .unwrap_or(0);
        out[near].push(at);
    }
    out
}

fn distance(centre: [f32; 2], pos: [f32; 3]) -> f32 {
    (centre[0] - pos[0]).hypot(centre[1] - pos[1])
}

/// The solids, textures, placements and walls of the placements `indices` of `pack`, with solids named `<key>_M<model>_<part>`.
pub fn cell_objects(pack: &Pack, pictures: &Pictures, indices: &[usize], key: &str) -> CellObjects {
    let mut out = CellObjects::default();
    let mut made: HashMap<u32, Vec<String>> = HashMap::new();
    let mut boxes: HashMap<u32, Option<Aabb>> = HashMap::new();
    for &at in indices {
        let instance = &pack.instances[at];
        if instance.rows.iter().flatten().any(|v| !v.is_finite() || v.abs() > MAX_COMPONENT) {
            out.skipped += 1;
            continue;
        }
        let names =
            made.entry(instance.model).or_insert_with(|| add_model(pack, pictures, instance.model, key, &mut out));
        for name in names.iter() {
            out.places.push(Placed { solid: name.clone(), pos: instance.pos, rows: instance.rows });
        }
        if !pack.models[instance.model as usize].solid {
            continue;
        }
        let bounds = *boxes.entry(instance.model).or_insert_with(|| model_bounds(pack, instance.model));
        if let Some((low, high)) = bounds
            && high[2] - low[2] >= MIN_WALL_HEIGHT
        {
            out.walls.extend(box_triangles(low, high, instance));
        }
    }
    out
}

/// Makes the solids of `model` (one per part of each of its groups) in `out`, and returns their names.
fn add_model(pack: &Pack, pictures: &Pictures, model: u32, key: &str, out: &mut CellObjects) -> Vec<String> {
    let mut names = Vec::new();
    for group in pack.object_groups.iter().filter(|g| g.model == model) {
        let texture = crate::import::texture_for(pack, pictures, group.material);
        for part in parts_of(group) {
            let name = format!("{key}_M{model}_{}", names.len());
            out.solids.push(model_solid(&name, &part, &texture.name));
            names.push(name);
        }
        if !out.textures.iter().any(|known| known.name == texture.name) {
            out.textures.push(texture);
        }
    }
    names
}

/// The box that holds all the vertices of a model, `None` for a model with none.
fn model_bounds(pack: &Pack, model: u32) -> Option<Aabb> {
    let mut low = [f32::INFINITY; 3];
    let mut high = [f32::NEG_INFINITY; 3];
    for position in pack.object_groups.iter().filter(|g| g.model == model).flat_map(|g| &g.positions) {
        for axis in 0..3 {
            low[axis] = low[axis].min(position[axis]);
            high[axis] = high[axis].max(position[axis]);
        }
    }
    (low[0] <= high[0]).then_some((low, high))
}

/// The twelve triangles of the box `low`-`high` of a model as placed by `instance`, in physics space.
fn box_triangles(low: [f32; 3], high: [f32; 3], instance: &Instance) -> Vec<SoupTriangle> {
    let corner = |k: usize| {
        let local = [
            if k & 1 == 0 { low[0] } else { high[0] },
            if k & 2 == 0 { low[1] } else { high[1] },
            if k & 4 == 0 { low[2] } else { high[2] },
        ];
        let at = |axis: usize| {
            instance.pos[axis]
                + local[0] * instance.rows[0][axis]
                + local[1] * instance.rows[1][axis]
                + local[2] * instance.rows[2][axis]
        };
        to_physics([at(0), at(1), at(2)])
    };
    let surface = surface_hash(WALL_SURFACE);
    const FACES: [[usize; 4]; 6] = [[0, 1, 3, 2], [4, 5, 7, 6], [0, 1, 5, 4], [2, 3, 7, 6], [0, 2, 6, 4], [1, 3, 7, 5]];
    FACES
        .iter()
        .flat_map(|f| [[f[0], f[1], f[2]], [f[0], f[2], f[3]]])
        .map(|t| SoupTriangle { points: t.map(corner), surface })
        .collect()
}

/// How many placements each model has, by model name: a summary for the log.
pub fn counts(pack: &Pack) -> BTreeMap<String, usize> {
    let mut out = BTreeMap::new();
    for instance in &pack.instances {
        *out.entry(pack.models[instance.model as usize].name.clone()).or_default() += 1;
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pack::{Group, Lap, Material, ModelInfo};

    fn pack() -> Pack {
        let material = Material { name: "m".into(), texture: None, color: [9, 9, 9], surface: "concrete".into() };
        // One model: a 2 by 1 by 1 box of triangles (just its corners matter for the bounds), solid.
        let wall = Group {
            material: 0,
            collide: true,
            model: 0,
            positions: vec![[0.0, 0.0, 0.0], [2.0, 0.0, 0.0], [2.0, 1.0, 1.0]],
            uvs: vec![[0.0, 0.0]; 3],
            normals: vec![[0, 0, 127]; 3],
            indices: vec![0, 1, 2],
        };
        let rows = [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]];
        Pack {
            track: "T".into(),
            materials: vec![material],
            groups: Vec::new(),
            lap: Lap::default(),
            sea: None,
            models: vec![ModelInfo { name: "wall".into(), solid: true }],
            object_groups: vec![wall],
            instances: vec![
                Instance { model: 0, pos: [10.0, 0.0, 5.0], rows },
                Instance { model: 0, pos: [500.0, 0.0, 5.0], rows },
                Instance { model: 0, pos: [0.0, 0.0, 0.0], rows: [[9.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]] },
            ],
        }
    }

    fn pictures() -> Pictures {
        Pictures::load(std::path::Path::new("."), &[], 64).0
    }

    #[test]
    fn a_placement_goes_to_the_nearest_cell() {
        let cell = |x: f32| Cell {
            key: (0, 0),
            parts: vec![crate::cells::Part {
                material: 0,
                positions: vec![[x - 1.0, 0.0, 0.0], [x + 1.0, 0.0, 0.0], [x, 1.0, 0.0]],
                uvs: vec![[0.0, 0.0]; 3],
                normals: vec![[0, 0, 127]; 3],
                indices: vec![0, 1, 2],
            }],
            ground: Vec::new(),
        };
        let assigned = assign(&pack(), &[cell(0.0), cell(500.0)]);
        assert_eq!(assigned, vec![vec![0, 2], vec![1]]);
    }

    #[test]
    fn a_cell_gets_the_model_once_each_placement_and_a_box_of_walls() {
        let objects = cell_objects(&pack(), &pictures(), &[0, 1], "G000");
        assert_eq!(objects.solids.len(), 1, "the model is made once");
        assert_eq!(objects.places.len(), 2);
        assert_eq!(objects.places[0].solid, "G000_M0_0");
        assert_eq!(objects.walls.len(), 24, "two boxes of twelve triangles");
        assert_eq!(objects.textures.len(), 1);
        // The first box is 2 m long in x: render x 10..12 is physics z 10..12.
        let z: Vec<f32> = objects.walls[..12].iter().flat_map(|t| t.points.map(|p| p[2])).collect();
        assert!(z.iter().cloned().fold(f32::INFINITY, f32::min) >= 10.0 - 1e-4);
        assert!(z.iter().cloned().fold(f32::NEG_INFINITY, f32::max) <= 12.0 + 1e-4);
    }

    #[test]
    fn a_scale_beyond_the_format_is_left_out_and_counted() {
        let objects = cell_objects(&pack(), &pictures(), &[2], "G000");
        assert_eq!(objects.skipped, 1);
        assert!(objects.places.is_empty());
        assert_eq!(counts(&pack()).get("wall"), Some(&3));
    }
}

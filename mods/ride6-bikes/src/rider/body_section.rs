//! The cross-section of a bike's body, from its exported chassis mesh, written to `contacts.json` as `body_section` so the game can keep
//! the rider's knees, elbows and chest out of the tank, the fairing and the exhaust when he leans
//! (`docs/specs/ride6-rider-clearance.md`).
//!
//! The side view around the rider is cut into cells of [`STEP_M`]; the value of a cell is the largest `|y|` (the half width of the bike
//! there) of the vertices of the body-role parts that fall in it, in whole millimetres, 0 where fewer than [`MIN_VERTICES`] fall. Only
//! what is fixed to the chassis counts: the parts that are not skinned, and the vertices skinned mostly to the chassis bone, not the fork,
//! the handlebar, the levers and the wheels, which turn or move (the hands hold the grips on purpose). All numbers are choices of
//! the measurement, not data of the game.

use std::path::Path;

use anyhow::Result;
use glam::Vec3;
use serde_json::{Value, json};

use crate::exports::Exports;
use crate::partfile::{Kind, PartTable, Role};

/// The cell of the grid, metres: a rider's knee is 15 cm across, a tank's flank changes over several.
pub const STEP_M: f32 = 0.03;
/// The grid's extent around the rider's root (car frame), metres: behind it, ahead of it (the handlebar), below it (the pegs) and
/// above it (the chest and the screen).
pub const BEHIND_M: f32 = 0.55;
pub const AHEAD_M: f32 = 0.65;
pub const BELOW_M: f32 = 0.70;
pub const ABOVE_M: f32 = 0.35;
/// The fewest vertices a cell must have to count: a lone vertex is a bolt or an antenna, not a wall.
pub const MIN_VERTICES: u16 = 2;
/// The bones whose vertices are fixed to the chassis.
const FIXED_BONES: [&str; 2] = ["BONE_CHASSIS", "VEHICLE_BASE"];

/// The grid being filled.
pub struct Grid {
    x0: f32,
    z0: f32,
    nx: usize,
    nz: usize,
    widest: Vec<f32>,
    count: Vec<u16>,
}

impl Grid {
    /// A grid around the rider's root `root` (car frame).
    pub fn around(root: Vec3) -> Self {
        let (x0, z0) = (root.x - BEHIND_M, root.z - BELOW_M);
        let nx = ((BEHIND_M + AHEAD_M) / STEP_M).ceil() as usize;
        let nz = ((BELOW_M + ABOVE_M) / STEP_M).ceil() as usize;
        Self { x0, z0, nx, nz, widest: vec![0.0; nx * nz], count: vec![0; nx * nz] }
    }

    /// Takes in a vertex of the body at `p` (car frame).
    pub fn add(&mut self, p: Vec3) {
        let (i, j) = (((p.x - self.x0) / STEP_M).floor(), ((p.z - self.z0) / STEP_M).floor());
        if !(i >= 0.0 && j >= 0.0 && (i as usize) < self.nx && (j as usize) < self.nz) {
            return;
        }
        let cell = i as usize * self.nz + j as usize;
        self.widest[cell] = self.widest[cell].max(p.y.abs());
        self.count[cell] = self.count[cell].saturating_add(1);
    }

    /// The `body_section` object of `contacts.json`.
    pub fn to_json(&self) -> Value {
        let mm: Vec<u16> = self
            .widest
            .iter()
            .zip(&self.count)
            .map(|(w, n)| if *n >= MIN_VERTICES { (w * 1000.0).round().min(f32::from(u16::MAX)) as u16 } else { 0 })
            .collect();
        json!({
            "x0": round(self.x0), "z0": round(self.z0), "step": STEP_M,
            "nx": self.nx, "nz": self.nz, "half_width_mm": mm,
        })
    }

    /// Whether any cell has a body in it.
    pub fn is_empty(&self) -> bool {
        self.count.iter().all(|n| *n < MIN_VERTICES)
    }
}

/// Millimetres, so the file stays short.
fn round(metres: f32) -> f32 {
    (metres * 1000.0).round() / 1000.0
}

/// Adds `body_section` to `contacts`, measured on the body parts of the bike in `bike_dir`, around the rider's root. A folder without
/// meshes, or a bike whose mesh has nothing near the rider, leaves the file as it is.
pub fn add(contacts: &mut Value, bike_dir: &Path) -> Result<()> {
    let root = contacts
        .get("rider_root_in_bike")
        .and_then(|s| s.get("translation"))
        .and_then(Value::as_array)
        .and_then(|t| t.iter().map(|v| v.as_f64().map(|f| f as f32)).collect::<Option<Vec<f32>>>())
        .filter(|t| t.len() == 3);
    let Some(root) = root else { return Ok(()) };
    if !bike_dir.join("glb").is_dir() {
        return Ok(());
    }
    let table = PartTable::read(bike_dir)?;
    let exports = Exports::scan(&bike_dir.join("glb"))?;
    let mut grid = Grid::around(Vec3::new(root[0], root[1], root[2]));
    for part in table.parts.iter().filter(|p| p.kind == Kind::Body && p.role == Role::Body) {
        let glb = exports.load(&part.export, 0)?;
        for primitive in &glb.primitives {
            for (i, position) in primitive.positions.iter().enumerate() {
                if primitive.is_skinned() {
                    let weights = primitive.weights[i];
                    let strongest = (0..4).fold(0, |best, k| if weights[k] > weights[best] { k } else { best });
                    let joint = usize::from(primitive.joints[i][strongest]);
                    let name = glb.joints.get(joint).and_then(|n| glb.nodes.get(*n)).map(|node| node.name.as_str());
                    if !name.is_some_and(|name| FIXED_BONES.contains(&name)) {
                        continue;
                    }
                }
                grid.add(super::seat_surface::car_point(&part.anchor, *position));
            }
        }
    }
    if grid.is_empty() {
        println!("{}: no body near the rider for a section", bike_dir.display());
        return Ok(());
    }
    contacts["body_section"] = grid.to_json();
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_cell_holds_the_widest_vertex_in_it_and_ignores_a_lone_one() {
        let root = Vec3::new(0.0, 0.0, 1.0);
        let mut grid = Grid::around(root);
        // A tank flank: vertices 0.13 and 0.12 m from the centre line (both sides) in one cell, a lone bolt 0.4 m out in another.
        for y in [0.13, -0.12, 0.10] {
            grid.add(Vec3::new(0.2, y, 0.9));
        }
        grid.add(Vec3::new(-0.3, 0.4, 0.9));
        let section = grid.to_json();
        let cells: Vec<u64> = section["half_width_mm"].as_array().unwrap().iter().filter_map(Value::as_u64).collect();
        assert_eq!(cells.iter().copied().max(), Some(130), "the widest of the three");
        assert_eq!(cells.iter().filter(|c| **c > 0).count(), 1, "the bolt is not a wall");
        assert_eq!(cells.len(), section["nx"].as_u64().unwrap() as usize * section["nz"].as_u64().unwrap() as usize);
        assert!(!grid.is_empty());
    }

    #[test]
    fn the_cell_is_found_from_the_corner_and_a_point_off_the_grid_is_left_out() {
        let root = Vec3::new(0.0, 0.0, 1.0);
        let mut grid = Grid::around(root);
        for _ in 0..2 {
            grid.add(Vec3::new(root.x - BEHIND_M + 0.01, 0.2, root.z - BELOW_M + 0.01));
            grid.add(Vec3::new(root.x + AHEAD_M + 1.0, 0.2, root.z));
            grid.add(Vec3::new(root.x, 0.2, root.z + ABOVE_M + 1.0));
        }
        let section = grid.to_json();
        let cells: Vec<u64> = section["half_width_mm"].as_array().unwrap().iter().filter_map(Value::as_u64).collect();
        assert_eq!(cells[0], 200, "the corner cell");
        assert_eq!(cells.iter().filter(|c| **c > 0).count(), 1);
        assert!((section["x0"].as_f64().unwrap() - f64::from(root.x - BEHIND_M)).abs() < 1e-3);
        assert!((section["z0"].as_f64().unwrap() - f64::from(root.z - BELOW_M)).abs() < 1e-3);
    }

    #[test]
    fn an_empty_grid_is_empty() {
        assert!(Grid::around(Vec3::ZERO).is_empty());
    }
}

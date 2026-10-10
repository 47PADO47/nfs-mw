//! Reverse-Z precision: pairs of quads from 2 m to 9 km away that cross each other or sit 0.1 % apart.
//!
//! Every pair is the same shape scaled about the camera, so they all project to the same size on screen.
//! Along the top, a green quad is tilted through a red one: the picture must show the crossing line. Along
//! the bottom a green quad sits 0.1 % behind a red one but is drawn after it: the overlap must stay red.

use blackbox_gfx::{BlendMode, Instance, Shading};
use glam::{Mat4, Vec3};

use super::{Cx, Parts};
use crate::camera::Camera;
use crate::mesh::{MeshBuilder, prelit};

const CLEAR: [f32; 3] = [0.04, 0.04, 0.06];
/// Distances (the depth of the red quad's plane) of the four columns.
pub const DISTANCES: [f32; 4] = [2.0, 80.0, 2500.0, 9000.0];
/// The height (in the unit space) of the row where a quad is tilted through another.
pub const ROW_CROSSING: f32 = 0.25;
/// The height of the row where a quad sits 0.1 % behind another.
pub const ROW_BEHIND: f32 = -0.25;
const RED: [f32; 3] = [0.9, 0.1, 0.1];
const GREEN: [f32; 3] = [0.1, 0.8, 0.2];

/// The x of the centre of column `k`, in the unit space where the quads sit at distance 1.
pub fn column_x(k: usize) -> f32 {
    -0.75 + 0.5 * k as f32
}

/// A quad with corners given at distance 1, scaled about the camera at the origin.
fn scaled_quad(mesh: &mut MeshBuilder, corners: [Vec3; 4], scale: f32, rgb: [f32; 3]) {
    mesh.flat_quad(corners.map(|c| c * scale), prelit(rgb, 1.0));
}

pub(super) fn build(cx: &mut Cx) -> Parts {
    let mut mesh = MeshBuilder::new();
    mesh.state(None, BlendMode::Opaque, Shading::Prelit);
    for (k, &distance) in DISTANCES.iter().enumerate() {
        let x = column_x(k);
        // Row A: the green quad runs from 0.9 (in front) to 1.1 (behind) the red plane.
        let z = ROW_CROSSING;
        let red = [
            Vec3::new(x - 0.12, 1.0, z - 0.15),
            Vec3::new(x + 0.12, 1.0, z - 0.15),
            Vec3::new(x + 0.12, 1.0, z + 0.15),
            Vec3::new(x - 0.12, 1.0, z + 0.15),
        ];
        let tilted = [
            Vec3::new(x - 0.06, 0.9, z - 0.15),
            Vec3::new(x + 0.18, 1.1, z - 0.15),
            Vec3::new(x + 0.18, 1.1, z + 0.15),
            Vec3::new(x - 0.06, 0.9, z + 0.15),
        ];
        scaled_quad(&mut mesh, red, distance, RED);
        scaled_quad(&mut mesh, tilted, distance, GREEN);
        // Row B: green 0.1 % behind red, drawn second.
        let z = ROW_BEHIND;
        let red = [
            Vec3::new(x - 0.12, 1.0, z - 0.15),
            Vec3::new(x + 0.12, 1.0, z - 0.15),
            Vec3::new(x + 0.12, 1.0, z + 0.15),
            Vec3::new(x - 0.12, 1.0, z + 0.15),
        ];
        let behind = [
            Vec3::new(x - 0.02, 1.001, z - 0.15),
            Vec3::new(x + 0.2, 1.001, z - 0.15),
            Vec3::new(x + 0.2, 1.001, z + 0.15),
            Vec3::new(x - 0.02, 1.001, z + 0.15),
        ];
        scaled_quad(&mut mesh, red, distance, RED);
        scaled_quad(&mut mesh, behind, distance, GREEN);
    }
    let mesh = cx.mesh("depth probe", &mesh.finish());
    let camera = Camera::new(Vec3::ZERO, Vec3::Y);
    Parts::world(camera.frame(cx.aspect, CLEAR, None), vec![Instance::new(mesh, Mat4::IDENTITY)])
}

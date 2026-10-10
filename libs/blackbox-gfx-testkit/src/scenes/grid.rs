//! An opaque ground grid: BC1 and RGBA textures, vertex colours, a fog ramp, and lit boxes.

use blackbox_gfx::{BlendMode, Instance, Shading};
use glam::{Mat4, Vec3};

use super::{Cx, Parts};
use crate::camera::Camera;
use crate::mesh::{MeshBuilder, prelit};
use crate::texture::{Image, Storage};

const CLEAR: [f32; 3] = [0.55, 0.65, 0.80];
const COLUMNS: i32 = 12;
const ROWS: i32 = 28;
const CELL: f32 = 2.0;

/// A smoothly varying pre-lit colour, so interpolation across each cell shows.
fn tint(x: f32, y: f32) -> [u8; 4] {
    prelit([0.75 + 0.25 * (x * 0.7).sin(), 0.75 + 0.25 * (y * 0.3 + x * 0.2).cos(), 0.8 + 0.2 * (y * 0.5).sin()], 1.0)
}

/// The cells of the chessboard whose parity is `parity`, as one mesh with one texture.
fn ground(parity: i32, texture: blackbox_gfx::TextureHandle) -> MeshBuilder {
    let mut mesh = MeshBuilder::new();
    mesh.state(Some(texture), BlendMode::Opaque, Shading::Prelit);
    for i in 0..COLUMNS {
        for j in 0..ROWS {
            if (i + j) % 2 != parity {
                continue;
            }
            let (x0, y0) = ((i - COLUMNS / 2) as f32 * CELL, j as f32 * CELL);
            let (x1, y1) = (x0 + CELL, y0 + CELL);
            let corners =
                [Vec3::new(x0, y0, 0.0), Vec3::new(x1, y0, 0.0), Vec3::new(x1, y1, 0.0), Vec3::new(x0, y1, 0.0)];
            mesh.quad(corners, [tint(x0, y0), tint(x1, y0), tint(x1, y1), tint(x0, y1)], [1.0, 1.0]);
        }
    }
    mesh
}

pub(super) fn build(cx: &mut Cx) -> Parts {
    let bc1 = cx.texture("grid bc1", &Image::checker(64, 8, [230, 150, 60], [120, 60, 30]), Storage::Bc1);
    let rgba = cx.texture("grid rgba", &Image::ramp(64, [40, 170, 170], [240, 240, 230]), Storage::Rgba8);
    let even = cx.mesh("grid even", &ground(0, bc1).finish());
    let odd = cx.mesh("grid odd", &ground(1, rgba).finish());

    let mut boxes = MeshBuilder::new();
    boxes.state(Some(bc1), BlendMode::Opaque, Shading::Lit);
    boxes.cuboid(Vec3::new(-1.0, -1.0, 0.0), Vec3::new(1.0, 1.0, 2.0), prelit([1.0, 1.0, 1.0], 1.0));
    let boxes = cx.mesh("grid boxes", &boxes.finish());

    let place = |x: f32, y: f32, yaw: f32, scale: f32| {
        Mat4::from_translation(Vec3::new(x, y, 0.0)) * Mat4::from_rotation_z(yaw) * Mat4::from_scale(Vec3::splat(scale))
    };
    let instances = vec![
        Instance::new(even, Mat4::IDENTITY),
        Instance::new(odd, Mat4::IDENTITY),
        Instance::new(boxes, place(-4.0, 9.0, 0.4, 1.0)),
        Instance::new(boxes, place(3.5, 15.0, -0.7, 1.4)),
        Instance::new(boxes, place(-1.0, 24.0, 1.2, 1.8)),
    ];

    let camera = Camera::new(Vec3::new(0.0, -6.0, 4.5), Vec3::new(0.0, 14.0, 0.0));
    Parts::world(camera.frame(cx.aspect, CLEAR, Some((8.0, 45.0))), instances)
}

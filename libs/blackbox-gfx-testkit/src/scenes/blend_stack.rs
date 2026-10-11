//! Alpha-blended and additive quads in a fixed submission order over an opaque background.

use blackbox_gfx::{BlendMode, Instance, Shading};
use glam::{Mat4, Vec3};

use super::{Cx, Parts};
use crate::camera::Camera;
use crate::mesh::{MeshBuilder, prelit};
use crate::texture::{Image, Storage};

const CLEAR: [f32; 3] = [0.05, 0.06, 0.10];

/// A square card `size` wide, its centre at (`x`, `y`, `z`).
fn square(mesh: &mut MeshBuilder, x: f32, y: f32, z: f32, size: f32, color: [u8; 4]) {
    mesh.card(Vec3::new(x, y, z - size * 0.5), size, size, 0.0, color);
}

pub(super) fn build(cx: &mut Cx) -> Parts {
    let checker = cx.texture("blend background", &Image::checker(64, 8, [200, 200, 210], [90, 90, 110]), Storage::Bc1);
    let glow = cx.texture("blend glow", &Image::glow(64), Storage::Rgba8);

    let mut wall = MeshBuilder::new();
    wall.state(Some(checker), BlendMode::Opaque, Shading::Prelit);
    let corners =
        [Vec3::new(-8.0, 10.0, 0.0), Vec3::new(8.0, 10.0, 0.0), Vec3::new(8.0, 10.0, 9.0), Vec3::new(-8.0, 10.0, 9.0)];
    wall.quad(corners, [prelit([0.9; 3], 1.0); 4], [8.0, 4.0]);
    let wall = cx.mesh("blend wall", &wall.finish());

    // Each blend mesh keeps its own quads in order, and the instances keep the meshes in order.
    let tints = [[1.0, 0.2, 0.2], [0.2, 1.0, 0.3], [0.3, 0.4, 1.0], [1.0, 0.9, 0.2]];
    let mut first = MeshBuilder::new();
    first.state(None, BlendMode::AlphaBlend, Shading::Prelit);
    let mut second = MeshBuilder::new();
    second.state(Some(glow), BlendMode::AlphaBlend, Shading::Prelit);
    for (k, tint) in tints.iter().enumerate() {
        let shift = k as f32 * 1.6;
        square(&mut first, -4.5 + shift, 8.0 - k as f32 * 0.2, 5.0 - shift * 0.2, 3.6, prelit(*tint, 0.5));
        square(&mut second, -3.0 + shift, 7.0, 3.5 + shift * 0.15, 3.2, prelit(*tint, 0.85));
    }
    let (first, second) = (cx.mesh("blend first", &first.finish()), cx.mesh("blend second", &second.finish()));

    let mut light = MeshBuilder::new();
    light.state(Some(glow), BlendMode::Additive, Shading::Prelit);
    let beams = [[1.0, 0.5, 0.1], [0.1, 0.8, 1.0], [1.0, 0.2, 0.8]];
    for (k, tint) in beams.iter().enumerate() {
        square(&mut light, -2.0 + k as f32 * 2.4, 6.0, 4.2, 4.0, prelit(*tint, 1.0));
    }
    light.state(None, BlendMode::Additive, Shading::Prelit);
    square(&mut light, 3.5, 6.5, 2.0, 2.4, prelit([0.3, 0.3, 0.6], 0.6));
    // Behind the opaque wall: depth testing must hide it completely.
    square(&mut light, 3.0, 12.0, 6.0, 5.0, prelit([1.0, 1.0, 1.0], 1.0));
    let light = cx.mesh("blend light", &light.finish());

    let instances = [wall, first, second, light].map(|mesh| Instance::new(mesh, Mat4::IDENTITY)).to_vec();
    let camera = Camera::new(Vec3::new(0.0, -6.0, 4.0), Vec3::new(0.0, 10.0, 4.0));
    Parts::world(camera.frame(cx.aspect, CLEAR, None), instances)
}

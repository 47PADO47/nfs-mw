//! The 2D UI layer over a plain backdrop: panels, a translucent overlay, a texture, a clip rectangle and
//! per-vertex colours. The layout is 320 x 180 points whatever the image size.

use blackbox_gfx::{BlendMode, Instance, Shading, UiLayer, UiMesh, UiTextureId, UiTexturePatch, UiVertex};
use glam::{Mat4, Vec3};

use super::{Cx, Parts};
use crate::camera::Camera;
use crate::mesh::{MeshBuilder, prelit};

const LAYOUT_WIDTH: f32 = 320.0;
const CLEAR: [f32; 3] = [0.10, 0.12, 0.16];
const WHITE: u64 = 7001;
const CHECKER: u64 = 7002;

/// A rectangle of two triangles with one colour (premultiplied RGBA) and UVs 0..1.
fn rect(texture: UiTextureId, [x0, y0, x1, y1]: [f32; 4], color: [u8; 4], clip: [f32; 4]) -> UiMesh {
    let vertex = |x: f32, y: f32, u: f32, v: f32| UiVertex { position: [x, y], uv: [u, v], color_rgba: color };
    UiMesh {
        vertices: vec![
            vertex(x0, y0, 0.0, 0.0),
            vertex(x1, y0, 1.0, 0.0),
            vertex(x1, y1, 1.0, 1.0),
            vertex(x0, y1, 0.0, 1.0),
        ],
        indices: vec![0, 1, 2, 0, 2, 3],
        texture,
        clip,
    }
}

fn checker_pixels() -> Vec<u8> {
    (0..16 * 16)
        .flat_map(|i| {
            let (x, y) = (i % 16, i / 16);
            match (x / 4 + y / 4) % 2 == 0 {
                true => [240, 240, 240, 255],
                false => [40, 40, 60, 255],
            }
        })
        .collect()
}

pub(super) fn build(cx: &mut Cx) -> Parts {
    // A backdrop for the UI to sit on.
    let mut backdrop = MeshBuilder::new();
    backdrop.state(None, BlendMode::Opaque, Shading::Prelit);
    let (low, high) = (prelit([0.15, 0.20, 0.30], 1.0), prelit([0.55, 0.65, 0.75], 1.0));
    let wall = [
        Vec3::new(-10.0, 10.0, -6.0),
        Vec3::new(10.0, 10.0, -6.0),
        Vec3::new(10.0, 10.0, 6.0),
        Vec3::new(-10.0, 10.0, 6.0),
    ];
    backdrop.quad(wall, [low, low, high, high], [1.0, 1.0]);
    let backdrop = cx.mesh("ui backdrop", &backdrop.finish());

    let white = cx.ui_texture(WHITE, [1, 1], &[255; 4]);
    let checker = cx.ui_texture(CHECKER, [16, 16], &checker_pixels());
    // A partial update: a red 4x4 block in the second checker cell.
    let red: Vec<u8> = (0..16).flat_map(|_| [220, 30, 30, 255]).collect();
    cx.backend.update_ui_texture(&UiTexturePatch { id: checker, offset: Some([4, 4]), size: [4, 4], rgba: &red });

    let everything = [0.0, 0.0, LAYOUT_WIDTH, 180.0];
    let mut meshes = vec![
        rect(white, [10.0, 10.0, 150.0, 70.0], [30, 60, 160, 255], everything),
        rect(white, [80.0, 40.0, 200.0, 110.0], [115, 26, 13, 128], everything),
        rect(checker, [210.0, 10.0, 300.0, 60.0], [255; 4], everything),
        // Reaches beyond its clip rectangle on every side: only the middle shows.
        rect(white, [100.0, 100.0, 400.0, 190.0], [30, 160, 70, 255], [150.0, 110.0, 250.0, 160.0]),
    ];
    let corner = |x: f32, y: f32, color: [u8; 4]| UiVertex { position: [x, y], uv: [0.5, 0.5], color_rgba: color };
    meshes.push(UiMesh {
        vertices: vec![
            corner(20.0, 170.0, [255, 0, 0, 255]),
            corner(100.0, 170.0, [0, 255, 0, 255]),
            corner(60.0, 110.0, [0, 0, 255, 255]),
        ],
        indices: vec![0, 1, 2],
        texture: white,
        clip: everything,
    });

    let ui = UiLayer { pixels_per_point: cx.size[0] as f32 / LAYOUT_WIDTH, meshes };
    let camera = Camera::new(Vec3::new(0.0, 0.0, 0.0), Vec3::new(0.0, 10.0, 0.0));
    let mut parts = Parts::world(camera.frame(cx.aspect, CLEAR, None), vec![Instance::new(backdrop, Mat4::IDENTITY)]);
    parts.ui = ui;
    parts
}

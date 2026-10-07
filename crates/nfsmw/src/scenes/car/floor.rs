//! A plain floor under the car, so the wheels have something to stand on.

use blackbox_render::{BlendMode, DrawRange, MeshDesc, MeshHandle, Renderer, Shading, Vertex};

/// Half the floor's side, metres.
const HALF_SIZE: f32 = 6.0;

pub fn upload(renderer: &mut Renderer) -> MeshHandle {
    let corner = |x: f32, y: f32| Vertex {
        position: [x * HALF_SIZE, y * HALF_SIZE, 0.0],
        normal: [0.0, 0.0, 1.0],
        color_bgra: [0x58, 0x55, 0x52, 0xFF],
        uv: [0.0, 0.0],
    };
    let vertices = [corner(-1.0, -1.0), corner(1.0, -1.0), corner(1.0, 1.0), corner(-1.0, 1.0)];
    let indices = [0, 1, 2, 0, 2, 3];
    renderer.create_mesh(&MeshDesc {
        label: "floor",
        vertices: &vertices,
        indices: &indices,
        draws: vec![DrawRange {
            first_index: 0,
            index_count: 6,
            base_vertex: 0,
            texture: None,
            blend: BlendMode::Opaque,
            shading: Shading::Lit,
        }],
    })
}

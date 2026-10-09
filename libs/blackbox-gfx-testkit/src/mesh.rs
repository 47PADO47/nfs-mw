//! Procedural geometry: quads, boxes and spheres collected into a mesh with draw ranges.

use blackbox_gfx::{BlendMode, DrawRange, MeshDesc, MeshHandle, RenderBackend, Shading, TextureHandle, Vertex};
use glam::Vec3;

/// An opaque white-ish colour that a pre-lit draw leaves unchanged (the shader doubles vertex colours).
pub const NEUTRAL: [u8; 4] = [128, 128, 128, 255];

/// A pre-lit vertex colour that shows `rgb` (each 0..=1) after the shader's doubling, with `alpha` 0..=1.
pub fn prelit(rgb: [f32; 3], alpha: f32) -> [u8; 4] {
    let byte = |v: f32| (v * 127.5).round().clamp(0.0, 255.0) as u8;
    [byte(rgb[0]), byte(rgb[1]), byte(rgb[2]), (alpha * 255.0).round().clamp(0.0, 255.0) as u8]
}

/// The state of a run of triangles drawn together.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct DrawState {
    texture: Option<TextureHandle>,
    blend: BlendMode,
    shading: Shading,
}

/// Collects vertices and indices; each change of texture, blend mode or shading starts a new draw range.
#[derive(Debug, Default)]
pub struct MeshBuilder {
    vertices: Vec<Vertex>,
    indices: Vec<u16>,
    draws: Vec<DrawRange>,
    state: Option<DrawState>,
    range_start: u32,
}

impl MeshBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    /// Triangles added from now on use this state.
    pub fn state(&mut self, texture: Option<TextureHandle>, blend: BlendMode, shading: Shading) -> &mut Self {
        let next = DrawState { texture, blend, shading };
        if self.state != Some(next) {
            self.close_range();
            self.state = Some(next);
        }
        self
    }

    fn close_range(&mut self) {
        let end = self.indices.len() as u32;
        if let (Some(s), true) = (self.state, end > self.range_start) {
            self.draws.push(DrawRange {
                first_index: self.range_start,
                index_count: end - self.range_start,
                base_vertex: 0,
                texture: s.texture,
                blend: s.blend,
                shading: s.shading,
            });
        }
        self.range_start = end;
    }

    /// Add a vertex; `rgba` is the colour as stored (see [`prelit`]).
    fn vertex(&mut self, position: Vec3, normal: Vec3, rgba: [u8; 4], uv: [f32; 2]) -> u16 {
        let index = u16::try_from(self.vertices.len()).expect("a testkit mesh holds at most 65536 vertices");
        let color_bgra = [rgba[2], rgba[1], rgba[0], rgba[3]];
        self.vertices.push(Vertex { position: position.to_array(), normal: normal.to_array(), color_bgra, uv });
        index
    }

    /// A quad from four corners in counter-clockwise order with one colour per corner. UVs run
    /// (0,0), (1,0), (1,1), (0,1) scaled by `uv_scale`.
    pub fn quad(&mut self, corners: [Vec3; 4], colors: [[u8; 4]; 4], uv_scale: [f32; 2]) -> &mut Self {
        let normal = (corners[1] - corners[0]).cross(corners[3] - corners[0]).normalize_or_zero();
        let uvs = [[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]];
        let ids: [u16; 4] = std::array::from_fn(|i| {
            self.vertex(corners[i], normal, colors[i], [uvs[i][0] * uv_scale[0], uvs[i][1] * uv_scale[1]])
        });
        self.indices.extend_from_slice(&[ids[0], ids[1], ids[2], ids[0], ids[2], ids[3]]);
        self
    }

    /// A quad of one colour with UVs 0..1.
    pub fn flat_quad(&mut self, corners: [Vec3; 4], color: [u8; 4]) -> &mut Self {
        self.quad(corners, [color; 4], [1.0, 1.0])
    }

    /// An upright quad facing -y (towards a camera looking along +y): `centre` is the middle of its
    /// bottom edge, `width` along x and `height` along z, rotated about z by `yaw` radians.
    pub fn card(&mut self, centre: Vec3, width: f32, height: f32, yaw: f32, color: [u8; 4]) -> &mut Self {
        let (s, c) = yaw.sin_cos();
        let along = Vec3::new(c, s, 0.0) * (width * 0.5);
        let up = Vec3::new(0.0, 0.0, height);
        self.flat_quad([centre - along, centre + along, centre + along + up, centre - along + up], color)
    }

    /// An axis-aligned box with every face one colour; faces have outward normals.
    pub fn cuboid(&mut self, min: Vec3, max: Vec3, color: [u8; 4]) -> &mut Self {
        let p = |x: bool, y: bool, z: bool| {
            Vec3::new(if x { max.x } else { min.x }, if y { max.y } else { min.y }, if z { max.z } else { min.z })
        };
        let faces = [
            [p(false, false, false), p(true, false, false), p(true, false, true), p(false, false, true)],
            [p(true, true, false), p(false, true, false), p(false, true, true), p(true, true, true)],
            [p(true, false, false), p(true, true, false), p(true, true, true), p(true, false, true)],
            [p(false, true, false), p(false, false, false), p(false, false, true), p(false, true, true)],
            [p(false, false, true), p(true, false, true), p(true, true, true), p(false, true, true)],
            [p(false, true, false), p(true, true, false), p(true, false, false), p(false, false, false)],
        ];
        for face in faces {
            self.flat_quad(face, color);
        }
        self
    }

    /// A UV sphere with outward normals. `color_at` gives the vertex colour from the unit direction.
    pub fn sphere(
        &mut self,
        centre: Vec3,
        radius: f32,
        rings: u16,
        segments: u16,
        color_at: impl Fn(Vec3) -> [u8; 4],
    ) -> &mut Self {
        let base = self.vertices.len() as u16;
        for ring in 0..=rings {
            let polar = std::f32::consts::PI * f32::from(ring) / f32::from(rings);
            for seg in 0..=segments {
                let azimuth = std::f32::consts::TAU * f32::from(seg) / f32::from(segments);
                let dir = Vec3::new(polar.sin() * azimuth.cos(), polar.sin() * azimuth.sin(), polar.cos());
                let uv = [f32::from(seg) / f32::from(segments), f32::from(ring) / f32::from(rings)];
                self.vertex(centre + dir * radius, dir, color_at(dir), uv);
            }
        }
        let stride = segments + 1;
        for ring in 0..rings {
            for seg in 0..segments {
                let a = base + ring * stride + seg;
                let b = a + stride;
                self.indices.extend_from_slice(&[a, b, a + 1, a + 1, b, b + 1]);
            }
        }
        self
    }

    /// The finished vertices, indices and draw ranges.
    pub fn finish(mut self) -> BuiltMesh {
        self.close_range();
        BuiltMesh { vertices: self.vertices, indices: self.indices, draws: self.draws }
    }
}

/// Geometry ready to upload.
#[derive(Debug, Clone)]
pub struct BuiltMesh {
    pub vertices: Vec<Vertex>,
    pub indices: Vec<u16>,
    pub draws: Vec<DrawRange>,
}

impl BuiltMesh {
    pub fn upload(&self, backend: &mut dyn RenderBackend, label: &str) -> MeshHandle {
        backend.create_mesh(&MeshDesc {
            label,
            vertices: &self.vertices,
            indices: &self.indices,
            draws: self.draws.clone(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lit(b: &mut MeshBuilder, blend: BlendMode) {
        b.state(None, blend, Shading::Lit);
    }

    #[test]
    fn a_change_of_state_starts_a_new_draw_range() {
        let mut b = MeshBuilder::new();
        lit(&mut b, BlendMode::Opaque);
        b.card(Vec3::ZERO, 1.0, 1.0, 0.0, NEUTRAL).card(Vec3::X, 1.0, 1.0, 0.0, NEUTRAL);
        lit(&mut b, BlendMode::Additive);
        b.card(Vec3::Y, 1.0, 1.0, 0.0, NEUTRAL);
        let mesh = b.finish();
        assert_eq!(mesh.draws.len(), 2);
        assert_eq!((mesh.draws[0].first_index, mesh.draws[0].index_count), (0, 12));
        assert_eq!((mesh.draws[1].first_index, mesh.draws[1].index_count), (12, 6));
        assert_eq!(mesh.draws[1].blend, BlendMode::Additive);
    }

    #[test]
    fn setting_the_same_state_again_keeps_one_range() {
        let mut b = MeshBuilder::new();
        lit(&mut b, BlendMode::Opaque);
        b.card(Vec3::ZERO, 1.0, 1.0, 0.0, NEUTRAL);
        lit(&mut b, BlendMode::Opaque);
        b.card(Vec3::X, 1.0, 1.0, 0.0, NEUTRAL);
        assert_eq!(b.finish().draws.len(), 1);
    }

    #[test]
    fn a_card_faces_the_camera_that_looks_along_plus_y() {
        let mut b = MeshBuilder::new();
        lit(&mut b, BlendMode::Opaque);
        b.card(Vec3::ZERO, 2.0, 1.0, 0.0, NEUTRAL);
        let mesh = b.finish();
        assert!(mesh.vertices.iter().all(|v| v.normal == [0.0, -1.0, 0.0]));
    }

    #[test]
    fn a_cuboid_has_outward_normals() {
        let mut b = MeshBuilder::new();
        lit(&mut b, BlendMode::Opaque);
        b.cuboid(Vec3::splat(-1.0), Vec3::splat(1.0), NEUTRAL);
        let mesh = b.finish();
        assert_eq!(mesh.vertices.len(), 24);
        assert!(mesh.vertices.iter().all(|v| Vec3::from(v.position).dot(Vec3::from(v.normal)) > 0.5));
        for tri in mesh.indices.chunks_exact(3) {
            let [a, b, c] = [0, 1, 2].map(|i| Vec3::from(mesh.vertices[tri[i] as usize].position));
            let winding = (b - a).cross(c - a);
            assert!(winding.dot(a) > 0.0, "counter-clockwise seen from outside");
        }
    }

    #[test]
    fn a_sphere_has_unit_normals_and_stays_in_u16_range() {
        let mut b = MeshBuilder::new();
        lit(&mut b, BlendMode::Opaque);
        b.sphere(Vec3::new(1.0, 2.0, 3.0), 2.0, 8, 16, |_| NEUTRAL);
        let mesh = b.finish();
        assert_eq!(mesh.vertices.len(), 9 * 17);
        assert!(mesh.vertices.iter().all(|v| (Vec3::from(v.normal).length() - 1.0).abs() < 1e-5));
        assert!(mesh.indices.iter().all(|&i| usize::from(i) < mesh.vertices.len()));
    }

    #[test]
    fn prelit_colours_are_halved_so_the_shader_doubling_restores_them() {
        assert_eq!(prelit([1.0, 0.5, 0.0], 1.0), [128, 64, 0, 255]);
        assert_eq!(prelit([2.0, -1.0, 0.0], 0.5)[..2], [255, 0]);
    }
}

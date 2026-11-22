//! Immutable per-mesh BLAS and compact CPU hit-shading sidecar.
use blackbox_gfx::{BlendMode, MeshDesc, Shading};
use wgpu::util::DeviceExt;

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub(super) struct Vertex {
    pub position: [f32; 4],
    pub normal: [f32; 4],
    pub colour: [f32; 4],
    pub uv: [f32; 4],
}
pub(super) struct Mesh {
    pub blas: Option<wgpu::Blas>,
    pub center: glam::Vec3,
    pub radius: f32,
    pub vertices: Vec<Vertex>,
    pub triangles: Vec<[u32; 4]>,
    pub textures: Vec<usize>,
    /// Alpha-tested triangles, stored first. The rest form a second, `OPAQUE` BLAS geometry whose
    /// shadow-ray hits commit without shader involvement; see [`Mesh::partition_cutouts`].
    pub cutouts: u32,
}
impl Mesh {
    pub fn new(desc: &MeshDesc<'_>, layers: &[Option<u32>]) -> Option<Self> {
        let vertices: Vec<_> = desc
            .vertices
            .iter()
            .map(|v| Vertex {
                position: [v.position[0], v.position[1], v.position[2], 1.0],
                normal: [v.normal[0], v.normal[1], v.normal[2], 0.0],
                colour: [v.color_bgra[2], v.color_bgra[1], v.color_bgra[0], v.color_bgra[3]]
                    .map(|v| f32::from(v) / 255.0),
                uv: [v.uv[0], v.uv[1], 0.0, 0.0],
            })
            .collect();
        let mut triangles = Vec::new();
        for (draw, layer) in desc.draws.iter().zip(layers) {
            if !matches!(draw.blend, BlendMode::Opaque | BlendMode::AlphaTest) {
                continue;
            }
            if matches!(draw.shading, Shading::Sky | Shading::SkyLayer(_)) {
                continue;
            }
            let Some(layer) = layer else { continue };
            let kind = match draw.shading {
                Shading::World | Shading::WorldDistant | Shading::WorldNormal(_) | Shading::Tree => 1,
                Shading::Lit => 2,
                Shading::WorldEmissive => 3,
                Shading::Glossy(h) if h.raw() < super::bindings::MAX_MATERIALS => h.raw() as u32 + 4,
                Shading::Glossy(_) => 2,
                _ => 0,
            };
            let flags = layer
                | ((kind & 255) << 16)
                | ((kind >> 8) << 26)
                | (u32::from(draw.blend == BlendMode::AlphaTest) << 24)
                | (u32::from(!draw.two_sided) << 25);
            let start = draw.first_index as usize;
            let end = start.checked_add(draw.index_count as usize)?;
            for triangle in desc.indices.get(start..end)?.as_chunks::<3>().0 {
                let mut ids = [0; 3];
                for (id, index) in ids.iter_mut().zip(triangle) {
                    let value = i64::from(*index) + i64::from(draw.base_vertex);
                    if value < 0 || value >= vertices.len() as i64 {
                        return None;
                    }
                    *id = value as u32;
                }
                triangles.push([ids[0], ids[1], ids[2], flags]);
            }
        }
        if triangles.is_empty() {
            return None;
        }
        let textures = triangles
            .iter()
            .map(|t| (t[3] & 65535) as usize)
            .collect::<std::collections::BTreeSet<_>>()
            .into_iter()
            .collect();
        let mut low = glam::Vec3::splat(f32::INFINITY);
        let mut high = glam::Vec3::splat(f32::NEG_INFINITY);
        for v in &vertices {
            let p = glam::Vec3::from_slice(&v.position[..3]);
            low = low.min(p);
            high = high.max(p);
        }
        let center = (low + high) * 0.5;
        let radius = (high - low).length() * 0.5;
        Some(Self { blas: None, center, radius, vertices, triangles, textures, cutouts: 0 })
    }
    /// Moves alpha-tested triangles (flag bit 24) ahead of opaque ones, keeping each group's order.
    /// Runs once after per-draw passes that walk `triangles` in draw order. Opaque-blend triangles
    /// then form their own BLAS geometry flagged `OPAQUE`: the sun-shadow query confirms such hits
    /// unconditionally anyway, and reflection rays force every hit non-opaque. A hit in geometry 1 is
    /// at triangle `cutouts + primitive_index`; the reflection shader reads `cutouts` from the
    /// instance's winding word.
    pub fn partition_cutouts(&mut self) {
        let (mut cutouts, opaque): (Vec<_>, Vec<_>) = self.triangles.iter().partition(|t| t[3] & (1 << 24) != 0);
        self.cutouts = cutouts.len() as u32;
        cutouts.extend(opaque);
        self.triangles = cutouts;
    }
    pub fn build(&mut self, device: &wgpu::Device, queue: &wgpu::Queue) {
        if self.blas.is_some() {
            return;
        }
        let vertices = &self.vertices;
        let indices: Vec<u32> = self.triangles.iter().flat_map(|t| t[..3].iter().copied()).collect();
        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("RT BLAS vertices"),
            contents: bytemuck::cast_slice(vertices),
            usage: wgpu::BufferUsages::BLAS_INPUT,
        });
        let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("RT BLAS indices"),
            contents: bytemuck::cast_slice(&indices),
            usage: wgpu::BufferUsages::BLAS_INPUT,
        });
        let split = self.cutouts as usize * 3;
        let sizes: Vec<_> = [
            (0, split, wgpu::AccelerationStructureGeometryFlags::empty()),
            (split, indices.len(), wgpu::AccelerationStructureGeometryFlags::OPAQUE),
        ]
        .into_iter()
        .filter(|(start, end, _)| end > start)
        .map(|(start, end, flags)| {
            let size = wgpu::BlasTriangleGeometrySizeDescriptor {
                vertex_format: wgpu::VertexFormat::Float32x3,
                vertex_count: vertices.len() as u32,
                index_format: Some(wgpu::IndexFormat::Uint32),
                index_count: Some((end - start) as u32),
                flags,
            };
            (start as u32, size)
        })
        .collect();
        let blas = device.create_blas(
            &wgpu::CreateBlasDescriptor {
                label: Some("RT resident mesh"),
                flags: wgpu::AccelerationStructureFlags::PREFER_FAST_TRACE,
                update_mode: wgpu::AccelerationStructureUpdateMode::Build,
            },
            wgpu::BlasGeometrySizeDescriptors::Triangles {
                descriptors: sizes.iter().map(|(_, size)| size.clone()).collect(),
            },
        );
        let geometries = sizes
            .iter()
            .map(|(first, size)| wgpu::BlasTriangleGeometry {
                size,
                vertex_buffer: &vertex_buffer,
                first_vertex: 0,
                vertex_stride: 64,
                index_buffer: Some(&index_buffer),
                first_index: Some(*first),
                transform_buffer: None,
                transform_buffer_offset: None,
            })
            .collect();
        let entry =
            wgpu::BlasBuildEntry { blas: &blas, geometry: wgpu::BlasGeometries::TriangleGeometries(geometries) };
        let mut encoder = device.create_command_encoder(&Default::default());
        encoder.build_acceleration_structures([&entry], std::iter::empty());
        queue.submit([encoder.finish()]);
        self.blas = Some(blas);
    }
}

//! Mesh upload and release.

use wgpu::util::DeviceExt;

use super::Renderer;
use crate::{DrawRange, MeshDesc, MeshHandle};

pub(super) struct GpuMesh {
    pub vertices: wgpu::Buffer,
    pub secondary_uvs: Option<wgpu::Buffer>,
    pub tangents: Option<wgpu::Buffer>,
    pub indices: wgpu::Buffer,
    pub draws: Vec<DrawRange>,
    pub center: glam::Vec3,
    pub radius: f32,
}

impl Renderer {
    pub fn create_mesh(&mut self, desc: &MeshDesc<'_>) -> MeshHandle {
        let vertices = self.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some(desc.label),
            contents: bytemuck::cast_slice(desc.vertices),
            usage: wgpu::BufferUsages::VERTEX,
        });
        // Only the authored sky and normal-map pipelines consume extra streams.
        // Ordinary world/car meshes retain their original upload footprint.
        let secondary_uvs =
            desc.draws.iter().any(|draw| matches!(draw.shading, crate::Shading::SkyLayer(_))).then(|| {
                let values: Vec<[f32; 2]> = desc
                    .vertices
                    .iter()
                    .enumerate()
                    .map(|(index, vertex)| {
                        desc.secondary_uvs.and_then(|uvs| uvs.get(index)).copied().unwrap_or(vertex.uv)
                    })
                    .collect();
                self.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("secondary UV channel"),
                    contents: bytemuck::cast_slice(&values),
                    usage: wgpu::BufferUsages::VERTEX,
                })
            });
        let tangents =
            desc.draws.iter().any(|draw| matches!(draw.shading, crate::Shading::WorldNormal(_))).then(|| {
                let values: Vec<[f32; 4]> = desc
                    .vertices
                    .iter()
                    .enumerate()
                    .map(|(index, _)| desc.tangents.and_then(|values| values.get(index)).copied().unwrap_or([0.0; 4]))
                    .collect();
                self.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("tangent handedness channel"),
                    contents: bytemuck::cast_slice(&values),
                    usage: wgpu::BufferUsages::VERTEX,
                })
            });
        // Index buffers must be a multiple of 4 bytes.
        let mut indices = desc.indices.to_vec();
        if indices.len() % 2 == 1 {
            indices.push(0);
        }
        let indices = self.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some(desc.label),
            contents: bytemuck::cast_slice(&indices),
            usage: wgpu::BufferUsages::INDEX,
        });
        let mut min = glam::Vec3::splat(f32::INFINITY);
        let mut max = glam::Vec3::splat(f32::NEG_INFINITY);
        for v in desc.vertices {
            let p = glam::Vec3::from_array(v.position);
            min = min.min(p);
            max = max.max(p);
        }
        let center = (min + max) * 0.5;
        let radius = center.distance(max);
        MeshHandle::from_raw(self.meshes.insert(GpuMesh {
            vertices,
            secondary_uvs,
            tangents,
            indices,
            draws: desc.draws.clone(),
            center,
            radius,
        }))
    }

    pub fn destroy_mesh(&mut self, handle: MeshHandle) {
        self.meshes.remove(handle.raw());
    }
}

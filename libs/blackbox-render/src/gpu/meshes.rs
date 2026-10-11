//! Mesh upload and release.

use wgpu::util::DeviceExt;

use super::Renderer;
use crate::{DrawRange, MeshDesc, MeshHandle};

pub(super) struct GpuMesh {
    pub vertices: wgpu::Buffer,
    pub indices: wgpu::Buffer,
    pub draws: Vec<DrawRange>,
}

impl Renderer {
    pub fn create_mesh(&mut self, desc: &MeshDesc<'_>) -> MeshHandle {
        let vertices = self.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some(desc.label),
            contents: bytemuck::cast_slice(desc.vertices),
            usage: wgpu::BufferUsages::VERTEX,
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
        MeshHandle::from_raw(self.meshes.insert(GpuMesh { vertices, indices, draws: desc.draws.clone() }))
    }

    pub fn destroy_mesh(&mut self, handle: MeshHandle) {
        self.meshes.remove(handle.raw());
    }
}

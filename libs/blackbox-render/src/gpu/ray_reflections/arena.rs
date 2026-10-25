//! Stable hit-data addresses, partial updates and rare capacity growth.
use super::{
    atlas::Atlas,
    bindings::Bindings,
    mesh::{Mesh, Vertex},
    ranges::Ranges,
};
use std::{collections::HashMap, ops::Range};
struct Record {
    vertices: Range<usize>,
    triangles: Range<usize>,
}
#[derive(Default)]
pub(super) struct Arena {
    records: HashMap<usize, Record>,
    vertices: Vec<Vertex>,
    triangles: Vec<[u32; 4]>,
    vertex_ranges: Ranges,
    triangle_ranges: Ranges,
    changed: Vec<usize>,
}
impl Arena {
    pub fn len(&self) -> usize {
        self.records.len()
    }
    pub fn ids(&self) -> impl Iterator<Item = &usize> {
        self.records.keys()
    }
    pub fn offset(&self, id: usize) -> Option<u32> {
        self.records.get(&id).map(|r| r.triangles.start as u32)
    }
    pub fn remove(&mut self, id: usize) {
        if let Some(record) = self.records.remove(&id) {
            self.vertex_ranges.release(record.vertices);
            self.triangle_ranges.release(record.triangles);
        }
    }
    pub fn retain(&mut self, ids: &[usize]) {
        let remove: Vec<_> = self.records.keys().copied().filter(|id| ids.binary_search(id).is_err()).collect();
        for id in remove {
            self.remove(id);
        }
    }
    pub fn ensure(
        &mut self,
        id: usize,
        mesh: &Mesh,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        (atlas, detail, detail_textures): (&mut Atlas, &mut Atlas, &std::collections::HashSet<usize>),
        views: &HashMap<usize, wgpu::TextureView>,
    ) -> bool {
        if self.records.contains_key(&id) {
            return true;
        }
        let limit = device.limits().max_storage_buffer_binding_size as usize;
        if !self.vertex_ranges.fits(mesh.vertices.len(), limit / 64)
            || !self.triangle_ranges.fits(mesh.triangles.len(), limit / 16)
        {
            return false;
        }
        let vertices = self.vertex_ranges.allocate(mesh.vertices.len());
        let triangles = self.triangle_ranges.allocate(mesh.triangles.len());
        self.vertices.resize(self.vertex_ranges.end, bytemuck::Zeroable::zeroed());
        self.triangles.resize(self.triangle_ranges.end, [0; 4]);
        self.vertices[vertices.clone()].copy_from_slice(&mesh.vertices);
        let white = views.get(&0).and_then(|v| atlas.layer(device, queue, 0, v)).unwrap_or(0);
        for vertex in &mut self.vertices[vertices.clone()] {
            if vertex.uv[2] <= 0.0 {
                continue;
            }
            let handle = vertex.uv[2] as usize - 1;
            let layer = views.get(&handle).and_then(|view| {
                if detail_textures.contains(&handle) && detail.can_fit(handle) {
                    return detail.layer(device, queue, handle, view).map(|layer| layer | 32768);
                }
                atlas.layer(device, queue, handle, view)
            });
            vertex.uv[2] = layer.map_or(0.0, |layer| (layer + 1) as f32);
        }
        let base = vertices.start as u32;
        for (dst, t) in self.triangles[triangles.clone()].iter_mut().zip(&mesh.triangles) {
            let handle = (t[3] & 65535) as usize;
            let layer = views
                .get(&handle)
                .and_then(|v| {
                    if detail_textures.contains(&handle) && detail.can_fit(handle) {
                        return detail.layer(device, queue, handle, v).map(|layer| layer | 32768);
                    }
                    atlas.layer(device, queue, handle, v)
                })
                .unwrap_or(white);
            *dst = [t[0] + base, t[1] + base, t[2] + base, (t[3] & !65535) | layer];
        }
        self.records.insert(id, Record { vertices, triangles });
        self.changed.push(id);
        true
    }
    pub fn flush(&mut self, device: &wgpu::Device, queue: &wgpu::Queue, bindings: &mut Bindings) -> bool {
        let limit = device.limits().max_storage_buffer_binding_size;
        let grow = |buffer: &mut wgpu::Buffer, data: &[u8], label| {
            if data.len() as u64 <= buffer.size() {
                return false;
            }
            *buffer = device.create_buffer(&wgpu::BufferDescriptor {
                label: Some(label),
                size: (data.len() as u64).next_power_of_two().min(limit),
                usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            });
            queue.write_buffer(buffer, 0, data);
            true
        };
        let vertices_grew =
            grow(&mut bindings.vertices, bytemuck::cast_slice(&self.vertices), "RT shading vertex arena");
        let triangles_grew =
            grow(&mut bindings.triangles, bytemuck::cast_slice(&self.triangles), "RT shading triangle arena");
        let mut bytes = 0;
        for id in self.changed.drain(..) {
            let record = &self.records[&id];
            if !vertices_grew {
                queue.write_buffer(
                    &bindings.vertices,
                    (record.vertices.start * 64) as u64,
                    bytemuck::cast_slice(&self.vertices[record.vertices.clone()]),
                );
            }
            if !triangles_grew {
                queue.write_buffer(
                    &bindings.triangles,
                    (record.triangles.start * 16) as u64,
                    bytemuck::cast_slice(&self.triangles[record.triangles.clone()]),
                );
            }
            bytes += record.vertices.len() * 64 + record.triangles.len() * 16;
        }
        blackbox_trace::global().counter(
            "Vulkan RT hit-data uploads",
            &[("bytes", bytes as f64), ("buffer_grows", u8::from(vertices_grew || triangles_grew) as f64)],
        );
        vertices_grew || triangles_grew
    }
}

//! Mesh upload and release.

use wgpu::util::DeviceExt;

use super::Renderer;
use super::draw_mask::DrawMask;
use crate::{DrawRange, MeshDesc, MeshHandle};

pub(super) struct GpuMesh {
    pub vertices: wgpu::Buffer,
    pub secondary_uvs: Option<wgpu::Buffer>,
    pub tangents: Option<wgpu::Buffer>,
    pub indices: wgpu::Buffer,
    pub draws: Vec<DrawRange>,
    /// Which draw lists each of `draws` belongs in, worked out once here instead of in every pass.
    pub draw_masks: Vec<DrawMask>,
    /// Whether any draw is glossy: its runs of instances need their matrices written for the glossy pipelines.
    pub has_glossy: bool,
    /// Whether any draw is single-sided: its runs of instances split where the mirroring changes.
    pub single_sided: bool,
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
        // Index buffers must be a multiple of 4 bytes: only an odd count needs a padded copy.
        let indices = pad_indices(desc.indices);
        let indices = self.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some(desc.label),
            contents: bytemuck::cast_slice(&indices[..]),
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
        let single_sided = desc.draws.iter().any(|draw| !draw.two_sided);
        if single_sided {
            self.pipelines.enable_culling();
        }
        MeshHandle::from_raw(self.meshes.insert(GpuMesh {
            vertices,
            secondary_uvs,
            tangents,
            indices,
            draws: desc.draws.clone(),
            draw_masks: desc.draws.iter().map(DrawMask::of).collect(),
            has_glossy: desc.draws.iter().any(|draw| matches!(draw.shading, crate::Shading::Glossy(_))),
            single_sided,
            center,
            radius,
        }))
    }

    pub fn destroy_mesh(&mut self, handle: MeshHandle) {
        self.meshes.remove(handle.raw());
    }
}

/// The indices as an index buffer holds them: an odd count gets one more index so the buffer is a whole number of
/// 4-byte words. An even count is used as it is, without a copy.
fn pad_indices(indices: &[u16]) -> std::borrow::Cow<'_, [u16]> {
    if indices.len() % 2 == 1 {
        let mut padded = Vec::with_capacity(indices.len() + 1);
        padded.extend_from_slice(indices);
        padded.push(0);
        return std::borrow::Cow::Owned(padded);
    }
    std::borrow::Cow::Borrowed(indices)
}

#[cfg(test)]
mod tests {
    use super::pad_indices;
    use std::borrow::Cow;

    #[test]
    fn an_even_index_count_is_not_copied() {
        let indices = [0u16, 1, 2, 3];
        assert!(matches!(pad_indices(&indices), Cow::Borrowed(_)));
    }

    #[test]
    fn an_odd_index_count_gets_one_zero() {
        let indices = [0u16, 1, 2];
        assert_eq!(&pad_indices(&indices)[..], &[0, 1, 2, 0]);
    }
}

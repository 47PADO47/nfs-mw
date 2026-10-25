//! Bounded reflection-only texture array. Raster textures keep their original resolution.
use std::collections::HashMap;

pub(super) struct Atlas {
    texture: wgpu::Texture,
    pub view: wgpu::TextureView,
    pub sampler: wgpu::Sampler,
    pipeline: wgpu::RenderPipeline,
    layers: HashMap<usize, u32>,
    free: Vec<u32>,
}
impl Atlas {
    pub fn new(device: &wgpu::Device) -> Self {
        Self::sized(device, 128, 2048)
    }
    pub fn detail(device: &wgpu::Device) -> Self {
        Self::quality(device, 1)
    }
    pub fn quality(device: &wgpu::Device, quality: u32) -> Self {
        let (size, capacity) = super::cache::budget(quality);
        Self::sized(device, size, capacity)
    }
    pub fn capacity(&self) -> usize {
        self.texture.depth_or_array_layers() as usize
    }
    pub fn bytes(&self) -> u64 {
        let size = self.texture.width();
        (0..self.texture.mip_level_count()).map(|m| u64::from((size >> m).max(1)).pow(2) * 4).sum::<u64>()
            * self.capacity() as u64
    }
    pub fn retain(&mut self, handles: &std::collections::HashSet<usize>) {
        let removed: Vec<_> = self.layers.keys().copied().filter(|h| !handles.contains(h)).collect();
        for handle in removed {
            self.forget(handle);
        }
    }
    pub fn len(&self) -> usize {
        self.layers.len()
    }
    pub fn can_fit(&self, handle: usize) -> bool {
        self.layers.contains_key(&handle) || !self.free.is_empty()
    }
    fn sized(device: &wgpu::Device, size: u32, capacity: u32) -> Self {
        let count = device.limits().max_texture_array_layers.min(capacity);
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("RT reflection texture atlas"),
            size: wgpu::Extent3d { width: size, height: size, depth_or_array_layers: count },
            mip_level_count: size.ilog2() + 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        });
        let view = texture.create_view(&wgpu::TextureViewDescriptor {
            dimension: Some(wgpu::TextureViewDimension::D2Array),
            ..Default::default()
        });
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            address_mode_u: wgpu::AddressMode::Repeat,
            address_mode_v: wgpu::AddressMode::Repeat,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::MipmapFilterMode::Linear,
            ..Default::default()
        });
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("RT texture resample"),
            source: wgpu::ShaderSource::Wgsl(include_str!("../../shaders/ray_atlas.wgsl").into()),
        });
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("RT texture resample"),
            layout: None,
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs"),
                compilation_options: Default::default(),
                buffers: &[],
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs"),
                compilation_options: Default::default(),
                targets: &[Some(wgpu::TextureFormat::Rgba8Unorm.into())],
            }),
            primitive: Default::default(),
            depth_stencil: None,
            multisample: Default::default(),
            multiview_mask: None,
            cache: None,
        });
        Self { texture, view, sampler, pipeline, layers: HashMap::new(), free: (0..count).rev().collect() }
    }
    pub fn layer(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        handle: usize,
        view: &wgpu::TextureView,
    ) -> Option<u32> {
        if let Some(layer) = self.layers.get(&handle) {
            return Some(*layer);
        }
        let Some(layer) = self.free.pop() else {
            log::warn!("RT reflection texture atlas is full");
            return None;
        };
        let group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("RT atlas source"),
            layout: &self.pipeline.get_bind_group_layout(0),
            entries: &[
                wgpu::BindGroupEntry { binding: 0, resource: wgpu::BindingResource::TextureView(view) },
                wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::Sampler(&self.sampler) },
            ],
        });
        let mut encoder = device.create_command_encoder(&Default::default());
        for mip in 0..self.texture.mip_level_count() {
            let target = self.texture.create_view(&wgpu::TextureViewDescriptor {
                dimension: Some(wgpu::TextureViewDimension::D2),
                base_array_layer: layer,
                array_layer_count: Some(1),
                base_mip_level: mip,
                mip_level_count: Some(1),
                ..Default::default()
            });
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("RT atlas mip"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &target,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::WHITE),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                ..Default::default()
            });
            pass.set_pipeline(&self.pipeline);
            pass.set_bind_group(0, &group, &[]);
            pass.draw(0..3, 0..1);
        }
        queue.submit([encoder.finish()]);
        self.layers.insert(handle, layer);
        Some(layer)
    }
    pub fn forget(&mut self, handle: usize) {
        if let Some(layer) = self.layers.remove(&handle) {
            self.free.push(layer);
        }
    }
}

#[cfg(test)]
#[path = "atlas_tests.rs"]
mod tests;

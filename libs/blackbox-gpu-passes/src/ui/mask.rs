//! Persistent RGBA8 mask composition, before the existing SDR/HDR/coverage UI draw.
use super::{UiPass, UiTexture};
use blackbox_gfx::UiTextureComposition;
use std::collections::HashMap;

#[cfg(test)]
#[path = "mask_tests.rs"]
mod tests;

pub(super) struct MaskPass {
    pipeline: wgpu::ComputePipeline,
    layout: wgpu::BindGroupLayout,
    /// Per output texture, what its last composition bound.
    compositions: HashMap<u64, Composition>,
}

/// The inputs of one output texture: valid while its picture, mask and output are the textures it was made for.
struct Composition {
    textures: [wgpu::Texture; 3],
    uniform: wgpu::Buffer,
    group: wgpu::BindGroup,
}

impl Composition {
    fn uses(&self, textures: [&wgpu::Texture; 3]) -> bool {
        self.textures.iter().zip(textures).all(|(made, now)| made == now)
    }
}

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct Params {
    window: [f32; 4],
    pivot: [f32; 2],
    sin_cos: [f32; 2],
    additive: [u32; 4],
}

impl MaskPass {
    fn new(device: &wgpu::Device, cache: Option<&wgpu::PipelineCache>) -> Self {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("UI mask composition"),
            source: wgpu::ShaderSource::Wgsl(include_str!("../shaders/ui_mask.wgsl").into()),
        });
        let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("UI mask composition"),
            layout: None,
            module: &shader,
            entry_point: Some("compose"),
            compilation_options: Default::default(),
            cache,
        });
        let layout = pipeline.get_bind_group_layout(0);
        Self { pipeline, layout, compositions: HashMap::new() }
    }

    fn compose(&self, device: &wgpu::Device, textures: [&wgpu::Texture; 3]) -> Composition {
        let uniform = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("UI mask parameters"),
            size: size_of::<Params>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let views = textures.map(|texture| texture.create_view(&Default::default()));
        let group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("UI mask composition"),
            layout: &self.layout,
            entries: &[
                wgpu::BindGroupEntry { binding: 0, resource: wgpu::BindingResource::TextureView(&views[0]) },
                wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::TextureView(&views[1]) },
                wgpu::BindGroupEntry { binding: 2, resource: wgpu::BindingResource::TextureView(&views[2]) },
                wgpu::BindGroupEntry { binding: 3, resource: uniform.as_entire_binding() },
            ],
        });
        Composition { textures: textures.map(Clone::clone), uniform, group }
    }

    /// Forget the inputs kept for `output`, which is gone.
    pub(super) fn forget(&mut self, output: u64) {
        self.compositions.remove(&output);
    }

    fn encode(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        encoder: &mut wgpu::CommandEncoder,
        command: &UiTextureComposition,
        [picture, mask, output]: [&UiTexture; 3],
    ) {
        let (sin, cos) = command.degrees.to_radians().sin_cos();
        let params = Params {
            window: command.window,
            pivot: command.pivot,
            sin_cos: [sin, cos],
            additive: [u32::from(command.additive), 0, 0, 0],
        };
        // One buffer per output texture, rewritten for each composition: the pending compositions are keyed by
        // output, so an encoder holds at most one dispatch of a slot.
        let textures = [&picture.texture, &mask.texture, &output.texture];
        let key = command.output.raw();
        if self.compositions.get(&key).is_none_or(|made| !made.uses(textures)) {
            let made = self.compose(device, textures);
            self.compositions.insert(key, made);
        }
        let Some(composition) = self.compositions.get(&key) else { return };
        queue.write_buffer(&composition.uniform, 0, bytemuck::bytes_of(&params));
        let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
            label: Some("UI mask composition / premultiply"),
            timestamp_writes: None,
        });
        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, &composition.group, &[]);
        pass.dispatch_workgroups(output.size[0].div_ceil(8), output.size[1].div_ceil(8), 1);
    }
}

impl UiPass {
    pub(super) fn compose_textures(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        encoder: &mut wgpu::CommandEncoder,
    ) {
        for (_, command) in std::mem::take(&mut self.pending_compositions) {
            if command.output == command.picture
                || command.output == command.mask
                || !command.pivot.into_iter().chain(command.window).chain([command.degrees]).all(f32::is_finite)
            {
                log::warn!("invalid UI mask composition for {}", command.output.raw());
                continue;
            }
            let Some(picture) = self.textures.get(&command.picture.raw()) else { continue };
            if !self.textures.contains_key(&command.mask.raw()) {
                continue;
            }
            let size = picture.size;
            if self.textures.get(&command.output.raw()).is_none_or(|t| t.size != size) {
                self.create_texture(device, command.output, size);
            }
            let pass = self.mask_pass.get_or_insert_with(|| MaskPass::new(device, self.cache.as_ref()));
            pass.encode(
                device,
                queue,
                encoder,
                &command,
                [
                    &self.textures[&command.picture.raw()],
                    &self.textures[&command.mask.raw()],
                    &self.textures[&command.output.raw()],
                ],
            );
        }
    }
}

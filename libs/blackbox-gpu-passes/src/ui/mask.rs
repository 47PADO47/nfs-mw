//! Persistent RGBA8 mask composition, before the existing SDR/HDR/coverage UI draw.
use super::{UiPass, UiTexture};
use blackbox_gfx::UiTextureComposition;
use wgpu::util::DeviceExt;

#[cfg(test)]
#[path = "mask_tests.rs"]
mod tests;

pub(super) struct MaskPass {
    pipeline: wgpu::ComputePipeline,
    layout: wgpu::BindGroupLayout,
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
        Self { pipeline, layout }
    }

    fn encode(
        &self,
        device: &wgpu::Device,
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
        // One immutable buffer per dispatch: later compositions of the same slot in this encoder
        // cannot overwrite parameters still needed by an earlier dispatch.
        let uniform = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("UI mask parameters"),
            contents: bytemuck::bytes_of(&params),
            usage: wgpu::BufferUsages::UNIFORM,
        });
        let views = [picture, mask, output].map(|t| t.texture.create_view(&Default::default()));
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
        let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
            label: Some("UI mask composition / premultiply"),
            timestamp_writes: None,
        });
        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, &group, &[]);
        pass.dispatch_workgroups(output.size[0].div_ceil(8), output.size[1].div_ceil(8), 1);
    }
}

impl UiPass {
    pub(super) fn compose_textures(&mut self, device: &wgpu::Device, encoder: &mut wgpu::CommandEncoder) {
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

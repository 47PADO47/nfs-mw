//! A picture drawn on the GPU (a view capture) shown as a UI texture: tinted and masked in one compute pass, with
//! no readback. The result is an ordinary UI texture, premultiplied, that the UI pass draws like any other.
use super::{UiPass, UiTexture, UiTextureError};
use blackbox_gfx::UiTextureId;
use wgpu::util::DeviceExt;

#[cfg(test)]
#[path = "picture_tests.rs"]
mod tests;

pub(super) struct PicturePass {
    pipeline: wgpu::ComputePipeline,
    layout: wgpu::BindGroupLayout,
}

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct Params {
    tint: [u32; 4],
}

impl PicturePass {
    fn new(device: &wgpu::Device, cache: Option<&wgpu::PipelineCache>) -> Self {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("UI picture"),
            source: wgpu::ShaderSource::Wgsl(include_str!("../shaders/ui_picture.wgsl").into()),
        });
        let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("UI picture"),
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
        picture: &wgpu::TextureView,
        [mask, output]: [&UiTexture; 2],
        tint: [u8; 3],
    ) {
        let params = Params { tint: [u32::from(tint[0]), u32::from(tint[1]), u32::from(tint[2]), 0] };
        // One immutable buffer per dispatch, as the mask pass does: a later dispatch cannot change these.
        let uniform = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("UI picture parameters"),
            contents: bytemuck::bytes_of(&params),
            usage: wgpu::BufferUsages::UNIFORM,
        });
        let views = [mask, output].map(|t| t.texture.create_view(&Default::default()));
        let group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("UI picture"),
            layout: &self.layout,
            entries: &[
                wgpu::BindGroupEntry { binding: 0, resource: wgpu::BindingResource::TextureView(picture) },
                wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::TextureView(&views[0]) },
                wgpu::BindGroupEntry { binding: 2, resource: wgpu::BindingResource::TextureView(&views[1]) },
                wgpu::BindGroupEntry { binding: 3, resource: uniform.as_entire_binding() },
            ],
        });
        let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
            label: Some("UI picture / tint and mask"),
            timestamp_writes: None,
        });
        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, &group, &[]);
        pass.dispatch_workgroups(output.size[0].div_ceil(8), output.size[1].div_ceil(8), 1);
    }
}

impl UiPass {
    /// Whether UI texture `id` exists and is exactly `size` (width, height) pixels.
    pub fn has_texture_of_size(&self, id: UiTextureId, size: (u32, u32)) -> bool {
        self.textures.get(&id.raw()).is_some_and(|t| t.size == [size.0, size.1])
    }

    /// Records into `encoder` a pass that writes UI texture `output` from `picture`, an `Rgba8Unorm` view of
    /// `size` pixels that is done being drawn: each colour channel is multiplied by `tint` (255 keeps it) and
    /// then by the alpha of the UI texture `mask`, which must be the same size, and the result's alpha is the
    /// mask's. `output` is created, or resized, to `size`. Nothing is recorded when the mask is missing or the
    /// wrong size, or is `output` itself.
    #[allow(clippy::too_many_arguments)]
    pub fn compose_picture(
        &mut self,
        device: &wgpu::Device,
        encoder: &mut wgpu::CommandEncoder,
        picture: &wgpu::TextureView,
        size: (u32, u32),
        mask: UiTextureId,
        tint: [u8; 3],
        output: UiTextureId,
    ) -> Result<(), UiTextureError> {
        if mask == output || !self.has_texture_of_size(mask, size) {
            return Err(UiTextureError::RegionDoesNotFit { id: mask.raw() });
        }
        let extent = [size.0, size.1];
        if self.textures.get(&output.raw()).is_none_or(|t| t.size != extent) {
            self.create_texture(device, output, extent);
        }
        let pass = self.picture_pass.get_or_insert_with(|| PicturePass::new(device, self.cache.as_ref()));
        pass.encode(device, encoder, picture, [&self.textures[&mask.raw()], &self.textures[&output.raw()]], tint);
        Ok(())
    }
}

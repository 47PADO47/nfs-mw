//! One camera depth/normal prepass shared by ambient occlusion and contact visibility.
mod prepass;
pub(super) mod targets;

use super::Renderer;
use blackbox_gfx::{FrameParams, GraphicsApi, Instance, Projection};

pub(super) fn mask(device: &wgpu::Device, size: (u32, u32), label: &str) -> wgpu::TextureView {
    device
        .create_texture(&wgpu::TextureDescriptor {
            label: Some(label),
            size: wgpu::Extent3d { width: size.0, height: size.1, depth_or_array_layers: 1 },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::R8Unorm,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                | wgpu::TextureUsages::TEXTURE_BINDING
                | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        })
        .create_view(&Default::default())
}

pub(super) struct Surfaces {
    prepass: prepass::Prepass,
    pub targets: targets::Targets,
}

impl Renderer {
    pub(super) fn screen_lighting_active(&self, frame: &FrameParams) -> bool {
        self.info.api != GraphicsApi::Gl && matches!(frame.projection, Projection::PerspectiveInfiniteReverse { .. })
    }

    pub(super) fn encode_surfaces(
        &mut self,
        encoder: &mut wgpu::CommandEncoder,
        size: (u32, u32),
        frame: &FrameParams,
        instances: &[Instance],
        depth: &wgpu::TextureView,
    ) {
        if !self.screen_lighting_active(frame) || !(self.experiments.contact_shadows || self.graphics.ao.active()) {
            self.surfaces = None;
            return;
        }
        let reuse_depth =
            self.reuse_prepass_depth && self.upscale.texture_lod_bias == 0.0 && !self.experiments.screen_reflections;
        let make_targets = || match reuse_depth {
            true => targets::Targets::with_depth(&self.device, size, depth.clone()),
            false => targets::Targets::new(&self.device, size),
        };
        let mut surfaces = self.surfaces.take().unwrap_or_else(|| Surfaces {
            prepass: prepass::Prepass::new(
                &self.device,
                &self.shared.bindings.texture_layout,
                self.pipeline_cache.as_ref(),
            ),
            targets: make_targets(),
        });
        if surfaces.targets.size != size {
            surfaces.targets = make_targets();
        }
        if reuse_depth {
            surfaces.targets.depth = depth.clone();
        }
        if !reuse_depth && surfaces.targets.depth == *depth {
            surfaces.targets = targets::Targets::new(&self.device, size);
        }
        surfaces.prepass.encode(self, encoder, &surfaces.targets, frame, instances);
        self.surfaces = Some(surfaces);
    }
}

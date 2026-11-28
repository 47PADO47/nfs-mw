//! One camera depth/normal prepass shared by ambient occlusion and contact visibility.
mod prepass;
pub(super) mod static_depth;
pub(super) mod targets;

use super::Renderer;
use blackbox_gfx::{FrameParams, GraphicsApi, Instance, Projection};

pub(super) fn mask(device: &wgpu::Device, size: (u32, u32), label: &str) -> wgpu::TextureView {
    mask_with_format(device, size, label, wgpu::TextureFormat::R8Unorm)
}

pub(super) fn mask_with_format(
    device: &wgpu::Device,
    size: (u32, u32),
    label: &str,
    format: wgpu::TextureFormat,
) -> wgpu::TextureView {
    device
        .create_texture(&wgpu::TextureDescriptor {
            label: Some(label),
            size: wgpu::Extent3d { width: size.0, height: size.1, depth_or_array_layers: 1 },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format,
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
    /// The depth of the static scene alone, for XeGTAO horizons that a dynamic model hides.
    pub static_depth: Option<static_depth::StaticDepth>,
}

impl Renderer {
    pub(super) fn screen_lighting_active(&self, frame: &FrameParams) -> bool {
        self.info.api != GraphicsApi::Gl && matches!(frame.projection, Projection::PerspectiveInfiniteReverse { .. })
    }

    pub(super) fn encode_surfaces(
        &mut self,
        encoder: &mut wgpu::CommandEncoder,
        size: (u32, u32),
        (frame, mip_bias): (&FrameParams, f32),
        instances: &[Instance],
        depth: &wgpu::TextureView,
    ) {
        if !self.screen_lighting_active(frame)
            || !(self.contact_active(frame)
                || self.ray_shadows_active(frame)
                || self.cascade_history_active(frame)
                || self.graphics.ao.active())
        {
            self.surfaces = None;
            return;
        }
        // The prepass samples cutouts with the colour pass's LOD bias (`mip_bias`), so it keeps no fragment the
        // colour pass discards and the main view can test against its depth whatever the bias.
        let reuse_depth = self.reuse_prepass_depth && !self.experiments.screen_reflections;
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
            static_depth: None,
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
        let split = self.graphics.ao.active() && self.graphics.ao.method.is_xe() && static_depth_enabled();
        match split {
            true => {
                let cache = self.pipeline_cache.as_ref();
                surfaces.static_depth.get_or_insert_with(|| static_depth::StaticDepth::new(&self.device, cache));
            }
            false => surfaces.static_depth = None,
        }
        surfaces.prepass.encode(
            self,
            encoder,
            &surfaces.targets,
            (frame, mip_bias),
            instances,
            surfaces.static_depth.as_mut(),
        );
        self.surfaces = Some(surfaces);
    }
}

/// `BLACKBOX_AO_STATIC_DEPTH=0` leaves the static depth out, so XeGTAO searches the full depth only (for comparisons).
fn static_depth_enabled() -> bool {
    static ENABLED: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    *ENABLED.get_or_init(|| std::env::var("BLACKBOX_AO_STATIC_DEPTH").as_deref() != Ok("0"))
}

//! Independent object/camera motion for screen-space effects, including native/SMAA rendering.
use super::{TemporalFrame, history::History, motion::MotionPass};
use crate::gpu::Renderer;
use blackbox_gfx::{FrameParams, Instance, Projection};
use glam::{Mat4, Vec2};
use std::time::Instant;

pub(in crate::gpu) struct EffectMotion {
    pub view: wgpu::TextureView,
    pub reset: bool,
    pub previous_from_current: Mat4,
    pub jitter_delta: Vec2,
    pass: MotionPass,
    objects: History,
    previous: Option<FrameParams>,
    jitter: Vec2,
    last: Instant,
}

impl EffectMotion {
    fn new(renderer: &Renderer, size: (u32, u32)) -> Self {
        let view = renderer
            .device
            .create_texture(&wgpu::TextureDescriptor {
                label: Some("screen-space effect object motion"),
                size: wgpu::Extent3d { width: size.0, height: size.1, depth_or_array_layers: 1 },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: wgpu::TextureFormat::Rgba16Float,
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
                view_formats: &[],
            })
            .create_view(&Default::default());
        Self {
            view,
            reset: true,
            previous_from_current: Mat4::IDENTITY,
            jitter_delta: Vec2::ZERO,
            pass: MotionPass::with_format(
                &renderer.device,
                &renderer.shared.bindings.texture_layout,
                renderer.pipeline_cache.as_ref(),
                wgpu::TextureFormat::Rgba16Float,
            ),
            objects: History::new(&renderer.device),
            previous: None,
            jitter: Vec2::ZERO,
            last: Instant::now(),
        }
    }
}

impl Renderer {
    pub(in crate::gpu) fn encode_effect_motion(
        &mut self,
        encoder: &mut wgpu::CommandEncoder,
        frame: &FrameParams,
        instances: &[Instance],
    ) {
        let sun_history = self.ray_shadows_active(frame)
            && self.experiments.ray_soft_shadows
            && self.experiments.ray_shadow_temporal
            && self.experiments.ray_shadow_filter > 0.0;
        if !(self.graphics.ao.method.temporal() && self.graphics.ao.active())
            && !sun_history
            && !self.cascade_history_active(frame)
        {
            self.effect_motion = None;
            return;
        }
        let Some(surface) = &self.surfaces else {
            self.effect_motion = None;
            return;
        };
        let size = surface.targets.size;
        let depth = surface.targets.depth.clone();
        let Projection::PerspectiveInfiniteReverse { fov_y, near, .. } = frame.projection else { return };
        let mut effect = self
            .effect_motion
            .take()
            .filter(|e| (e.view.texture().width(), e.view.texture().height()) == size)
            .unwrap_or_else(|| EffectMotion::new(self, size));
        let now = Instant::now();
        effect.reset = frame.camera_cut
            || effect.previous.is_none_or(|f| f.projection != frame.projection)
            || now.duration_since(effect.last).as_secs_f32() > 0.25;
        let jitter = self.temporal.current().map_or(Vec2::ZERO, |t| t.jitter);
        effect.jitter_delta = (effect.jitter - jitter) / Vec2::new(size.0 as f32, size.1 as f32);
        effect.previous_from_current = effect.previous.map_or(Mat4::IDENTITY, |f| f.view * frame.view.inverse());
        let previous = effect.previous.filter(|_| !effect.reset).map_or(frame.view_proj(), |f| f.view_proj());
        let motion = TemporalFrame {
            #[cfg(all(windows, feature = "streamline", streamline_sdk))]
            projection: frame.projection.matrix(),
            #[cfg(all(windows, feature = "streamline", streamline_sdk))]
            camera_to_world: frame.view.inverse(),
            jitter,
            jitter_matrix: super::jitter_matrix(fsr3_wgpu::jitter::projection_offset_ndc(
                jitter,
                glam::UVec2::new(size.0, size.1),
            )),
            unjittered: frame.view_proj(),
            previous,
            near,
            fov_y,
            reset: effect.reset,
            delta_time: now.duration_since(effect.last).as_secs_f32(),
            sharpness: None,
        };
        effect.objects.upload(&self.device, &self.queue, instances, effect.reset);
        effect.pass.encode(self, encoder, (&effect.view, &depth), &motion, instances, &effect.objects);
        effect.previous = Some(*frame);
        effect.jitter = jitter;
        effect.last = now;
        self.effect_motion = Some(effect);
    }
}

//! The temporal upscaler's side of a frame (FSR 3): the sub-pixel jitter of the projection, the camera and
//! object motion that become motion vectors, and the reset on camera cuts.
//!
//! A frame is drawn temporally when the upscaler is FSR 3, the frame goes to the window (a capture or a
//! mirror has no history to accumulate), and it has a perspective projection. Then the whole frame, the colour
//! pass, the depth prepass, contact shadows and the particles, draws with the same jittered projection
//! ([`Renderer::world_to_clip`]), and the motion pass writes the vectors from the unjittered matrices. Any other
//! frame draws as before, and the next temporal frame starts from a reset.

mod effects;
mod history;
pub(in crate::gpu) use effects::EffectMotion;
mod motion;
#[cfg(test)]
mod tests;

use std::time::Instant;

use blackbox_gfx::{FrameParams, Instance, Projection, Upscaler};
use glam::{Mat4, UVec2, Vec2};

use super::Renderer;
use super::targets::SceneTargets;
use history::History;
pub(super) use motion::MotionPass;

/// What the upscale and the motion pass need to know about the frame being drawn.
#[derive(Clone, Copy, Debug)]
pub(in crate::gpu) struct TemporalFrame {
    #[cfg(all(windows, feature = "streamline", streamline_sdk))]
    pub projection: Mat4,
    #[cfg(all(windows, feature = "streamline", streamline_sdk))]
    pub camera_to_world: Mat4,
    /// The sub-pixel offset of this frame's projection, in render pixels (right and down).
    pub jitter: Vec2,
    /// Moves clip space by the jitter after the projection.
    pub jitter_matrix: Mat4,
    /// World to clip without the jitter, this frame and the last temporal frame (the same on the first).
    pub unjittered: Mat4,
    pub previous: Mat4,
    pub near: f32,
    pub fov_y: f32,
    /// The history is not to be trusted: a camera cut, or the first temporal frame after other ones.
    pub reset: bool,
    pub delta_time: f32,
    /// The upscaler's own sharpening, or `None` for none.
    pub sharpness: Option<f32>,
}

/// The state that carries over from one temporal frame to the next.
#[derive(Default)]
pub(super) struct Temporal {
    /// The frame being drawn, if it is a temporal one.
    frame: Option<TemporalFrame>,
    /// World to clip of the last temporal frame, without jitter.
    previous: Option<Mat4>,
    /// How many temporal frames came before: picks the jitter.
    index: u32,
    last: Option<Instant>,
    /// Created by the first motion pass, so a renderer that never upscales temporally pays nothing.
    history: Option<History>,
    motion: Option<MotionPass>,
}

impl Temporal {
    /// Seconds since the previous temporal frame (0 for the first).
    fn tick(&mut self) -> f32 {
        let now = Instant::now();
        let delta = self.last.map_or(0.0, |last| now.duration_since(last).as_secs_f32());
        self.last = Some(now);
        delta
    }

    /// The temporal frame being drawn, for the upscale pass.
    pub(super) fn current(&self) -> Option<&TemporalFrame> {
        self.frame.as_ref()
    }

    /// Forget the last frame: this one was not drawn temporally, so the next starts a new history.
    pub(super) fn stop(&mut self) {
        self.frame = None;
        self.previous = None;
        self.last = None;
        if let Some(history) = &mut self.history {
            history.forget();
        }
    }
}

/// The matrix that moves clip space by `ndc` after the projection (see `fsr3_wgpu::jitter`).
fn jitter_matrix(ndc: Vec2) -> Mat4 {
    Mat4::from_translation(ndc.extend(0.0))
}

impl Renderer {
    /// Whether FSR 3 is the upscaler and this renderer can run it.
    pub(super) fn temporal_wanted(&self) -> bool {
        matches!(self.upscaler(), Upscaler::Fsr3 | Upscaler::Dlss) && self.caps.upscalers.contains(self.upscaler())
    }

    /// Decide whether the frame about to be drawn is a temporal one, and if so jitter it. `onscreen` is false
    /// for captures (a screenshot, the rear-view mirror), which draw at the output size with no history. They
    /// are drawn between the window's frames, so they leave the history alone.
    pub(super) fn begin_temporal(&mut self, onscreen: bool, frame: &FrameParams) {
        let _trace = blackbox_trace::scope("Renderer CPU: begin temporal frame");
        #[cfg(all(windows, feature = "streamline", streamline_sdk))]
        if self.upscaler() == Upscaler::Dlss
            && self.dlss.as_ref().is_some_and(|runtime| runtime.failed.load(std::sync::atomic::Ordering::Acquire))
        {
            self.caps.upscalers.remove(Upscaler::Dlss);
            let settings = self.graphics;
            let fallback = blackbox_gfx::RenderBackend::apply_graphics(self, &settings);
            log::warn!("DLSS evaluation failed; using {:?}", fallback.effective.upscaler);
        }
        if !onscreen {
            self.temporal.frame = None;
            return;
        }
        self.temporal.frame = self.next_temporal_frame(frame);
        if self.temporal.frame.is_none() {
            self.temporal.stop();
        }
    }

    fn next_temporal_frame(&mut self, frame: &FrameParams) -> Option<TemporalFrame> {
        if !self.post.has_temporal() {
            return None;
        }
        let Projection::PerspectiveInfiniteReverse { fov_y, near, .. } = frame.projection else { return None };
        let scene = self.targets.offscreen().filter(|targets| targets.motion.is_some())?;
        let render = UVec2::new(scene.size.0, scene.size.1);
        let output = UVec2::new(self.surface_size().0, self.surface_size().1);
        if render.x > output.x || render.y > output.y {
            return None;
        }
        let jitter = fsr3_wgpu::jitter::offset_for_sizes(self.temporal.index, render, output);
        self.temporal.index = self.temporal.index.wrapping_add(1);
        let unjittered = frame.view_proj();
        let previous = self.temporal.previous.replace(unjittered);
        let sharpness = self.upscale_sharpness();
        Some(TemporalFrame {
            #[cfg(all(windows, feature = "streamline", streamline_sdk))]
            projection: frame.projection.matrix(),
            #[cfg(all(windows, feature = "streamline", streamline_sdk))]
            camera_to_world: frame.view.inverse(),
            jitter,
            jitter_matrix: jitter_matrix(fsr3_wgpu::jitter::projection_offset_ndc(jitter, render)),
            unjittered,
            previous: previous.unwrap_or(unjittered),
            near,
            fov_y,
            reset: frame.camera_cut || previous.is_none(),
            delta_time: self.temporal.tick(),
            sharpness: (sharpness > 0.0).then_some(sharpness),
        })
    }

    /// World to clip space for the frame being drawn to the window: `frame`'s own projection, shifted by
    /// this frame's jitter when it is a temporal one. Views of the scene from elsewhere (reflections) use
    /// `frame.view_proj()` itself.
    pub(super) fn world_to_clip(&self, frame: &FrameParams) -> Mat4 {
        match &self.temporal.frame {
            Some(temporal) => temporal.jitter_matrix * frame.view_proj(),
            None => frame.view_proj(),
        }
    }

    /// Record the motion vectors of a temporal frame: `instances` over the depth the scene pass left.
    pub(super) fn encode_motion(
        &mut self,
        encoder: &mut wgpu::CommandEncoder,
        targets: Option<&SceneTargets>,
        instances: &[Instance],
    ) {
        let Some(frame) = self.temporal.frame else { return };
        let Some(scene) = targets.unwrap_or(&self.targets).offscreen() else { return };
        let Some(motion) = &scene.motion else { return };
        let mut history = self.temporal.history.take().unwrap_or_else(|| History::new(&self.device));
        history.upload(&self.device, &self.queue, instances, frame.reset);
        let mut pass = match self.temporal.motion.take() {
            Some(pass) => pass,
            None => MotionPass::new(&self.device, &self.shared.bindings.texture_layout, self.pipeline_cache.as_ref()),
        };
        pass.encode(self, encoder, (&motion.view, &scene.depth_view), &frame, instances, &history);
        self.temporal.motion = Some(pass);
        self.temporal.history = Some(history);
    }

    /// The frame is drawn: nothing of it is a temporal frame any more.
    pub(super) fn end_temporal(&mut self) {
        let _trace = blackbox_trace::scope("Renderer CPU: end temporal frame");
        self.temporal.frame = None;
    }
}

//! Showing the last finished scene image again. A caller that knows nothing in the picture changed asks for
//! [`SceneReuse::Keep`] on one frame and [`SceneReuse::Reuse`] on the next ones: those frames copy the kept image into
//! the surface and draw only the UI over it, so a still menu skips the scene, the shadows and the post chain.
//!
//! The renderer keeps the caller honest from its side: anything that changes the picture without a new frame (a resize,
//! settings, a resource created or destroyed, a capture drawn) bumps an epoch, and an image kept at an older epoch is
//! never shown. When in doubt the frame is drawn normally.

use super::Renderer;
use blackbox_gfx::{Antialiasing, SceneReuse};

/// What the renderer remembers about the scene image between frames.
#[derive(Default)]
pub(super) struct Reuse {
    /// What the next frame was asked to do (consumed by `render`).
    request: SceneReuse,
    /// The frame being drawn keeps its image.
    keep: bool,
    /// Bumped by everything that changes the picture without a new frame.
    epoch: u64,
    image: Option<Kept>,
}

/// A copy of a finished scene image: the surface's pixels before the UI went on.
struct Kept {
    texture: wgpu::Texture,
    size: (u32, u32),
    format: wgpu::TextureFormat,
    /// The epoch the image was kept at; `None` while the image does not show the scene.
    valid: Option<u64>,
}

impl Kept {
    fn new(device: &wgpu::Device, size: (u32, u32), format: wgpu::TextureFormat) -> Self {
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("kept scene image"),
            size: wgpu::Extent3d { width: size.0, height: size.1, depth_or_array_layers: 1 },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format,
            usage: wgpu::TextureUsages::COPY_SRC | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        Self { texture, size, format, valid: None }
    }
}

impl Renderer {
    /// The next frame's request.
    pub(super) fn request_scene_reuse(&mut self, mode: SceneReuse) {
        self.reuse.request = mode;
    }

    /// Something that changes the picture without a new frame happened: the kept image no longer shows it.
    pub(super) fn touch_scene(&mut self) {
        self.reuse.epoch = self.reuse.epoch.wrapping_add(1);
    }

    /// Whether this renderer can hold a still image at all now: not under a temporal method (which accumulates
    /// frames), not with frame generation, and only when the output can be copied from and to.
    fn can_reuse(&self) -> bool {
        #[cfg(all(windows, feature = "streamline"))]
        if self.dlss.is_some() {
            return false;
        }
        if self.post.has_temporal() || self.post_effects().antialiasing == Antialiasing::Taa {
            return false;
        }
        self.output.copyable()
    }

    /// Whether a `Reuse` frame would show a kept image now.
    pub(super) fn scene_reuse_ready(&self) -> bool {
        if !self.can_reuse() {
            return false;
        }
        let Some(kept) = &self.reuse.image else { return false };
        kept.valid == Some(self.reuse.epoch) && kept.size == self.surface_size() && kept.format == self.output.format()
    }

    /// Start of a window frame: takes the request. Returns whether this frame shows the kept image. A frame that does
    /// not keep its own image makes the kept one useless, since the scene may have moved on.
    pub(super) fn begin_reuse(&mut self) -> bool {
        let request = std::mem::take(&mut self.reuse.request);
        let show = request == SceneReuse::Reuse && self.scene_reuse_ready();
        self.reuse.keep = request == SceneReuse::Keep && self.can_reuse();
        if !show && !self.reuse.keep {
            self.forget_scene();
        }
        show
    }

    fn forget_scene(&mut self) {
        if let Some(kept) = &mut self.reuse.image {
            kept.valid = None;
        }
    }

    /// The frame is done: nothing keeps its image unless the next `render` asks.
    pub(super) fn end_reuse(&mut self) {
        self.reuse.keep = false;
    }

    /// Copy the finished scene image in `output` (before the UI goes on) into the kept one, when the frame was asked
    /// to keep it.
    pub(super) fn encode_keep(&mut self, encoder: &mut wgpu::CommandEncoder, output: &wgpu::TextureView) {
        if !self.reuse.keep {
            return;
        }
        let texture = output.texture();
        let size = (texture.width(), texture.height());
        let format = texture.format();
        let mut kept = match self.reuse.image.take() {
            Some(kept) if kept.size == size && kept.format == format => kept,
            _ => Kept::new(&self.device, size, format),
        };
        let _trace = blackbox_trace::scope("Renderer CPU: keep scene image");
        encoder.copy_texture_to_texture(texture.as_image_copy(), kept.texture.as_image_copy(), kept.texture.size());
        kept.valid = Some(self.reuse.epoch);
        self.reuse.image = Some(kept);
    }

    /// Draw a frame from the kept image: copy it into `output`, then the UI over it.
    pub(super) fn submit_reuse(&mut self, output: &wgpu::TextureView, size: (u32, u32)) {
        let _trace = blackbox_trace::scope("Renderer CPU: reuse kept scene image");
        let device = self.device.clone();
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: Some("reused frame") });
        if let Some(kept) = &self.reuse.image {
            encoder.copy_texture_to_texture(
                kept.texture.as_image_copy(),
                output.texture().as_image_copy(),
                kept.texture.size(),
            );
        }
        self.encode_ui(&mut encoder, output, size);
        self.queue.submit([encoder.finish()]);
    }
}

#[cfg(test)]
#[path = "reuse_tests.rs"]
mod tests;

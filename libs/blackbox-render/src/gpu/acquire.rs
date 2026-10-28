//! The image a frame's last passes write: a view the caller has, or the window's next swapchain image, acquired
//! once the work that goes into the renderer's own images is recorded, so that recording does not wait behind it.

use super::Renderer;
use super::output::Output;
use crate::RenderError;
use blackbox_gfx::Antialiasing;

/// Where a frame's last passes draw.
pub(super) enum FrameOutput<'a> {
    /// A view known before recording: a headless output, a capture or prewarm image, or a surface image already
    /// acquired.
    View(&'a wgpu::TextureView),
    /// The window's surface, acquired right before the first pass that writes it.
    Surface,
}

/// An image acquired for a frame.
pub(super) enum Acquired {
    /// The window's next image, presented once the frame is submitted.
    Surface(wgpu::SurfaceTexture, wgpu::TextureView),
    /// A view nothing presents: a headless output texture, or the Streamline swapchain's.
    View(wgpu::TextureView),
}

impl Acquired {
    pub(super) fn view(&self) -> &wgpu::TextureView {
        match self {
            Self::Surface(_, view) | Self::View(view) => view,
        }
    }

    /// The surface image to present, if this is one.
    pub(super) fn into_surface(self) -> Option<wgpu::SurfaceTexture> {
        match self {
            Self::Surface(texture, _) => Some(texture),
            Self::View(_) => None,
        }
    }
}

/// What became of a recorded frame.
pub(super) enum Recorded {
    /// Drawn into the view the caller gave.
    Given,
    /// Drawn into the window image acquired after the offscreen work; it is to be presented.
    Acquired(Acquired),
    /// The window had no image: the offscreen work went out alone and nothing is to be presented.
    Skipped,
}

impl Renderer {
    /// Whether this frame acquires the window's image only after recording the work that goes into the renderer's
    /// own images. That needs a scene that is not drawn straight into the output: offscreen targets, or SMAA, whose
    /// input image the scene and the post chain draw into. Streamline keeps its markers around an early acquire, and
    /// after a frame the window had no image for (minimised, resized) the next one asks first, so a hidden window
    /// records nothing.
    pub(super) fn acquires_late(&self) -> bool {
        #[cfg(all(windows, feature = "streamline", streamline_sdk))]
        if self.dlss.is_some() {
            return false;
        }
        if !matches!(self.output, Output::Window { .. }) {
            return false;
        }
        !self.surface_missed
            && (self.post_effects().antialiasing == Antialiasing::Smaa || self.targets.offscreen().is_some())
    }

    /// The image this frame's last passes write. `Ok(None)` skips the frame: the window is minimised, timed out, or
    /// its surface had to be reconfigured.
    pub(super) fn acquire_output(&mut self) -> Result<Option<Acquired>, RenderError> {
        let acquire_started = std::time::Instant::now();
        let acquire_trace = blackbox_trace::scope("Wait: acquire surface");
        let acquired = self.acquire_image(acquire_started)?;
        self.surface_missed = acquired.is_none();
        let Some(acquired) = acquired else { return Ok(None) };
        self.performance.acquire_wait_ms = acquire_started.elapsed().as_secs_f32() * 1000.0;
        drop(acquire_trace);
        Ok(Some(acquired))
    }

    fn acquire_image(&mut self, acquire_started: std::time::Instant) -> Result<Option<Acquired>, RenderError> {
        match &self.output {
            #[cfg(all(windows, feature = "streamline", streamline_sdk))]
            Output::Window { dx12: Some(native), .. } => Ok(Some(Acquired::View(native.view()?))),
            Output::Window { surface, config, .. } => {
                let acquired = surface.get_current_texture();
                blackbox_trace::global().counter(
                    "Surface acquisition",
                    &[("elapsed_ms", acquire_started.elapsed().as_secs_f64() * 1000.0)],
                );
                let outcome = match &acquired {
                    wgpu::CurrentSurfaceTexture::Success(_) => "success",
                    wgpu::CurrentSurfaceTexture::Suboptimal(_) => "suboptimal",
                    wgpu::CurrentSurfaceTexture::Timeout => "timeout",
                    wgpu::CurrentSurfaceTexture::Occluded => "occluded",
                    wgpu::CurrentSurfaceTexture::Outdated => "outdated",
                    wgpu::CurrentSurfaceTexture::Lost => "lost",
                    wgpu::CurrentSurfaceTexture::Validation => "validation",
                };
                if outcome != "success" {
                    blackbox_trace::global().note(
                        "Surface acquisition outcome",
                        &format!(
                            "{outcome}; present_mode={:?}; {}x{}",
                            config.present_mode, config.width, config.height
                        ),
                    );
                }
                let texture = match acquired {
                    wgpu::CurrentSurfaceTexture::Success(t) | wgpu::CurrentSurfaceTexture::Suboptimal(t) => t,
                    wgpu::CurrentSurfaceTexture::Timeout | wgpu::CurrentSurfaceTexture::Occluded => return Ok(None),
                    wgpu::CurrentSurfaceTexture::Outdated | wgpu::CurrentSurfaceTexture::Lost => {
                        #[cfg(all(windows, feature = "streamline", streamline_sdk))]
                        if self.dlss.is_some() {
                            self.presentation.suspend(&self.device);
                        }
                        surface.configure(&self.device, config);
                        return Ok(None);
                    }
                    #[allow(unreachable_patterns)]
                    other => return Err(RenderError::Surface(format!("{other:?}"))),
                };
                let view = texture.texture.create_view(&wgpu::TextureViewDescriptor::default());
                Ok(Some(Acquired::Surface(texture, view)))
            }
            Output::Texture { texture, .. } => {
                Ok(Some(Acquired::View(texture.create_view(&wgpu::TextureViewDescriptor::default()))))
            }
        }
    }
}

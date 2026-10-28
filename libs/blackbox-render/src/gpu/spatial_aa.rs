//! Recording and submitting a frame, with SMAA 1x on the display image before UI. The `blackbox_smaa` target owns
//! the spatial edge and weight buffers.
use super::acquire::{Acquired, FrameOutput, Recorded};
use super::{Renderer, targets::SceneTargets};
use crate::RenderError;
use blackbox_gfx::{Antialiasing, FrameParams, Instance};

pub(super) struct SpatialAa {
    target: blackbox_smaa::SmaaTarget,
    size: (u32, u32),
    format: wgpu::TextureFormat,
}

impl Renderer {
    /// Record one frame into `output`, a view the caller has, and submit it.
    pub(super) fn submit_frame(
        &mut self,
        scene: Option<&SceneTargets>,
        output: &wgpu::TextureView,
        size: (u32, u32),
        frame: &FrameParams,
        instances: &[Instance],
    ) {
        // A given view is never acquired, so the frame can neither fail nor be skipped.
        let _ = self.record_frame(scene, FrameOutput::View(output), size, frame, instances);
    }

    /// Record one frame and submit it after the queued captures. The scene with its shadows, reflections, AO and
    /// motion draws into the renderer's own images (and with SMAA the post chain too, into SMAA's input), so with
    /// [`FrameOutput::Surface`] the window's image is acquired only once they are recorded: right before the post
    /// chain, whose last pass writes it, or with SMAA right before the SMAA blend; the UI follows. A scene drawn
    /// straight into the output acquires it first.
    pub(super) fn record_frame(
        &mut self,
        scene: Option<&SceneTargets>,
        output: FrameOutput<'_>,
        size: (u32, u32),
        frame: &FrameParams,
        instances: &[Instance],
    ) -> Result<Recorded, RenderError> {
        let _trace = blackbox_trace::scope("Renderer CPU: record / submit");
        let onscreen = scene.is_none();
        self.begin_temporal(onscreen, frame);
        let device = self.device.clone();
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: Some("frame") });
        if onscreen && let Some(timing) = &mut self.timing {
            timing.begin(&device, &mut encoder);
        }
        #[cfg(all(windows, feature = "streamline", streamline_sdk))]
        let hudless = self.generation_target(onscreen);
        #[cfg(not(all(windows, feature = "streamline", streamline_sdk)))]
        let hudless: Option<wgpu::TextureView> = None;
        let given = match output {
            FrameOutput::View(view) => Some(view.clone()),
            FrameOutput::Surface => None,
        };
        // What the post chain (or SMAA) writes before the UI: the HUD-less image for frame generation, else the output.
        let scene_output = hudless.clone().or_else(|| given.clone());
        let aa = match self.post_effects().antialiasing == Antialiasing::Smaa {
            true => {
                let format = scene_output.as_ref().map_or(self.output.format(), |view| view.texture().format());
                Some(self.prepare_smaa(format, size))
            }
            false => None,
        };
        let smaa_input = aa.as_ref().and_then(|aa| aa.target.input_view().cloned());
        let offscreen = scene.unwrap_or(&self.targets).offscreen().map(|targets| targets.color_view.clone());
        let mut acquired = None;
        // The scene draws into SMAA's input, else into its offscreen image or the output. Only a scene drawn straight
        // into a window image that is not yet acquired has to wait for it here.
        let scene_view = match smaa_input.clone().or_else(|| scene_output.clone()).or(offscreen) {
            Some(view) => view,
            None => match self.output_view(&mut encoder, onscreen, None, &mut acquired) {
                Ok(Some(view)) => view,
                result => return self.skip_frame(aa, result),
            },
        };
        self.encode_scene(&mut encoder, scene, &scene_view, frame, instances);
        // Without SMAA the post chain's last pass writes the output.
        let post_view = match smaa_input {
            Some(input) => input,
            None => match self.output_view(&mut encoder, onscreen, scene_output.as_ref(), &mut acquired) {
                Ok(Some(view)) => view,
                result => return self.skip_frame(aa, result),
            },
        };
        self.encode_post(&mut encoder, scene, &post_view, size, false);
        let output_view = match self.output_view(&mut encoder, onscreen, given.as_ref(), &mut acquired) {
            Ok(Some(view)) => view,
            result => return self.skip_frame(aa, result),
        };
        if let Some(aa) = &aa {
            // SMAA records into the frame encoder: scene, post, SMAA and UI go out in one submit.
            let _trace = blackbox_trace::scope("Renderer CPU: encode SMAA");
            aa.target.encode_to(&mut encoder, hudless.as_ref().unwrap_or(&output_view));
        }
        self.smaa = aa;
        #[cfg(all(windows, feature = "streamline", streamline_sdk))]
        if let Some(hudless) = &hudless {
            self.composite_generation(&mut encoder, hudless, &output_view);
        }
        if hudless.is_none() {
            if onscreen {
                self.encode_keep(&mut encoder, &output_view);
            }
            self.encode_ui(&mut encoder, &output_view, size);
        }
        self.finish_submit(encoder, onscreen);
        self.end_temporal();
        Ok(match acquired {
            Some(image) => Recorded::Acquired(image),
            None => Recorded::Given,
        })
    }

    /// The SMAA target for `format` and `size`: the last SMAA frame's when its format still matches.
    fn prepare_smaa(&mut self, format: wgpu::TextureFormat, size: (u32, u32)) -> SpatialAa {
        let _trace = blackbox_trace::scope("Renderer CPU: prepare SMAA");
        let cache = self.pipeline_cache.as_ref();
        let mut aa = self.smaa.take().filter(|aa| aa.format == format).unwrap_or_else(|| SpatialAa {
            target: blackbox_smaa::SmaaTarget::new_cached(
                &self.device,
                &self.queue,
                size.0,
                size.1,
                format,
                blackbox_smaa::SmaaMode::Smaa1X,
                cache,
            ),
            size,
            format,
        });
        if aa.size != size {
            aa.target.resize(&self.device, size.0, size.1);
            aa.size = size;
        }
        aa
    }

    /// The view the output passes write: `known` (a given or HUD-less view), else the window's image, acquired now
    /// unless it already was. `Ok(None)` when the window has no image; the frame is then cut short.
    fn output_view(
        &mut self,
        encoder: &mut wgpu::CommandEncoder,
        onscreen: bool,
        known: Option<&wgpu::TextureView>,
        acquired: &mut Option<Acquired>,
    ) -> Result<Option<wgpu::TextureView>, RenderError> {
        if let Some(view) = known.or(acquired.as_ref().map(Acquired::view)) {
            return Ok(Some(view.clone()));
        }
        let Some(image) = self.acquire_mid_frame(encoder, onscreen)? else { return Ok(None) };
        let view = image.view().clone();
        *acquired = Some(image);
        Ok(Some(view))
    }

    /// Acquire the window's image in the middle of a frame. Without one (or on an error) the work recorded so far
    /// still goes out, since the shadow cache, the instance store and the queued captures count on it having run.
    fn acquire_mid_frame(
        &mut self,
        encoder: &mut wgpu::CommandEncoder,
        onscreen: bool,
    ) -> Result<Option<Acquired>, RenderError> {
        let acquired = self.acquire_output();
        if matches!(acquired, Ok(Some(_))) {
            return acquired;
        }
        let unused = self.device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: Some("frame") });
        let recorded = std::mem::replace(encoder, unused);
        self.finish_submit(recorded, onscreen);
        acquired
    }

    /// End a frame cut short for want of a window image. SMAA keeps its target, and the temporal history starts
    /// over: the upscaler, in the post chain, did not run on the frame its inputs were drawn for.
    fn skip_frame(
        &mut self,
        aa: Option<SpatialAa>,
        result: Result<Option<wgpu::TextureView>, RenderError>,
    ) -> Result<Recorded, RenderError> {
        self.smaa = aa;
        self.temporal.stop();
        self.end_temporal();
        result.map(|_| Recorded::Skipped)
    }

    /// Finish `encoder` and submit it after the queued captures, with the GPU timing of an onscreen frame.
    fn finish_submit(&mut self, mut encoder: wgpu::CommandEncoder, onscreen: bool) {
        if onscreen && let Some(timing) = &mut self.timing {
            timing.finish(&mut encoder);
        }
        // Keep command finalization separate from queue submission: both may enter the graphics driver.
        let commands = {
            let _trace = blackbox_trace::scope("GPU API: finish command encoder");
            encoder.finish()
        };
        {
            let _trace = blackbox_trace::scope("GPU API: queue submit");
            self.submit_after_captures(commands);
        }
        if onscreen && let Some(timing) = &mut self.timing {
            timing.submitted();
        }
    }
}

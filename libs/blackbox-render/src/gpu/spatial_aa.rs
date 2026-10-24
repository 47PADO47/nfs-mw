//! SMAA 1x on the display image, before UI. The `blackbox_smaa` target owns the spatial edge and weight buffers.
use super::{Renderer, targets::SceneTargets};
use blackbox_gfx::{Antialiasing, FrameParams, Instance};

pub(super) struct SpatialAa {
    target: blackbox_smaa::SmaaTarget,
    size: (u32, u32),
    format: wgpu::TextureFormat,
}

impl Renderer {
    pub(super) fn submit_frame(
        &mut self,
        scene: Option<&SceneTargets>,
        output: &wgpu::TextureView,
        size: (u32, u32),
        frame: &FrameParams,
        instances: &[Instance],
    ) {
        let _trace = blackbox_trace::scope("Renderer CPU: record / submit");
        self.begin_temporal(scene.is_none(), frame);
        let device = self.device.clone();
        let queue = self.queue.clone();
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: Some("frame") });
        if scene.is_none()
            && let Some(timing) = &mut self.timing
        {
            timing.begin(&device, &mut encoder);
        }
        #[cfg(all(windows, feature = "streamline"))]
        let hudless = self.generation_target(scene.is_none());
        #[cfg(not(all(windows, feature = "streamline")))]
        let hudless: Option<wgpu::TextureView> = None;
        let scene_output = hudless.as_ref().unwrap_or(output);
        if self.post_effects().antialiasing != Antialiasing::Smaa {
            self.encode_scene(&mut encoder, scene, scene_output, frame, instances);
            self.encode_post(&mut encoder, scene, scene_output, size);
            #[cfg(all(windows, feature = "streamline"))]
            if let Some(hudless) = &hudless {
                self.composite_generation(&mut encoder, hudless, output);
            }
            if hudless.is_none() {
                self.encode_ui(&mut encoder, output, size);
            }
            if scene.is_none()
                && let Some(timing) = &mut self.timing
            {
                timing.finish(&mut encoder);
            }
            self.submit(encoder);
            if scene.is_none()
                && let Some(timing) = &mut self.timing
            {
                timing.submitted();
            }
            self.smaa = None;
            self.end_temporal();
            return;
        }
        let prepare_trace = blackbox_trace::scope("Renderer CPU: prepare SMAA");
        let format = scene_output.texture().format();
        let cache = self.pipeline_cache.as_ref();
        let mut aa = self.smaa.take().filter(|aa| aa.format == format).unwrap_or_else(|| SpatialAa {
            target: blackbox_smaa::SmaaTarget::new_cached(
                &device,
                &queue,
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
            aa.target.resize(&device, size.0, size.1);
            aa.size = size;
        }
        let target = aa.target.start_frame(&device, &queue, scene_output);
        drop(prepare_trace);
        self.encode_scene(&mut encoder, scene, &target, frame, instances);
        self.encode_post(&mut encoder, scene, &target, size);
        {
            // SMAA records into the frame encoder: scene, post, SMAA and UI go out in one submit.
            let _trace = blackbox_trace::scope("Renderer CPU: encode SMAA");
            target.encode_into(&mut encoder);
        }
        self.smaa = Some(aa);
        #[cfg(all(windows, feature = "streamline"))]
        if let Some(hudless) = &hudless {
            self.composite_generation(&mut encoder, hudless, output);
        }
        if hudless.is_none() {
            self.encode_ui(&mut encoder, output, size);
        }
        if scene.is_none()
            && let Some(timing) = &mut self.timing
        {
            timing.finish(&mut encoder);
        }
        self.submit(encoder);
        if scene.is_none()
            && let Some(timing) = &mut self.timing
        {
            timing.submitted();
        }
        self.end_temporal();
    }

    /// Keep command finalization separate from queue submission: both may enter the graphics driver. The captures
    /// queued for this frame go out in the same submission, ahead of it.
    fn submit(&mut self, encoder: wgpu::CommandEncoder) {
        let commands = {
            let _trace = blackbox_trace::scope("GPU API: finish command encoder");
            encoder.finish()
        };
        let _trace = blackbox_trace::scope("GPU API: queue submit");
        self.submit_after_captures(commands);
    }
}

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
        self.update_irradiance();
        self.begin_temporal(scene.is_none(), frame);
        let device = self.device.clone();
        let queue = self.queue.clone();
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: Some("frame") });
        if let Some(timing) = &mut self.timing {
            timing.begin(&device, &mut encoder);
        }
        if self.post_effects().antialiasing != Antialiasing::Smaa {
            self.encode_scene(&mut encoder, scene, output, frame, instances);
            self.encode_post(&mut encoder, scene, output, size);
            self.encode_ui(&mut encoder, output, size);
            if let Some(timing) = &mut self.timing {
                timing.finish(&mut encoder);
            }
            queue.submit([encoder.finish()]);
            if let Some(timing) = &mut self.timing {
                timing.submitted();
            }
            self.smaa = None;
            self.end_temporal();
            return;
        }
        let format = self.output.format();
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
        let target = aa.target.start_frame(&device, &queue, output);
        self.encode_scene(&mut encoder, scene, &target, frame, instances);
        self.encode_post(&mut encoder, scene, &target, size);
        queue.submit([encoder.finish()]);
        target.resolve();
        self.smaa = Some(aa);
        let mut encoder =
            device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: Some("UI after SMAA") });
        self.encode_ui(&mut encoder, output, size);
        if let Some(timing) = &mut self.timing {
            timing.finish(&mut encoder);
        }
        queue.submit([encoder.finish()]);
        if let Some(timing) = &mut self.timing {
            timing.submitted();
        }
        self.end_temporal();
    }
}

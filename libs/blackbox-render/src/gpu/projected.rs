//! Retained projected scene capture, independent of car environment cubes.
use super::{
    Renderer,
    reflections::{CaptureKind, CaptureView},
    targets::{FrameTargets, SceneTargets},
};
use blackbox_gfx::ProjectedReflectionScene;
use blackbox_gpu_passes::{Blend, Draw, Filter, Params, filter_source};

const FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8Unorm;
const MAX_EDGE: u32 = 2048;
const BLUR: &str = r#"
fn blur(uv:vec2<f32>,step:vec2<f32>)->vec4<f32> {
    return textureSampleLevel(src,samp,uv+step*1.5,0.0)/3.0
        +textureSampleLevel(src,samp,uv+step*3.0,0.0)/6.0
        +textureSampleLevel(src,samp,uv-step*1.5,0.0)/3.0
        +textureSampleLevel(src,samp,uv-step*3.0,0.0)/6.0;
}
@fragment fn fs_blur(in:VsOut)->@location(0) vec4<f32> {return blur(in.uv,params.a.xy);}
"#;

struct Blur {
    images: [wgpu::TextureView; 2],
    passes: [Filter; 2],
}
impl Blur {
    fn new(device: &wgpu::Device, size: [u32; 2]) -> Self {
        let images = std::array::from_fn(|_| {
            device
                .create_texture(&wgpu::TextureDescriptor {
                    label: Some("projected reflection blur"),
                    size: wgpu::Extent3d { width: size[0], height: size[1], depth_or_array_layers: 1 },
                    mip_level_count: 1,
                    sample_count: 1,
                    dimension: wgpu::TextureDimension::D2,
                    format: FORMAT,
                    usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::RENDER_ATTACHMENT,
                    view_formats: &[],
                })
                .create_view(&Default::default())
        });
        Self {
            images,
            passes: std::array::from_fn(|_| Filter::new(device, "projected reflection blur", &filter_source(BLUR))),
        }
    }
    fn encode(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        encoder: &mut wgpu::CommandEncoder,
        source: &wgpu::TextureView,
        size: [u32; 2],
    ) {
        for i in 0..2 {
            let step = match i {
                0 => [1.0 / size[0] as f32, 0.0, 0.0, 0.0],
                _ => [0.0, 1.0 / size[1] as f32, 0.0, 0.0],
            };
            self.passes[i].set_params(queue, &Params { a: step, b: [0.0; 4] });
            let src = match i {
                0 => source,
                _ => &self.images[0],
            };
            self.passes[i].draw(
                device,
                encoder,
                &Draw {
                    entry: "fs_blur",
                    src,
                    src2: src,
                    target: &self.images[i],
                    format: FORMAT,
                    blend: Blend::Replace,
                },
            );
        }
    }
}

pub(super) struct ProjectedCapture {
    size: [u32; 2],
    scene: SceneTargets,
    view: CaptureView,
    blur: Option<Blur>,
}
impl ProjectedCapture {
    fn new(device: &wgpu::Device, layout: &wgpu::BindGroupLayout, size: [u32; 2]) -> Self {
        let mut view = CaptureView::new(device, layout);
        view.kind = CaptureKind::Projected;
        Self {
            size,
            scene: SceneTargets::Offscreen(FrameTargets::new(device, FORMAT, (size[0], size[1]))),
            view,
            blur: None,
        }
    }
    pub(super) fn output(&self) -> &wgpu::TextureView {
        match &self.blur {
            Some(blur) => &blur.images[1],
            None => &self.scene.offscreen().expect("offscreen capture").color_view,
        }
    }
}
impl Renderer {
    pub(super) fn capture_projected(&mut self, scene: Option<ProjectedReflectionScene<'_>>) {
        let scene = scene.filter(|s| self.caps.projected_reflections && s.size[0] > 0 && s.size[1] > 0);
        let Some(scene) = scene else {
            if let Some(normal) = &mut self.normal {
                normal.set_projected(&self.device, None);
            }
            self.projected = None;
            return;
        };
        let limit = self.device.limits().max_texture_dimension_2d.min(MAX_EDGE);
        let size = scene.size.map(|edge| edge.min(limit));
        let mut capture = self
            .projected
            .take()
            .filter(|p| p.size == size)
            .unwrap_or_else(|| ProjectedCapture::new(&self.device, &self.shared.bindings.globals_layout, size));
        capture.blur = match scene.blur {
            true => Some(capture.blur.take().unwrap_or_else(|| Blur::new(&self.device, size))),
            false => None,
        };
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor { label: Some("projected reflection capture") });
        let output = &capture.scene.offscreen().expect("offscreen capture").color_view;
        self.encode_scene_mode(
            &mut encoder,
            Some(&capture.scene),
            output,
            &scene.frame,
            scene.instances,
            Some(&mut capture.view),
        );
        if let Some(blur) = &mut capture.blur {
            blur.encode(&self.device, &self.queue, &mut encoder, output, size);
        }
        self.queue.submit([encoder.finish()]);
        if let Some(normal) = &mut self.normal {
            normal.set_projected(&self.device, Some(capture.output()));
        }
        self.projected = Some(capture);
    }
}

#[cfg(test)]
#[path = "projected_tests.rs"]
mod tests;

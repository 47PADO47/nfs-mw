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
    pub(super) receiver_matrix: glam::Mat4,
    scene: SceneTargets,
    view: CaptureView,
    blur: Option<Blur>,
}
impl ProjectedCapture {
    fn new(device: &wgpu::Device, layout: &wgpu::BindGroupLayout, size: [u32; 2], kind: CaptureKind) -> Self {
        let mut view = CaptureView::new(device, layout);
        view.kind = kind;
        let format = match kind == CaptureKind::Planar {
            true => super::targets::HDR_FORMAT,
            false => FORMAT,
        };
        Self {
            size,
            receiver_matrix: glam::Mat4::IDENTITY,
            scene: SceneTargets::Offscreen(Box::new(FrameTargets::new(device, format, (size[0], size[1])))),
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
    /// Read the unblurred projected scene for offscreen diagnostics. A capture still queued for the next frame's
    /// submission is submitted first.
    pub fn read_projected_scene(&mut self) -> Option<Result<blackbox_gfx::RgbaImage, crate::RenderError>> {
        self.flush_captures();
        let capture = self.projected.as_ref()?;
        let target = capture.scene.offscreen()?;
        let size = (capture.size[0], capture.size[1]);
        let planar = capture.view.kind == CaptureKind::Planar;
        Some(self.read_raw_texture(target.color_view.texture(), size, if planar { 8 } else { 4 }).and_then(|bytes| {
            let rgba = match planar {
                true => bytes
                    .as_chunks::<2>()
                    .0
                    .iter()
                    .map(|b| (half::f16::from_le_bytes([b[0], b[1]]).to_f32().clamp(0.0, 1.0) * 255.0).round() as u8)
                    .collect(),
                false => bytes,
            };
            blackbox_gfx::RgbaImage::new(size.0, size.1, rgba)
        }))
    }

    pub(super) fn capture_projected(&mut self, scene: Option<ProjectedReflectionScene<'_>>) {
        self.clear_planar_receivers();
        self.capture_projected_mode(scene, CaptureKind::Projected, [0.0; 4]);
    }

    pub(super) fn capture_planar(&mut self, scene: Option<blackbox_gfx::PlanarReflectionScene<'_>>) {
        self.clear_planar_receivers();
        let Some(scene) = scene else {
            self.capture_projected_mode(None, CaptureKind::Planar, [0.0; 4]);
            return;
        };
        self.capture_projected_mode(
            Some(ProjectedReflectionScene {
                frame: scene.frame,
                size: scene.size,
                instances: scene.instances,
                blur: false,
            }),
            CaptureKind::Planar,
            scene.clip_plane,
        );
        let Some(image) = self.projected.as_ref().map(|capture| capture.output().clone()) else { return };
        self.bind_planar_receivers(scene.receivers, scene.frame.view_proj(), &image);
    }

    fn capture_projected_mode(
        &mut self,
        scene: Option<ProjectedReflectionScene<'_>>,
        kind: CaptureKind,
        clip_plane: [f32; 4],
    ) {
        // Drawing a capture, or dropping the one that is there, changes what the receivers show.
        if scene.is_some() || self.projected.is_some() {
            self.touch_scene();
        }
        let scene = scene.filter(|s| {
            self.caps.projected_reflections
                && s.size[0] > 0
                && s.size[1] > 0
                && (kind != CaptureKind::Planar || self.hdr_format == super::targets::HDR_FORMAT)
        });
        let Some(scene) = scene else {
            if let Some(normal) = &mut self.normal {
                normal.set_projected(&self.device, None);
            }
            self.projected = None;
            return;
        };
        let _trace = match kind {
            CaptureKind::Planar => blackbox_trace::scope("Renderer CPU: planar capture"),
            _ => blackbox_trace::scope("Renderer CPU: projected capture"),
        };
        // An earlier capture still queued wrote the capture view's globals this one is about to rewrite.
        self.flush_captures();
        // A queued projected capture opens the instance store's epoch, which the frame joins (see `queued.rs`). The
        // planar capture is submitted at once and draws in the epoch that is open, as before.
        let queued = kind == CaptureKind::Projected;
        if queued {
            self.begin_store_epoch();
        }
        let limit = self.device.limits().max_texture_dimension_2d.min(MAX_EDGE);
        let size = scene.size.map(|edge| edge.min(limit));
        let mut capture =
            self.projected.take().filter(|p| p.size == size && p.view.kind == kind).unwrap_or_else(|| {
                ProjectedCapture::new(&self.device, &self.shared.bindings.globals_layout, size, kind)
            });
        capture.view.clip_plane = clip_plane;
        capture.receiver_matrix = scene.frame.view_proj();
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
            false,
            Some(&mut capture.view),
        );
        if let Some(blur) = &mut capture.blur {
            blur.encode(&self.device, &self.queue, &mut encoder, output, size);
        }
        // The projected capture reads nothing the frame rewrites without first sending it (see `queued.rs`), so it
        // goes out ahead of the frame in the frame's own submission. The planar one reads the sun shadow data and
        // the scenery lighting, which every frame rewrites.
        match queued {
            true => self.queue_capture(encoder.finish()),
            false => {
                self.queue.submit([encoder.finish()]);
            }
        }
        if let Some(normal) = &mut self.normal {
            normal.set_projected(&self.device, Some(capture.output()));
        }
        self.projected = Some(capture);
    }
}

#[cfg(test)]
#[path = "projected_tests.rs"]
mod tests;

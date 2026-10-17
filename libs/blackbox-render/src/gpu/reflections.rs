//! Local scenery captures using the existing scene shader, without post effects or readback.
use blackbox_gfx::{ReflectionQuality, ReflectionScene, reflection::face_frame};
use blackbox_gpu_passes::{Blend, Draw, Filter, REFLECTION_STORE_WGSL, filter_source};

use super::{
    Renderer,
    instances::InstanceBuffer,
    resources::Globals,
    targets::{FrameTargets, SceneTargets},
};

/// Separate buffers keep all six camera/instance uploads valid in one queue submission.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum CaptureKind {
    Cube,
    Projected,
}

pub(super) struct CaptureView {
    pub kind: CaptureKind,
    pub globals: wgpu::Buffer,
    pub bind_group: wgpu::BindGroup,
    pub instances: InstanceBuffer,
}

impl CaptureView {
    pub(super) fn new(device: &wgpu::Device, layout: &wgpu::BindGroupLayout) -> Self {
        let globals = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("reflection globals"),
            size: std::mem::size_of::<Globals>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("reflection globals"),
            layout,
            entries: &[wgpu::BindGroupEntry { binding: 0, resource: globals.as_entire_binding() }],
        });
        Self { kind: CaptureKind::Cube, globals, bind_group, instances: InstanceBuffer::new(device) }
    }
}

pub(super) struct ReflectionCapture {
    size: u32,
    scene: SceneTargets,
    cube: wgpu::TextureView,
    face_array: wgpu::TextureView,
    faces: [wgpu::TextureView; 6],
    views: [CaptureView; 6],
    store: Filter,
}

impl ReflectionCapture {
    fn new(
        device: &wgpu::Device,
        layout: &wgpu::BindGroupLayout,
        size: u32,
        cache: Option<&wgpu::PipelineCache>,
    ) -> Self {
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("local environment cube"),
            size: wgpu::Extent3d { width: size, height: size, depth_or_array_layers: 6 },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        });
        let cube = texture.create_view(&wgpu::TextureViewDescriptor {
            dimension: Some(wgpu::TextureViewDimension::Cube),
            ..Default::default()
        });
        let face_array = texture.create_view(&wgpu::TextureViewDescriptor {
            dimension: Some(wgpu::TextureViewDimension::D2Array),
            ..Default::default()
        });
        let faces = std::array::from_fn(|face| {
            texture.create_view(&wgpu::TextureViewDescriptor {
                dimension: Some(wgpu::TextureViewDimension::D2),
                base_array_layer: face as u32,
                array_layer_count: Some(1),
                ..Default::default()
            })
        });
        Self {
            size,
            cube,
            face_array,
            faces,
            views: std::array::from_fn(|_| CaptureView::new(device, layout)),
            scene: SceneTargets::Offscreen(Box::new(FrameTargets::new(
                device,
                wgpu::TextureFormat::Rgba8Unorm,
                (size, size),
            ))),
            store: Filter::new(device, "environment cube store", &filter_source(REFLECTION_STORE_WGSL))
                .with_pipeline_cache(cache.cloned()),
        }
    }
}

impl Renderer {
    pub(super) fn capture_reflections(&mut self, scene: Option<ReflectionScene<'_>>) {
        let Some(scene) = scene.filter(|s| s.quality != ReflectionQuality::Off) else {
            if self.reflections.take().is_some()
                && let Some(glossy) = self.glossy.as_mut()
            {
                glossy.restore_environment();
            }
            return;
        };
        let (_, output_height) = self.surface_size();
        let size = self.graphics.reflection_resolution.cube_face(
            scene.quality.size(),
            output_height,
            blackbox_gfx::reflection::PLANAR_MAX,
        );
        let mut capture = self.reflections.take().filter(|p| p.size == size).unwrap_or_else(|| {
            let layout = &self.shared.bindings.globals_layout;
            ReflectionCapture::new(&self.device, layout, size, self.pipeline_cache.as_ref())
        });
        let mut encoder =
            self.device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: Some("environment capture") });
        for face in 0..6 {
            let frame = face_frame(scene.origin, face, &scene.frame);
            let output = &capture.scene.offscreen().expect("capture is offscreen").color_view;
            self.encode_scene_mode(
                &mut encoder,
                Some(&capture.scene),
                output,
                &frame,
                scene.faces[face],
                false,
                Some(&mut capture.views[face]),
            );
            capture.store.draw(
                &self.device,
                &mut encoder,
                &Draw {
                    entry: "fs_store",
                    src: output,
                    src2: output,
                    target: &capture.faces[face],
                    format: wgpu::TextureFormat::Rgba8Unorm,
                    blend: Blend::Replace,
                },
            );
        }
        // Dedicated face buffers avoid overwriting queued draws, without six separate submissions.
        self.queue.submit([encoder.finish()]);
        if let Some(glossy) = self.glossy.as_mut() {
            glossy.use_environment(&self.device, &capture.cube, &capture.face_array);
        }
        self.reflections = Some(capture);
    }
}

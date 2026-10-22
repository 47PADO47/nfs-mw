//! Local scenery captures using the existing scene shader, without post effects or readback.
use blackbox_gfx::{ReflectionQuality, ReflectionScene, reflection::face_frame};
#[cfg(test)]
#[path = "reflection_lighting_tests.rs"]
mod lighting_tests;

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
    LitCube,
    Projected,
}

pub(super) struct CaptureView {
    pub kind: CaptureKind,
    pub globals: wgpu::Buffer,
    pub bind_group: wgpu::BindGroup,
    pub instances: InstanceBuffer,
    draws: super::draw_list::DrawList,
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
        Self {
            kind: CaptureKind::Cube,
            globals,
            bind_group,
            instances: InstanceBuffer::new(device),
            draws: Default::default(),
        }
    }
}

pub(super) struct ReflectionCapture {
    size: u32,
    scene: SceneTargets,
    cube: wgpu::TextureView,
    face_array: wgpu::TextureView,
    faces: [wgpu::TextureView; 6],
    depth: wgpu::TextureView,
    depth_faces: [wgpu::TextureView; 6],
    views: [CaptureView; 6],
    store: super::reflection_store::Store,
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
            usage: wgpu::TextureUsages::TEXTURE_BINDING
                | wgpu::TextureUsages::RENDER_ATTACHMENT
                | wgpu::TextureUsages::COPY_SRC,
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
        let depth_texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("environment face depths"),
            size: wgpu::Extent3d { width: size, height: size, depth_or_array_layers: 6 },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: super::resources::DEPTH_FORMAT,
            usage: wgpu::TextureUsages::TEXTURE_BINDING
                | wgpu::TextureUsages::RENDER_ATTACHMENT
                | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        });
        let depth = depth_texture.create_view(&wgpu::TextureViewDescriptor {
            dimension: Some(wgpu::TextureViewDimension::D2Array),
            ..Default::default()
        });
        let depth_faces = std::array::from_fn(|face| {
            depth_texture.create_view(&wgpu::TextureViewDescriptor {
                dimension: Some(wgpu::TextureViewDimension::D2),
                base_array_layer: face as u32,
                array_layer_count: Some(1),
                ..Default::default()
            })
        });
        let scene =
            SceneTargets::Offscreen(Box::new(FrameTargets::new(device, wgpu::TextureFormat::Rgba8Unorm, (size, size))));
        let store =
            super::reflection_store::Store::new(device, &scene.offscreen().expect("capture").color_view, &depth, cache);
        Self {
            size,
            depth,
            depth_faces,
            cube,
            face_array,
            faces,
            views: std::array::from_fn(|_| CaptureView::new(device, layout)),
            scene,
            store,
        }
    }
}

impl Renderer {
    pub(super) fn capture_reflections(
        &mut self,
        scene: Option<ReflectionScene<'_>>,
        encoder: Option<&mut wgpu::CommandEncoder>,
    ) {
        let Some(scene) = scene.filter(|s| s.quality != ReflectionQuality::Off) else {
            if self.reflections.take().is_some()
                && let Some(glossy) = self.glossy.as_mut()
            {
                glossy.restore_environment(&self.queue);
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
        let encoder = encoder.expect("active capture requires a command encoder");
        capture.store.configure(&self.queue, scene.fade_distance, scene.frame.clear_color);
        for face in 0..6 {
            let frame = face_frame(scene.origin, face, &scene.frame);
            capture.views[face].kind = match scene.sun_shadows {
                true => CaptureKind::LitCube,
                false => CaptureKind::Cube,
            };
            // Retain actual depth for all faces. Colour storage flips vertically; depth
            // addressing applies that flip explicitly in the local-probe shader.
            if let SceneTargets::Offscreen(targets) = &mut capture.scene {
                targets.depth_view = capture.depth_faces[face].clone();
            }
            let output = &capture.scene.offscreen().expect("capture is offscreen").color_view;
            std::mem::swap(&mut self.draw_list, &mut capture.views[face].draws);
            self.encode_scene_mode(
                encoder,
                Some(&capture.scene),
                output,
                &frame,
                scene.faces[face],
                false,
                Some(&mut capture.views[face]),
            );
            std::mem::swap(&mut self.draw_list, &mut capture.views[face].draws);
            capture.store.draw(encoder, &capture.faces[face], face);
        }
        // Dedicated face buffers avoid overwriting queued draws, without six separate submissions.
        if let Some(glossy) = self.glossy.as_mut() {
            glossy.set_probe(
                &self.device,
                &self.queue,
                &capture.depth,
                scene.origin,
                self.experiments.local_reflection_correction,
            );
            glossy.use_environment(&self.device, &capture.cube, &capture.face_array);
        }
        self.reflections = Some(capture);
    }
}

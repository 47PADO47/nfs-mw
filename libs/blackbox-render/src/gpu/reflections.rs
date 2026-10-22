//! Local scenery captures using the existing scene shader, without post effects or readback.
//!
//! The scene is drawn straight into the cube's face views. The cube addresses its faces with the opposite handedness
//! to the right-handed face cameras, so each camera's image is flipped vertically in the view (the scene pipelines
//! draw both sides, so the mirrored winding changes nothing). A call may refresh only some faces; the others keep
//! their last image.
use blackbox_gfx::{
    ReflectionQuality, ReflectionScene,
    reflection::{ALL_FACES, face_frame},
};
use glam::{Mat4, Vec3};

use super::{
    Renderer,
    instances::InstanceBuffer,
    resources::Globals,
    targets::{ScenePlan, SceneTargets},
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
    /// The faces drawn since the diffuse light was last projected from the cube.
    covered: u8,
}

impl ReflectionCapture {
    fn new(device: &wgpu::Device, layout: &wgpu::BindGroupLayout, size: u32) -> Self {
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
            // Only a depth buffer: the colour goes into each face view.
            scene: SceneTargets::new(
                device,
                &ScenePlan { direct: true, format: wgpu::TextureFormat::Rgba8Unorm, size: (size, size), motion: false },
            ),
            covered: 0,
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
        let (mut capture, created) = match self.reflections.take().filter(|p| p.size == size) {
            Some(capture) => (capture, false),
            None => (ReflectionCapture::new(&self.device, &self.shared.bindings.globals_layout, size), true),
        };
        // A fresh cube holds nothing yet, so it is drawn whole whatever the caller asked for.
        let faces = match created {
            true => ALL_FACES,
            false => scene.refresh & ALL_FACES,
        };
        // The cube texture outlives its captures, so the glossy group is built once, not on every refresh.
        if let Some(glossy) = self.glossy.as_mut()
            && (created || !glossy.capture_bound())
        {
            glossy.use_environment(&self.device, &capture.cube, &capture.face_array);
        }
        if faces != 0 {
            self.draw_cube_faces(&mut capture, &scene, faces);
        }
        self.reflections = Some(capture);
    }

    /// Draws `faces` of the cube in one submission. Once every face has been drawn since the last time, the diffuse
    /// light is projected from the cube, in the same submission, instead of once per capture.
    fn draw_cube_faces(&mut self, capture: &mut ReflectionCapture, scene: &ReflectionScene<'_>, faces: u8) {
        let mut encoder =
            self.device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: Some("environment capture") });
        let flip = Mat4::from_scale(Vec3::new(1.0, -1.0, 1.0));
        for face in (0..6usize).filter(|&face| (faces >> face) & 1 != 0) {
            let mut frame = face_frame(scene.origin, face, &scene.frame);
            frame.view = flip * frame.view;
            self.encode_scene_mode(
                &mut encoder,
                Some(&capture.scene),
                &capture.faces[face],
                &frame,
                scene.faces[face],
                false,
                Some(&mut capture.views[face]),
            );
        }
        capture.covered |= faces;
        if capture.covered == ALL_FACES {
            capture.covered = 0;
            let projects = self.experiments.car_skylight && self.supports_irradiance();
            if let Some(glossy) = self.glossy.as_mut() {
                glossy.environment_refreshed();
            }
            if projects && let Some(glossy) = self.glossy.as_mut() {
                glossy.encode_irradiance(&self.device, &mut encoder);
            }
        }
        // Dedicated face buffers avoid overwriting queued draws, without six separate submissions.
        self.queue.submit([encoder.finish()]);
    }
}

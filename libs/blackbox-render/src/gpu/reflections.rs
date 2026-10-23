//! Local scenery captures using the existing scene shader, without post effects or readback.
//!
//! Each face is drawn into a colour and depth target of its own size, then a store pass copies it into the cube's face
//! (flipped vertically, since the cube addresses its faces with the opposite handedness to the right-handed face
//! cameras) and fades a finite capture towards the fog colour at its edge. The depth of every face is kept for the
//! local reflection correction. A call may refresh only some faces; the others keep their last image and depth.
use blackbox_gfx::{
    ReflectionQuality, ReflectionScene,
    reflection::{ALL_FACES, face_frame},
};
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
    Planar,
}

pub(super) struct CaptureView {
    pub kind: CaptureKind,
    pub clip_plane: [f32; 4],
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
        Self {
            kind: CaptureKind::Cube,
            clip_plane: [0.0; 4],
            globals,
            bind_group,
            instances: InstanceBuffer::new(device),
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
    /// The faces drawn since the diffuse light was last projected from the cube.
    covered: u8,
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
            covered: 0,
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
        let (mut capture, created) = match self.reflections.take().filter(|p| p.size == size) {
            Some(capture) => (capture, false),
            None => {
                let layout = &self.shared.bindings.globals_layout;
                (ReflectionCapture::new(&self.device, layout, size, self.pipeline_cache.as_ref()), true)
            }
        };
        // A fresh cube holds nothing yet, so it is drawn whole whatever the caller asked for.
        let faces = match created {
            true => ALL_FACES,
            false => scene.refresh & ALL_FACES,
        };
        // The cube texture outlives its captures, so the glossy group is built once, not on every refresh.
        if let Some(glossy) = self.glossy.as_mut() {
            if created || !glossy.capture_bound() {
                glossy.use_environment(&self.device, &capture.cube, &capture.face_array);
            }
            glossy.set_probe(
                &self.device,
                &self.queue,
                &capture.depth,
                scene.origin,
                self.experiments.local_reflection_correction,
            );
        }
        if faces != 0 {
            let encoder = encoder.expect("active capture requires a command encoder");
            self.draw_cube_faces(&mut capture, &scene, faces, encoder);
        }
        self.reflections = Some(capture);
    }

    /// Records `faces` of the cube into `encoder`. Once every face has been drawn since the last time, the diffuse
    /// and specular light are marked to be projected from the cube again, instead of once per capture.
    fn draw_cube_faces(
        &mut self,
        capture: &mut ReflectionCapture,
        scene: &ReflectionScene<'_>,
        faces: u8,
        encoder: &mut wgpu::CommandEncoder,
    ) {
        capture.store.configure(&self.queue, scene.fade_distance, scene.frame.clear_color);
        let kind = match scene.sun_shadows {
            true => CaptureKind::LitCube,
            false => CaptureKind::Cube,
        };
        for face in (0..6usize).filter(|&face| (faces >> face) & 1 != 0) {
            let frame = face_frame(scene.origin, face, &scene.frame);
            capture.views[face].kind = kind;
            // Retain the actual depth of the faces drawn. Colour storage flips vertically; depth addressing applies
            // that flip explicitly in the local-probe shader.
            if let SceneTargets::Offscreen(targets) = &mut capture.scene {
                targets.depth_view = capture.depth_faces[face].clone();
            }
            let output = capture.scene.offscreen().expect("capture is offscreen").color_view.clone();
            self.encode_scene_mode(
                encoder,
                Some(&capture.scene),
                &output,
                &frame,
                scene.faces[face],
                false,
                Some(&mut capture.views[face]),
            );
            capture.store.draw(encoder, &capture.faces[face], face);
        }
        capture.covered |= faces;
        if capture.covered != ALL_FACES {
            return;
        }
        capture.covered = 0;
        if let Some(glossy) = self.glossy.as_mut() {
            glossy.environment_refreshed();
        }
    }
}

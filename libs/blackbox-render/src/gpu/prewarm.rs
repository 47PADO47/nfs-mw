//! Building the pipelines ahead of the first frame that needs them.
//!
//! Pipelines, shader modules and the glossy resources are created lazily, inside the frame that first draws with
//! them, so the first frames of a scene (and the first frame after a settings change) stall for hundreds of
//! milliseconds. [`Renderer::prewarm`] runs the frame's own code once over an empty scene, into a throw-away image
//! of the output's format and size. Every lazy cache the frame fills (scene pipelines per colour format, the sky,
//! normal and window families, the effect layer, the shadow, depth-prepass, AO, contact and screen-reflection
//! passes, the post chain, the upscaler, SMAA, the UI) is then filled exactly as a real frame would fill it, with
//! no second code path to keep in step. The environment cube capture is warmed the same way.
//!
//! What it cannot know: pipelines for outputs it is never asked to draw (the rear-view mirror's RGBA8 picture
//! with its own post chain, screenshots, the projected-reflection blur) and anything that depends on resources
//! created after the call; call it again then, it only does the missing work.

use std::time::Instant;

use blackbox_gfx::{
    Fog, FrameParams, GraphicsSettings, Instance, Projection, ReflectionQuality, ReflectionScene, RenderExperiments,
    reflection::ALL_FACES,
};
use glam::{Mat4, Vec3};

use super::{Renderer, output::Output};

/// The format the reflection and projected captures draw into.
const CAPTURE_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8Unorm;

/// What a prewarm was done for. A second call with the same state has nothing left to build.
#[derive(PartialEq)]
pub(super) struct Warmed {
    graphics: GraphicsSettings,
    experiments: RenderExperiments,
    output_format: wgpu::TextureFormat,
    scene_format: wgpu::TextureFormat,
    /// Whether the glossy state, the sky, the normal and the window families exist, and whether a single-sided
    /// draw does (the culling variants): each adds pipelines.
    families: [bool; 5],
}

impl Renderer {
    /// Build ahead of time what the next frames would build on first use, for the current effective settings,
    /// experiments and the resources created so far. Idempotent: with nothing changed since the last call it
    /// returns at once. It also creates the glossy state (see [`Self::glossy_in_use`]), which a scene that shades
    /// cars glossy needs anyway.
    pub fn prewarm(&mut self) {
        if self.prewarmed.as_ref() == Some(&self.warm_key()) {
            return;
        }
        let _trace = blackbox_trace::scope("Renderer: prewarm pipelines");
        let started = Instant::now();
        self.ensure_glossy();
        self.ui.use_format(&self.device, self.output.format());
        let frame = self.warm_params();
        self.warm_frame(&frame);
        self.warm_captures(&frame);
        // Captures leave their own format selected; the next scene pass selects its format again, but keep the
        // state a frame expects anyway.
        let scene_format = self.targets.color_format();
        self.pipelines.use_format(&self.device, scene_format, self.glossy.is_some());
        self.effects.use_format(&self.device, scene_format);
        self.prewarmed = Some(self.warm_key());
        log::info!("renderer: prewarmed pipelines in {:.0} ms", started.elapsed().as_secs_f64() * 1000.0);
    }

    /// [`Self::prewarm`] for a renderer that presents to a window. Headless renderers (tests, tools) are driven
    /// explicitly, so a settings change does not run a frame behind their back.
    pub(super) fn prewarm_windowed(&mut self) {
        let Output::Window { .. } = &self.output else { return };
        self.prewarm();
    }

    pub(super) fn warm_key(&self) -> Warmed {
        Warmed {
            graphics: self.graphics,
            experiments: self.experiments,
            output_format: self.output.format(),
            scene_format: self.targets.color_format(),
            families: [
                self.glossy.is_some(),
                self.sky.is_some(),
                self.normal.is_some(),
                self.window.is_some(),
                self.pipelines.culled(),
            ],
        }
    }

    /// A valid perspective view of nothing: it takes every screen-space path (AO, contact, reflections) a
    /// perspective frame takes, and `camera_cut` keeps any history from the frame out of the next real one.
    fn warm_params(&self) -> FrameParams {
        let (width, height) = self.surface_size();
        FrameParams {
            view: Mat4::IDENTITY,
            projection: Projection::PerspectiveInfiniteReverse {
                fov_y: 1.0,
                aspect: width as f32 / height.max(1) as f32,
                near: 0.1,
            },
            camera_position: Vec3::ZERO,
            light_dir: Vec3::new(0.3, 0.4, -0.8).normalize(),
            clear_color: [0.0; 3],
            fog: Some(Fog { start: 50.0, end: 500.0 }),
            scattering: None,
            surface_light: None,
            camera_cut: true,
            shadow_tuning: None,
        }
    }

    /// One whole frame of an empty scene into a throw-away image of the output's format and size, without
    /// presenting it and without the GPU timing or the temporal history noticing.
    fn warm_frame(&mut self, frame: &FrameParams) {
        // Streamline presents through its own swapchain wrapper, which an image of ours cannot stand in for.
        #[cfg(all(windows, feature = "streamline"))]
        if self.dlss.is_some() {
            return;
        }
        let size = self.surface_size();
        let target = self.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("prewarm output"),
            size: wgpu::Extent3d { width: size.0.max(1), height: size.1.max(1), depth_or_array_layers: 1 },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: self.output.format(),
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        });
        let view = target.create_view(&wgpu::TextureViewDescriptor::default());
        let timing = self.timing.take();
        self.submit_frame(None, &view, size, frame, &[]);
        self.timing = timing;
        self.temporal.stop();
    }

    /// The environment cube capture (six scene passes) when reflections are on, else the
    /// scene pipelines the projected capture draws with when the world uses authored reflective materials.
    fn warm_captures(&mut self, frame: &FrameParams) {
        let quality = self.graphics.reflections;
        if quality != ReflectionQuality::Off && self.reflections.is_none() {
            let nothing: &[Instance] = &[];
            self.capture_reflections(Some(ReflectionScene {
                origin: Vec3::ZERO,
                quality,
                frame: *frame,
                faces: [nothing; 6],
                refresh: ALL_FACES,
            }));
            // The capture is kept, so the first real one reuses it, but it is empty: glossy surfaces keep the
            // static environment until a real capture replaces it.
            if let Some(glossy) = self.glossy.as_mut() {
                glossy.restore_environment();
            }
            return;
        }
        let projected =
            self.experiments.original_world_materials && self.caps.projected_reflections && self.normal.is_some();
        if projected {
            self.warm_format(CAPTURE_FORMAT, frame);
        }
    }

    /// What a scene pass into `format` builds on its first use, without drawing.
    fn warm_format(&mut self, format: wgpu::TextureFormat, frame: &FrameParams) {
        let cache = self.pipeline_cache.as_ref();
        let culled = self.pipelines.culled();
        self.pipelines.use_format(&self.device, format, self.glossy.is_some());
        self.effects.use_format(&self.device, format);
        if let Some(sky) = &mut self.sky {
            sky.prepare(&self.device, &self.queue, format, cache);
        }
        if let Some(normal) = &mut self.normal {
            normal.prepare(&self.device, &self.queue, (format, culled), frame, cache);
        }
        if let Some(window) = &mut self.window {
            window.prepare(&self.device, (format, culled), cache);
        }
    }
}

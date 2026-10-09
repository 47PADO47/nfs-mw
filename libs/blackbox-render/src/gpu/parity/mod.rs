//! Regression guard for the native renderer: every testkit scene, rendered headless under several graphics
//! settings, must keep looking the same.
//!
//! - `digest_tests`: each (setting combination, scene) pair against a digest recorded in [`digests`];
//! - `structure_tests`: facts about the pictures that hold on any adapter (depth order, fog, clipping...);
//! - `api_tests`: the native capabilities, `resolve` and `apply_graphics` on a real renderer.
//!
//! All of them are `#[ignore = "needs a GPU"]` and skip, not fail, when no adapter is available. The digests
//! depend on the GPU and driver: they were recorded on Intel Xe graphics (Vulkan, Mesa), and the digest tests
//! skip on any other adapter. See `docs/testing.md` for running and regenerating them.

mod api_tests;
mod digest_tests;
mod digests;
mod structure_tests;

use blackbox_gfx::{
    Antialiasing, FrameParams, GraphicsSettings, PostSettings, RenderBackend, RenderError, RgbaImage, Tonemap, Upscaler,
};
use blackbox_gfx_testkit::SceneId;

use crate::Renderer;

/// The size every scene is rendered at: 8x8 pixels per digest cell.
pub(super) const SIZE: [u32; 2] = [256, 144];

/// A named set of graphics settings and what path the scene is expected to take.
pub(super) struct Combo {
    pub name: &'static str,
    pub settings: GraphicsSettings,
    /// Whether the scene goes straight into the output (no offscreen image, no post chain).
    pub direct: bool,
}

/// The setting combinations the digests cover.
pub(super) fn combos() -> Vec<Combo> {
    let base = GraphicsSettings::default();
    let post = |post: PostSettings| GraphicsSettings { post, ..base };
    vec![
        Combo { name: "direct", settings: base, direct: true },
        Combo {
            name: "offscreen",
            settings: GraphicsSettings { upscaler: Upscaler::Bilinear, render_scale: 0.67, ..base },
            direct: false,
        },
        Combo {
            name: "fxaa",
            settings: post(PostSettings { antialiasing: Antialiasing::Fxaa, ..PostSettings::default() }),
            direct: false,
        },
        Combo {
            name: "bloom_aces",
            settings: post(PostSettings {
                tonemap: Tonemap::Aces,
                bloom_intensity: 0.6,
                bloom_threshold: 0.6,
                ..PostSettings::default()
            }),
            direct: false,
        },
        Combo {
            name: "fsr1_67",
            settings: GraphicsSettings { upscaler: Upscaler::Fsr1, render_scale: 0.67, ..base },
            direct: false,
        },
    ]
}

/// A capture of `scene` through the trait, and the frame parameters it was drawn with.
pub(super) fn capture(renderer: &mut Renderer, scene: SceneId) -> Result<(RgbaImage, FrameParams), RenderError> {
    let backend: &mut dyn RenderBackend = renderer;
    let built = scene.build(backend, SIZE);
    built.apply_layers(backend);
    let id = backend.request_capture(SIZE, &built.frame, &built.instances);
    let image = id.and_then(|id| backend.poll_capture(id).expect("the native backend completes captures at once"));
    let frame = built.frame;
    built.release(backend);
    image.map(|image| (image, frame))
}

/// Project a world point to pixel coordinates of a `SIZE` image drawn with `frame`.
pub(super) fn project(frame: &FrameParams, point: glam::Vec3) -> (u32, u32) {
    let clip = frame.view_proj() * point.extend(1.0);
    let ndc = clip / clip.w;
    let x = (ndc.x * 0.5 + 0.5) * SIZE[0] as f32;
    let y = (0.5 - ndc.y * 0.5) * SIZE[1] as f32;
    (x.clamp(0.0, SIZE[0] as f32 - 1.0) as u32, y.clamp(0.0, SIZE[1] as f32 - 1.0) as u32)
}

/// The RGB of the pixel at `at`.
pub(super) fn rgb(image: &RgbaImage, at: (u32, u32)) -> [u8; 3] {
    let p = image.pixel(at.0, at.1).expect("a pixel inside the image");
    [p[0], p[1], p[2]]
}

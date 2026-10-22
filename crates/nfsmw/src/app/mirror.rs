//! The rear-view mirror's picture. Each frame the scene's rear view is captured offscreen and shown as the mirror's UI
//! texture; the HUD presenter draws it under the mirror's bezel (`ui::present::blackbox`).
//!
//! Where the renderer can (`RenderBackend::view_capture_to_ui`) the capture never leaves the GPU: it is tinted and
//! clipped to the bezel's opening in a compute pass and lands in the UI texture in the same frame. Otherwise the
//! capture is read back, tinted and clipped here, and uploaded a frame or more later.

use blackbox_gfx::reflection::distance::cull_instances;
use blackbox_gfx::reflection::{MirrorTint, PLANAR_MAX};
use blackbox_gfx::{
    CaptureId, FrameParams, Instance, RenderBackend, RenderError, UiLayer, UiTextureId, UiTexturePatch, ViewCaptureUi,
};

use crate::ui::present::blackbox::MIRROR_TEXTURE;
use crate::viewer::Scene;

/// The bezel's proportions in pixels (about 256 by 87 HUD units): the base of the mirror picture's size. The
/// setting scales it; the width always keeps this aspect.
const BASE: [u32; 2] = [384, 131];
/// Inset of the picture from the captured image's edge, in pixels, so the clip does not cut the bezel's edge.
const INSET: f32 = 2.0;
/// The bezel's coverage as a UI texture for the GPU path: the same size as the picture, the coverage in its alpha.
const MIRROR_MASK: UiTextureId = UiTextureId(0x4D49_5252_4D53);

#[derive(Default)]
pub struct MirrorOverlay {
    pending: Option<CaptureId>,
    /// The texture has a picture (or the blank one) uploaded.
    uploaded: bool,
    /// The HUD and UI layer as the presenter last made it, drawn again after a capture.
    layer: Option<UiLayer>,
    /// The bezel's coverage for a picture of `coverage_size` pixels. Rebuilt only when the picture's size changes.
    coverage: Vec<u8>,
    coverage_size: [u32; 2],
    /// The size of the picture the mask texture of the GPU path holds, when it holds one.
    mask_size: Option<[u32; 2]>,
    /// The GPU path failed once; the readback path serves from then on.
    gpu_failed: bool,
}

impl MirrorOverlay {
    /// The HUD and UI layer as the presenter last made it, without the streaks.
    pub fn base_layer(&self) -> Option<&UiLayer> {
        self.layer.as_ref()
    }

    /// Sets the frame's UI layer, and keeps it so the rear view can be captured without it. An unchanged layer is
    /// not sent again: `StreakOverlay::show` runs after this in the frame and sends the layer the renderer draws.
    pub fn set_layer(&mut self, renderer: &mut dyn RenderBackend, layer: UiLayer) {
        if self.layer.as_ref() == Some(&layer) {
            return;
        }
        renderer.set_ui_layer_ref(self.layer.insert(layer));
    }

    /// Collect a finished capture, then ask for the next one when the scene has a rear view.
    pub fn update(&mut self, renderer: &mut dyn RenderBackend, scene: &mut dyn Scene) {
        let _span = bevy_log::info_span!("render::mirror").entered();
        let _trace = blackbox_trace::scope("Render bridge: rear-view mirror");
        let tint = renderer.graphics().mirror_tint;
        let distance = renderer.graphics().reflection_distance;
        if !self.uploaded {
            // Until the first capture the bezel's picture is clear, so the frame shows the glass only.
            renderer.update_ui_texture(&UiTexturePatch {
                id: MIRROR_TEXTURE,
                offset: None,
                size: [1, 1],
                rgba: &[0; 4],
            });
            self.uploaded = true;
        }
        let on_gpu = !self.gpu_failed && renderer.view_capture_to_ui();
        if !self.collect(renderer, tint) {
            return;
        }
        let resolution = renderer.graphics().reflection_resolution;
        let size = resolution.mirror_size(BASE, renderer.surface_size(), PLANAR_MAX);
        let aspect = size[0] as f32 / size[1] as f32;
        let Some((params, instances)) = scene.rear_view(aspect) else { return };
        // The original's mirror far plane (192 m) is applied to the object origins, as the setting says.
        let instances = cull_instances(instances, params.camera_position, distance.mirror_far());
        // A backend whose view captures draw the UI needs it hidden, since the mirror shows only the road.
        let hides_ui = !renderer.capabilities().view_capture_without_ui;
        if hides_ui {
            renderer.set_ui_layer(UiLayer::default());
        }
        let started = match on_gpu {
            true => self.capture_to_ui(renderer, size, &params, &instances, tint),
            false => renderer.request_view_capture(size, &params, &instances).map(|id| self.pending = Some(id)),
        };
        if let Some(layer) = self.layer.as_ref().filter(|_| hides_ui) {
            renderer.set_ui_layer_ref(layer);
        }
        let Err(e) = started else { return };
        log::warn!("rear-view mirror capture could not start: {e}");
        self.gpu_failed |= on_gpu;
    }

    /// Takes the finished readback capture, if one is in flight, and uploads it as the mirror's picture. Returns
    /// `false` while the capture is still being read back, so no new one starts.
    fn collect(&mut self, renderer: &mut dyn RenderBackend, tint: MirrorTint) -> bool {
        let Some(id) = self.pending else { return true };
        match renderer.poll_capture(id) {
            None => return false,
            Some(Ok(mut image)) => {
                let coverage = self.coverage_for(image.width, image.height);
                apply_tint_and_clip(&mut image.rgba, coverage, tint);
                renderer.update_ui_texture(&UiTexturePatch {
                    id: MIRROR_TEXTURE,
                    offset: None,
                    size: [image.width, image.height],
                    rgba: &image.rgba,
                });
            }
            Some(Err(e)) => log::warn!("rear-view mirror capture failed: {e}"),
        }
        self.pending = None;
        true
    }

    /// Draws the rear view into the mirror's UI texture on the GPU, tinted and clipped to the bezel's opening. The
    /// mask texture holding the bezel's coverage is uploaded here, again only when the picture's size changes.
    fn capture_to_ui(
        &mut self,
        renderer: &mut dyn RenderBackend,
        size: [u32; 2],
        params: &FrameParams,
        instances: &[Instance],
        tint: MirrorTint,
    ) -> Result<(), RenderError> {
        if self.mask_size != Some(size) {
            let rgba = mask_rgba(self.coverage_for(size[0], size[1]));
            renderer.update_ui_texture(&UiTexturePatch { id: MIRROR_MASK, offset: None, size, rgba: &rgba });
            self.mask_size = Some(size);
        }
        let target = ViewCaptureUi { texture: MIRROR_TEXTURE, mask: MIRROR_MASK, tint: tint.0 };
        renderer.request_view_capture_ui(size, params, instances, &target)
    }

    /// The bezel's coverage for a picture of this size, rebuilt only when the size differs from the last one.
    fn coverage_for(&mut self, width: u32, height: u32) -> &[u8] {
        let size = [width, height];
        if self.coverage_size != size {
            self.coverage = bezel_coverage(width, height);
            self.coverage_size = size;
        }
        &self.coverage
    }
}

/// The coverage as the mask texture's pixels: the coverage in the alpha, the colour channels unused.
fn mask_rgba(coverage: &[u8]) -> Vec<u8> {
    coverage.iter().flat_map(|&cover| [255, 255, 255, cover]).collect()
}

/// The bezel's coverage per pixel, 0 to 255: 255 inside the rounded rectangle that is the bezel's opening, 0 outside,
/// and a half-pixel anti-aliased edge between.
fn bezel_coverage(width: u32, height: u32) -> Vec<u8> {
    let (w, h) = (width as f32, height as f32);
    let radius = h * 0.35;
    let half = [w * 0.5 - INSET - radius, h * 0.5 - INSET - radius];
    let mut coverage = Vec::with_capacity(width as usize * height as usize);
    for y in 0..height {
        for x in 0..width {
            let p = [x as f32 + 0.5 - w * 0.5, y as f32 + 0.5 - h * 0.5];
            // Signed distance to the rounded rectangle: negative inside.
            let q = [p[0].abs() - half[0], p[1].abs() - half[1]];
            let outside = (q[0].max(0.0).powi(2) + q[1].max(0.0).powi(2)).sqrt() + q[0].max(q[1]).min(0.0) - radius;
            let fraction = (0.5 - outside).clamp(0.0, 1.0);
            coverage.push((fraction * 255.0).round() as u8);
        }
    }
    coverage
}

/// Tints the colour by `tint` and clips it by `coverage` in one pass. Each channel is multiplied by the tint and
/// then by the coverage, so the colour is premultiplied and the edge blends the way the rest of the UI does. The
/// alpha is the coverage. A [`MirrorTint::NONE`] tint multiplies by 255, which leaves each channel as it was.
///
/// The GPU path (`ui_picture.wgsl` in `blackbox-gpu-passes`) does the same integer arithmetic on each pixel.
fn apply_tint_and_clip(rgba: &mut [u8], coverage: &[u8], tint: MirrorTint) {
    for (pixel, &cover) in rgba.as_chunks_mut::<4>().0.iter_mut().zip(coverage) {
        let cov = u16::from(cover);
        for (channel, factor) in pixel[..3].iter_mut().zip(tint.0) {
            let tinted = (u16::from(*channel) * u16::from(factor) + 127) / 255;
            *channel = ((tinted * cov + 127) / 255) as u8;
        }
        pixel[3] = cover;
    }
}

#[cfg(test)]
mod tests {
    use blackbox_gfx::Capabilities;
    use blackbox_gfx::reflection::tint_rgba;

    use super::*;
    use crate::app::test_backend::NullBackend;

    /// The clip as it was before the coverage was precomputed: float math per pixel, kept here as the reference.
    fn clip_reference(rgba: &mut [u8], width: u32, height: u32) {
        let (w, h) = (width as f32, height as f32);
        let radius = h * 0.35;
        let half = [w * 0.5 - INSET - radius, h * 0.5 - INSET - radius];
        for y in 0..height {
            for x in 0..width {
                let p = [x as f32 + 0.5 - w * 0.5, y as f32 + 0.5 - h * 0.5];
                let q = [p[0].abs() - half[0], p[1].abs() - half[1]];
                let outside = (q[0].max(0.0).powi(2) + q[1].max(0.0).powi(2)).sqrt() + q[0].max(q[1]).min(0.0) - radius;
                let coverage = (0.5 - outside).clamp(0.0, 1.0);
                let i = ((y * width + x) * 4) as usize;
                for channel in &mut rgba[i..i + 3] {
                    *channel = (f32::from(*channel) * coverage) as u8;
                }
                rgba[i + 3] = (coverage * 255.0) as u8;
            }
        }
    }

    #[test]
    fn coverage_keeps_the_middle_and_drops_the_corners() {
        let (w, h) = (BASE[0], BASE[1]);
        let coverage = bezel_coverage(w, h);
        assert_eq!(coverage.len(), (w * h) as usize);
        let at = |x: u32, y: u32| coverage[(y * w + x) as usize];
        assert_eq!(at(w / 2, h / 2), 255, "the middle is kept");
        assert_eq!(at(0, 0), 0, "the corner is clipped");
        assert_eq!(at(w - 1, h - 1), 0, "the far corner is clipped");
    }

    #[test]
    fn one_pass_matches_tint_then_clip() {
        let (w, h) = (BASE[0], BASE[1]);
        let coverage = bezel_coverage(w, h);
        let source: Vec<u8> = (0..w * h * 4).map(|i| (i * 37 % 256) as u8).collect();
        for tint in [MirrorTint::NONE, MirrorTint::STOCK, MirrorTint([37, 200, 91])] {
            let mut reference = source.clone();
            tint_rgba(&mut reference, tint);
            clip_reference(&mut reference, w, h);
            let mut one_pass = source.clone();
            apply_tint_and_clip(&mut one_pass, &coverage, tint);
            for (i, (a, b)) in one_pass.iter().zip(&reference).enumerate() {
                assert!(a.abs_diff(*b) <= 1, "tint {tint}: byte {i} is {a}, reference {b}");
            }
        }
    }

    #[test]
    fn the_mask_texture_carries_the_coverage_in_its_alpha() {
        let coverage = bezel_coverage(BASE[0], BASE[1]);
        let mask = mask_rgba(&coverage);
        assert_eq!(mask.len(), coverage.len() * 4);
        for (pixel, &cover) in mask.as_chunks::<4>().0.iter().zip(&coverage) {
            assert_eq!(pixel[3], cover);
        }
    }

    /// The mask texture's alpha is exactly the coverage the CPU clip uses, so the GPU's product (the integer
    /// arithmetic of `apply_tint_and_clip`) gives the CPU path's picture. Checked here through the CPU function with
    /// the coverage taken back out of the mask texture's bytes.
    #[test]
    fn clipping_by_the_mask_texture_is_the_cpu_clip() {
        let (w, h) = (BASE[0], BASE[1]);
        let coverage = bezel_coverage(w, h);
        let from_mask: Vec<u8> = mask_rgba(&coverage).as_chunks::<4>().0.iter().map(|pixel| pixel[3]).collect();
        let source: Vec<u8> = (0..w * h * 4).map(|i| (i * 53 % 256) as u8).collect();
        let (mut direct, mut via_mask) = (source.clone(), source);
        apply_tint_and_clip(&mut direct, &coverage, MirrorTint::STOCK);
        apply_tint_and_clip(&mut via_mask, &from_mask, MirrorTint::STOCK);
        assert_eq!(direct, via_mask);
    }

    struct RearView {
        params: FrameParams,
    }

    impl Scene for RearView {
        fn title(&self) -> String {
            String::new()
        }
        fn init(&mut self, _: &mut dyn RenderBackend) -> anyhow::Result<()> {
            Ok(())
        }
        fn update(&mut self, _: &mut dyn RenderBackend, _: &crate::input::ActionState, _: f32) {}
        fn frame(&mut self, _: f32) -> (FrameParams, &[Instance]) {
            unreachable!()
        }
        fn rear_view(&mut self, _: f32) -> Option<(FrameParams, &[Instance])> {
            Some((self.params, &[][..]))
        }
    }

    fn scene() -> RearView {
        RearView {
            params: FrameParams {
                view: glam::Mat4::IDENTITY,
                projection: blackbox_gfx::Projection::Identity,
                camera_position: glam::Vec3::ZERO,
                light_dir: glam::Vec3::NEG_Z,
                clear_color: [0.0; 3],
                fog: None,
                scattering: None,
                surface_light: None,
                camera_cut: false,
                shadow_tuning: None,
            },
        }
    }

    fn backend(gpu: bool) -> NullBackend {
        let backend = NullBackend::new(Capabilities::baseline("test", blackbox_gfx::GraphicsApi::Auto));
        match gpu {
            true => backend.with_view_capture_ui(),
            false => backend,
        }
    }

    #[test]
    fn a_backend_that_can_draws_the_mirror_on_the_gpu_with_its_mask_uploaded_once() {
        let mut backend = backend(true);
        let mut mirror = MirrorOverlay::default();
        let mut scene = scene();
        for _ in 0..3 {
            mirror.update(&mut backend, &mut scene);
        }
        assert_eq!(backend.capture_requests, 0, "no readback capture is started");
        assert_eq!(backend.view_capture_ui_requests.len(), 3, "a picture every frame");
        let target = backend.view_capture_ui_requests[0];
        assert_eq!((target.texture, target.mask), (MIRROR_TEXTURE, MIRROR_MASK));
        assert_eq!(target.tint, backend.graphics().mirror_tint.0);
        assert!(backend.ui_texture_sizes.contains_key(&MIRROR_MASK), "the mask is uploaded");
        assert_eq!(backend.ui_texture_uploads[&MIRROR_MASK], 1, "the mask goes up once for an unchanged size");
    }

    #[test]
    fn a_backend_that_cannot_keeps_the_readback_path() {
        let mut backend = backend(false);
        let mut mirror = MirrorOverlay::default();
        let mut scene = scene();
        mirror.update(&mut backend, &mut scene);
        assert_eq!(backend.capture_requests, 1, "the capture is read back");
        assert!(backend.view_capture_ui_requests.is_empty());
        assert!(!backend.ui_texture_sizes.contains_key(&MIRROR_MASK), "no mask is uploaded for it");
    }
}

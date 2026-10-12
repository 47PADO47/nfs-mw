//! The rear-view mirror's picture. Each frame the scene's rear view is captured offscreen and uploaded as the
//! mirror's UI texture; the HUD presenter draws it under the mirror's bezel (`ui::present::blackbox`).

use blackbox_gfx::{CaptureId, RenderBackend, UiLayer, UiTexturePatch};

use crate::ui::present::blackbox::MIRROR_TEXTURE;
use crate::viewer::Scene;

/// Capture size in pixels: the bezel's proportions (about 256 by 87 HUD units).
const SIZE: [u32; 2] = [384, 131];
/// Inset of the picture from the captured image's edge, in pixels, so the clip does not cut the bezel's edge.
const INSET: f32 = 2.0;

#[derive(Default)]
pub struct MirrorOverlay {
    pending: Option<CaptureId>,
    /// The texture has a picture (or the blank one) uploaded.
    uploaded: bool,
    /// The HUD and UI layer as the presenter last made it, drawn again after a capture.
    layer: Option<UiLayer>,
}

impl MirrorOverlay {
    /// Sets the frame's UI layer, and keeps it so the rear view can be captured without it.
    /// The HUD and UI layer as the presenter last made it, without the streaks.
    pub fn base_layer(&self) -> Option<&UiLayer> {
        self.layer.as_ref()
    }

    pub fn set_layer(&mut self, renderer: &mut dyn RenderBackend, layer: UiLayer) {
        renderer.set_ui_layer(layer.clone());
        self.layer = Some(layer);
    }

    /// Collect a finished capture, then ask for the next one when the scene has a rear view.
    pub fn update(&mut self, renderer: &mut dyn RenderBackend, scene: &mut dyn Scene) {
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
        if let Some(id) = self.pending {
            match renderer.poll_capture(id) {
                None => return,
                Some(Ok(mut image)) => {
                    clip_to_bezel(&mut image.rgba, image.width, image.height);
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
        }
        let aspect = SIZE[0] as f32 / SIZE[1] as f32;
        let Some((params, instances)) = scene.rear_view(aspect) else { return };
        // The capture draws the UI like the screen does; the mirror shows the road, so the HUD is left out.
        renderer.set_ui_layer(UiLayer::default());
        let started = renderer.request_capture(SIZE, &params, instances);
        if let Some(layer) = &self.layer {
            renderer.set_ui_layer(layer.clone());
        }
        match started {
            Ok(id) => self.pending = Some(id),
            Err(e) => log::warn!("rear-view mirror capture could not start: {e}"),
        }
    }
}

/// Clips the picture to a rounded rectangle, the shape of the bezel's opening. Premultiplies the colour so the
/// edge blends the way the rest of the UI does.
fn clip_to_bezel(rgba: &mut [u8], width: u32, height: u32) {
    let (w, h) = (width as f32, height as f32);
    let radius = h * 0.35;
    let half = [w * 0.5 - INSET - radius, h * 0.5 - INSET - radius];
    for y in 0..height {
        for x in 0..width {
            let p = [x as f32 + 0.5 - w * 0.5, y as f32 + 0.5 - h * 0.5];
            // Signed distance to the rounded rectangle: negative inside.
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clip_keeps_the_middle_and_drops_the_corners() {
        let (w, h) = (SIZE[0], SIZE[1]);
        let mut rgba = vec![200u8; (w * h * 4) as usize];
        clip_to_bezel(&mut rgba, w, h);
        let alpha = |x: u32, y: u32| rgba[((y * w + x) * 4 + 3) as usize];
        assert_eq!(alpha(w / 2, h / 2), 255, "the middle is kept");
        assert_eq!(alpha(0, 0), 0, "the corner is clipped");
        assert_eq!(alpha(w - 1, h - 1), 0, "the far corner is clipped");
    }
}

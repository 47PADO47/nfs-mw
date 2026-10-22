//! Retained capture inputs, consumed after current-frame shadow preparation.
use super::Renderer;
use blackbox_gfx::{FrameParams, Instance, ReflectionScene};

pub(super) struct Pending {
    ready: bool,
    shadows: bool,
    origin: glam::Vec3,
    quality: blackbox_gfx::ReflectionQuality,
    fade: Option<f32>,
    frame: FrameParams,
    faces: [Vec<Instance>; 6],
}

impl Renderer {
    pub(super) fn queue_reflections(&mut self, scene: Option<ReflectionScene<'_>>) {
        let Some(scene) = scene.filter(|s| s.quality != blackbox_gfx::ReflectionQuality::Off) else {
            if let Some(pending) = &mut self.pending_reflections {
                pending.ready = false;
            }
            self.capture_reflections(None, None);
            return;
        };
        let pending = self.pending_reflections.get_or_insert_with(|| Pending {
            ready: false,
            shadows: false,
            origin: scene.origin,
            quality: scene.quality,
            fade: scene.fade_distance,
            frame: scene.frame,
            faces: std::array::from_fn(|_| Vec::new()),
        });
        pending.ready = true;
        pending.shadows = scene.sun_shadows;
        pending.origin = scene.origin;
        pending.quality = scene.quality;
        pending.fade = scene.fade_distance;
        pending.frame = scene.frame;
        for (stored, face) in pending.faces.iter_mut().zip(scene.faces) {
            stored.clear();
            stored.extend_from_slice(face);
        }
    }

    pub(super) fn encode_reflections(&mut self, encoder: &mut wgpu::CommandEncoder, allow_shadowed: bool) {
        let Some(mut pending) = self.pending_reflections.take() else { return };
        if !pending.ready || (pending.shadows && !allow_shadowed) {
            self.pending_reflections = Some(pending);
            return;
        }
        self.capture_reflections(
            Some(ReflectionScene {
                sun_shadows: pending.shadows,
                fade_distance: pending.fade,
                origin: pending.origin,
                quality: pending.quality,
                frame: pending.frame,
                faces: pending.faces.each_ref().map(Vec::as_slice),
            }),
            Some(encoder),
        );
        pending.ready = false;
        self.pending_reflections = Some(pending);
    }
}

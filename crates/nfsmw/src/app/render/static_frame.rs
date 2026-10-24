//! Spotting a frame that looks like the last one. A menu backdrop or a paused world holds still once its camera has
//! settled, so after a few identical frames the renderer keeps the finished scene image and shows it again, drawing
//! only the UI over it (`RenderBackend::set_scene_reuse`). Any doubt draws the frame normally.

use std::hash::Hasher;

use blackbox_gfx::{FrameParams, Instance, RenderBackend, SceneReuse};

use crate::settings::Settings;
use crate::viewer::StaticHash;

/// Identical frames in a row before the renderer is asked to keep one. The first ones after a change still settle what
/// the renderer finishes over a few frames (shadow cascades, reflection faces), so the kept image is a settled one.
const KEEP_AFTER: u32 = 3;

/// What the last frames drew.
#[derive(Default)]
pub struct StaticFrames {
    /// The fingerprint of the last frame and the settings it was drawn under.
    last: Option<(u64, Settings)>,
    /// Frames in a row that drew exactly what the one before did.
    stable: u32,
}

impl StaticFrames {
    /// Whether this frame will most likely be shown from the kept image, so the scene can skip the renderer work it does
    /// before a frame (view-dependent captures). A frame that turns out different anyway is drawn without them once.
    pub fn settled(&self, renderer: &dyn RenderBackend) -> bool {
        self.stable >= KEEP_AFTER && renderer.scene_reuse_ready()
    }

    /// Forget the streak: the next frames start counting again.
    pub fn reset(&mut self) {
        (self.last, self.stable) = (None, 0);
    }

    /// What the renderer does about this frame's scene image. `still` says the scene is one that holds still (a menu is
    /// up over it, see `Scene::menu_frame`); `scene` counts the scenes put in the window, so a new one starts over.
    pub fn next(
        &mut self,
        renderer: &dyn RenderBackend,
        (still, scene): (bool, u32),
        settings: &Settings,
        params: &FrameParams,
        instances: &[Instance],
    ) -> SceneReuse {
        if !still {
            self.reset();
            return SceneReuse::Off;
        }
        let print = fingerprint(scene, params, instances);
        let unchanged = self.last.as_ref().is_some_and(|(last, under)| *last == print && under == settings);
        self.last = Some((print, *settings));
        if !unchanged {
            self.stable = 0;
            return SceneReuse::Off;
        }
        self.stable = self.stable.saturating_add(1);
        if self.stable < KEEP_AFTER {
            return SceneReuse::Off;
        }
        match renderer.scene_reuse_ready() {
            true => SceneReuse::Reuse,
            false => SceneReuse::Keep,
        }
    }
}

/// The frame's parameters and instances, and which scene they are of.
fn fingerprint(scene: u32, params: &FrameParams, instances: &[Instance]) -> u64 {
    let mut hash = StaticHash::default();
    hash.write_u32(scene);
    hash.params(params);
    hash.instances(instances);
    hash.finish()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::null_backend::NullBackend;
    use blackbox_gfx::{Capabilities, GraphicsApi, MeshHandle, Projection};
    use glam::{Mat4, Vec3};

    fn params() -> FrameParams {
        FrameParams {
            view: Mat4::IDENTITY,
            projection: Projection::Identity,
            camera_position: Vec3::ZERO,
            light_dir: Vec3::NEG_Z,
            clear_color: [0.0; 3],
            fog: None,
            scattering: None,
            surface_light: None,
            camera_cut: false,
            shadow_tuning: None,
        }
    }

    #[test]
    fn a_still_frame_is_kept_after_a_few_identical_ones_and_a_change_starts_over() {
        let renderer = NullBackend::new(Capabilities::baseline("test", GraphicsApi::Auto));
        let settings = Settings::from(crate::settings::Partial::default());
        let list = [Instance::new(MeshHandle::from_raw(1), Mat4::IDENTITY)];
        let mut frames = StaticFrames::default();
        let mut modes = Vec::new();
        for _ in 0..KEEP_AFTER + 2 {
            modes.push(frames.next(&renderer, (true, 0), &settings, &params(), &list));
        }
        // The null backend never has a kept image, so a settled streak asks for one each frame.
        assert_eq!(modes[0], SceneReuse::Off);
        assert_eq!(modes[KEEP_AFTER as usize], SceneReuse::Keep);
        let moved = [Instance::new(MeshHandle::from_raw(1), Mat4::from_translation(Vec3::X))];
        assert_eq!(frames.next(&renderer, (true, 0), &settings, &params(), &moved), SceneReuse::Off);
        assert_eq!(frames.next(&renderer, (false, 0), &settings, &params(), &moved), SceneReuse::Off);
    }
}

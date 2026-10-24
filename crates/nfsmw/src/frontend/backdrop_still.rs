//! A still backdrop's renderer inputs. The shadow casters and the floor capture are made from the camera, the car's and
//! the room's instances and a few settings; when none of them changed since the last update the renderer already holds
//! the same result, so it is not handed again (the floor capture is a second car render with its own submit).
use std::hash::Hasher;

use blackbox_gfx::{GraphicsSettings, RenderBackend, RenderExperiments, TextureHandle};

use super::MenuScene;
use crate::settings::{CarShading, fe_shadows};
use crate::viewer::StaticHash;

/// What the casters and the floor capture were made from. Equal keys make equal results.
#[derive(PartialEq)]
pub(super) struct StillKey {
    graphics: GraphicsSettings,
    experiments: RenderExperiments,
    surface: [u32; 2],
    shading: CarShading,
    shadows: fe_shadows::Snapshot,
    receiver: Option<TextureHandle>,
    /// The camera's matrices and every instance the casters and the capture read.
    content: u64,
}

impl MenuScene {
    /// Whether the casters and the floor capture would come out as they did at the last call; remembers this call's
    /// inputs for the next.
    pub(super) fn still_unchanged(&mut self, renderer: &dyn RenderBackend) -> bool {
        let key = self.still_key(renderer);
        let same = self.still.as_ref() == Some(&key);
        self.still = Some(key);
        same
    }

    fn still_key(&self, renderer: &dyn RenderBackend) -> StillKey {
        let surface = renderer.surface_size();
        let aspect = surface[0] as f32 / surface[1].max(1) as f32;
        let mut hash = StaticHash::default();
        if let Some(car) = &self.car {
            let (frame, parts) = car.snapshot(aspect);
            hash.params(&frame);
            hash.instances(parts);
        }
        if let Some(pose) = self.rig.pose().filter(|_| self.look.camera_intro().is_some()) {
            let lens = pose.lens(aspect);
            hash.mat4(&lens.view);
            hash.projection(&lens.projection);
            hash.vec3(lens.position);
        }
        if let Some(room) = &self.room {
            hash.instances(room.shadow_casters());
        }
        StillKey {
            graphics: *renderer.graphics(),
            experiments: renderer.render_experiments(),
            surface,
            shading: self.shading,
            shadows: fe_shadows::active(),
            receiver: self.room.as_ref().and_then(|room| room.floor_texture()),
            content: hash.finish(),
        }
    }
}

//! The split screen's picture: each player's half is the world seen from their camera, captured off-screen at the half's
//! size (the rear-view mirror's path, `app::mirror`) and drawn as a UI texture over the top or the bottom half of the
//! window, under the HUD. Spec: `docs/specs/split-screen.md` section 4.

use blackbox_gfx::{CaptureId, RenderBackend, UiLayer, UiMesh, UiTextureId, UiTexturePatch, UiVertex};

use crate::viewer::Scene;

/// The two halves' pictures: the top half is player 1's, the bottom half player 2's.
pub const TEXTURES: [UiTextureId; 2] = [UiTextureId(0x5350_4C49_5430), UiTextureId(0x5350_4C49_5431)];

const WHITE: [u8; 4] = [255; 4];

#[derive(Default)]
pub struct SplitOverlay {
    pending: [Option<CaptureId>; 2],
    /// Which half asks for its next picture first (the other follows on the next frame), so both stay as fresh.
    turn: usize,
    /// The base layer the composed one was made from, and the window size it was made for.
    composed_from: Option<(UiLayer, [u32; 2])>,
    /// The layer the renderer has now, to put back after a capture that cannot leave the UI out.
    composed: Option<UiLayer>,
}

/// A mesh of one half: the picture over `[top, bottom)` of a window of `size` points, drawn opaque.
pub fn half_mesh(player: usize, size: [f32; 2]) -> UiMesh {
    let (top, bottom) = match player {
        0 => (0.0, size[1] * 0.5),
        _ => (size[1] * 0.5, size[1]),
    };
    let corners = [[0.0, top], [size[0], top], [size[0], bottom], [0.0, bottom]];
    let uvs = [[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]];
    UiMesh {
        vertices: corners
            .into_iter()
            .zip(uvs)
            .map(|(position, uv)| UiVertex { position, uv, color_rgba: WHITE })
            .collect(),
        indices: vec![0, 1, 2, 0, 2, 3],
        texture: TEXTURES[player.min(1)],
        clip: [0.0, top, size[0], bottom],
    }
}

/// The HUD layer with the two pictures under it.
pub fn compose(base: &UiLayer, size: [f32; 2]) -> UiLayer {
    let mut meshes = Vec::with_capacity(base.meshes.len() + 2);
    meshes.push(half_mesh(0, size));
    meshes.push(half_mesh(1, size));
    meshes.extend_from_slice(&base.meshes);
    UiLayer { compositions: base.compositions.clone(), pixels_per_point: base.pixels_per_point, meshes }
}

/// What the main frame draws while the halves are the picture: nothing.
pub fn blank_frame() -> blackbox_gfx::FrameParams {
    blackbox_gfx::FrameParams {
        view: glam::Mat4::IDENTITY,
        projection: blackbox_gfx::Projection::Identity,
        camera_position: glam::Vec3::ZERO,
        light_dir: glam::Vec3::NEG_Z,
        clear_color: [0.0, 0.0, 0.0],
        fog: None,
        scattering: None,
        surface_light: None,
        camera_cut: false,
        shadow_tuning: None,
    }
}

/// The size in pixels of a half of a surface `surface` pixels big.
pub fn half_size(surface: [u32; 2]) -> [u32; 2] {
    [surface[0].max(1), (surface[1] / 2).max(1)]
}

impl SplitOverlay {
    /// One frame of a split-screen race: collect the finished pictures, put the layer with them under the HUD, and ask for
    /// the next pictures. `base` is the HUD layer as the presenter made it; `window` the window size in points.
    pub fn update(
        &mut self,
        renderer: &mut dyn RenderBackend,
        scene: &mut dyn Scene,
        base: Option<&UiLayer>,
        window: [f32; 2],
    ) {
        let _span = bevy_log::info_span!("render::split").entered();
        let _trace = blackbox_trace::scope("Render bridge: split screen");
        let surface = renderer.surface_size();
        self.collect(renderer);
        self.show_layer(renderer, base, window, surface);
        self.request(renderer, scene, surface);
    }

    /// The split screen is over: the HUD layer goes back as it was and the pictures are forgotten.
    pub fn stop(&mut self, renderer: &mut dyn RenderBackend, base: Option<&UiLayer>) {
        self.pending = [None; 2];
        self.composed = None;
        if self.composed_from.take().is_none() {
            return;
        }
        if let Some(base) = base {
            renderer.set_ui_layer(base.clone());
        }
    }

    fn collect(&mut self, renderer: &mut dyn RenderBackend) {
        for (player, slot) in self.pending.iter_mut().enumerate() {
            let Some(id) = *slot else { continue };
            match renderer.poll_capture(id) {
                None => continue,
                Some(Ok(image)) => renderer.update_ui_texture(&UiTexturePatch {
                    id: TEXTURES[player],
                    offset: None,
                    size: [image.width, image.height],
                    rgba: &image.rgba,
                }),
                Some(Err(e)) => log::warn!("split screen: the capture of player {} failed: {e}", player + 1),
            }
            *slot = None;
        }
    }

    fn show_layer(
        &mut self,
        renderer: &mut dyn RenderBackend,
        base: Option<&UiLayer>,
        window: [f32; 2],
        surface: [u32; 2],
    ) {
        let Some(base) = base else { return };
        let unchanged = self.composed_from.as_ref().is_some_and(|(from, size)| from == base && *size == surface);
        if unchanged {
            return;
        }
        let layer = compose(base, window);
        renderer.set_ui_layer(layer.clone());
        self.composed = Some(layer);
        self.composed_from = Some((base.clone(), surface));
    }

    fn request(&mut self, renderer: &mut dyn RenderBackend, scene: &mut dyn Scene, surface: [u32; 2]) {
        let size = half_size(surface);
        let aspect = size[0] as f32 / size[1] as f32;
        let hides_ui = !renderer.capabilities().view_capture_without_ui;
        for step in 0..2 {
            let player = (self.turn + step) % 2;
            if self.pending[player].is_some() {
                continue;
            }
            let Some((params, instances)) = scene.split_view(player, aspect) else { continue };
            if hides_ui {
                renderer.set_ui_layer(UiLayer::default());
            }
            let started = renderer.request_view_capture(size, &params, instances);
            if let Some(layer) = self.composed.as_ref().filter(|_| hides_ui) {
                renderer.set_ui_layer(layer.clone());
            }
            match started {
                Ok(id) => self.pending[player] = Some(id),
                Err(e) => log::warn!("split screen: the capture of player {} could not start: {e}", player + 1),
            }
        }
        self.turn = (self.turn + 1) % 2;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_halves_split_the_window_in_the_middle() {
        let top = half_mesh(0, [1280.0, 720.0]);
        let bottom = half_mesh(1, [1280.0, 720.0]);
        assert_eq!(top.clip, [0.0, 0.0, 1280.0, 360.0]);
        assert_eq!(bottom.clip, [0.0, 360.0, 1280.0, 720.0]);
        assert_eq!(top.vertices[2].position, [1280.0, 360.0]);
        assert_eq!(bottom.vertices[0].position, [0.0, 360.0]);
        assert_ne!(top.texture, bottom.texture);
    }

    #[test]
    fn the_pictures_go_under_the_hud() {
        let hud =
            UiMesh { vertices: Vec::new(), indices: Vec::new(), texture: UiTextureId(7), clip: [0.0, 0.0, 1.0, 1.0] };
        let base = UiLayer { pixels_per_point: 2.0, meshes: vec![hud.clone()], compositions: Vec::new() };
        let layer = compose(&base, [640.0, 360.0]);
        assert_eq!(layer.pixels_per_point, 2.0);
        assert_eq!(layer.meshes.len(), 3);
        assert_eq!(layer.meshes[0].texture, TEXTURES[0]);
        assert_eq!(layer.meshes[1].texture, TEXTURES[1]);
        assert_eq!(layer.meshes[2], hud, "the HUD is drawn last, on top");
    }

    #[test]
    fn a_capture_is_half_the_surface_high() {
        assert_eq!(half_size([1920, 1080]), [1920, 540]);
        assert_eq!(half_size([1281, 721]), [1281, 360]);
        assert_eq!(half_size([0, 0]), [1, 1], "never a zero-sized capture");
    }
}

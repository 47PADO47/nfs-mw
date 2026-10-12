//! The high-speed streaks drawn over the view: short lines that radiate from the centre of the window and move out
//! as the car goes faster (`blackbox_speedfx`, `docs/specs/speed-effects.md`). They are a UI mesh on top of the HUD
//! layer, appended each frame: the presenter only emits a new HUD layer when something changes, so the streaks are
//! composed onto the last one here.

use blackbox_gfx::{UiLayer, UiMesh, UiTextureId, UiTexturePatch, UiVertex};

/// The 1x1 white texture the streaks sample; uploaded once.
pub const TEXTURE: UiTextureId = UiTextureId(0x5354_5245_414B);
/// Streaks on screen at full strength.
const STREAKS: usize = 48;
/// Line width, points.
const WIDTH: f32 = 1.5;
/// The golden angle, radians: spreads the streaks evenly round the centre.
const GOLDEN: f32 = 2.399_963;
const WHITE: [u8; 4] = [255; 4];

/// The white texture patch, to upload once.
pub fn white_texture() -> UiTexturePatch<'static> {
    UiTexturePatch { id: TEXTURE, offset: None, size: [1, 1], rgba: &WHITE }
}

/// A deterministic number in `0..1` for streak `i`, so each streak keeps its own brightness and phase.
fn seed(i: usize) -> f32 {
    let x = (i as f32 + 1.0) * 12.9898;
    (x.sin() * 43_758.547).fract().abs()
}

/// The streak mesh for a window of `size` points, at `strength` (0 to 1) and `time` seconds. `None` when there is
/// nothing to draw.
pub fn mesh(size: [f32; 2], strength: f32, time: f32) -> Option<UiMesh> {
    if !(strength > 0.0 && strength.is_finite()) {
        return None;
    }
    let centre = [size[0] * 0.5, size[1] * 0.5];
    let reach = size[0].hypot(size[1]) * 0.5;
    let mut vertices = Vec::with_capacity(STREAKS * 4);
    let mut indices = Vec::with_capacity(STREAKS * 6);
    for i in 0..STREAKS {
        let s = seed(i);
        let angle = (i as f32 + s) * GOLDEN;
        let (dir, side) = ([angle.cos(), angle.sin()], [-angle.sin() * WIDTH * 0.5, angle.cos() * WIDTH * 0.5]);
        // Each streak moves outward once per cycle, faster for the brighter ones, and fades in as it leaves.
        let phase = (time * (0.8 + s) + s).fract();
        let inner = reach * (0.15 + 0.6 * phase);
        let outer = inner + reach * (0.08 + 0.12 * s);
        let alpha = (strength * (0.3 + 0.7 * s) * phase.min(1.0 - phase) * 2.0 * 255.0).clamp(0.0, 255.0) as u8;
        let at = |r: f32, o: f32| [centre[0] + dir[0] * r + o * side[0], centre[1] + dir[1] * r + o * side[1]];
        let base = vertices.len() as u32;
        for position in [at(inner, 1.0), at(inner, -1.0), at(outer, -1.0), at(outer, 1.0)] {
            let colour = [alpha, alpha, alpha, alpha];
            vertices.push(UiVertex { position, uv: [0.5, 0.5], color_rgba: colour });
        }
        indices.extend_from_slice(&[base, base + 1, base + 2, base, base + 2, base + 3]);
    }
    Some(UiMesh { vertices, indices, texture: TEXTURE, clip: [0.0, 0.0, size[0], size[1]] })
}

/// The HUD layer with the streak mesh drawn over it.
pub fn compose(base: &UiLayer, streaks: UiMesh) -> UiLayer {
    let mut layer = base.clone();
    layer.meshes.push(streaks);
    layer
}

/// Holds the streaks' texture and whether the last frame drew them, so the HUD layer is restored when they stop.
#[derive(Default)]
pub struct StreakOverlay {
    uploaded: bool,
    drawn: bool,
}

impl StreakOverlay {
    /// Sets this frame's layer: the HUD layer `base` (if the presenter has made one) with the streaks on top.
    pub fn show(
        &mut self,
        renderer: &mut dyn blackbox_gfx::RenderBackend,
        base: Option<&UiLayer>,
        strength: f32,
        size: [f32; 2],
        time: f32,
    ) {
        if !self.uploaded {
            renderer.update_ui_texture(&white_texture());
            self.uploaded = true;
        }
        let Some(base) = base else { return };
        let streaks = mesh(size, strength, time);
        match (streaks, self.drawn) {
            (Some(streaks), _) => {
                renderer.set_ui_layer(compose(base, streaks));
                self.drawn = true;
            }
            (None, true) => {
                renderer.set_ui_layer(base.clone());
                self.drawn = false;
            }
            (None, false) => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SIZE: [f32; 2] = [1280.0, 720.0];

    #[test]
    fn nothing_is_drawn_at_no_strength() {
        assert!(mesh(SIZE, 0.0, 1.0).is_none());
        assert!(mesh(SIZE, f32::NAN, 1.0).is_none());
    }

    #[test]
    fn each_streak_is_a_quad_of_two_triangles() {
        let m = mesh(SIZE, 1.0, 0.3).expect("drawn");
        assert_eq!(m.vertices.len(), STREAKS * 4);
        assert_eq!(m.indices.len(), STREAKS * 6);
        assert_eq!(m.texture, TEXTURE);
        assert_eq!(m.clip, [0.0, 0.0, SIZE[0], SIZE[1]]);
    }

    #[test]
    fn stronger_streaks_are_more_opaque() {
        let weak = mesh(SIZE, 0.2, 0.3).expect("drawn");
        let strong = mesh(SIZE, 1.0, 0.3).expect("drawn");
        let peak = |m: &UiMesh| m.vertices.iter().map(|v| v.color_rgba[3]).max().unwrap_or(0);
        assert!(peak(&strong) > peak(&weak));
    }

    #[test]
    fn streaks_move_outward_over_time() {
        let early = mesh(SIZE, 1.0, 0.05).expect("drawn");
        let later = mesh(SIZE, 1.0, 0.25).expect("drawn");
        let centre = [SIZE[0] * 0.5, SIZE[1] * 0.5];
        let radius = |m: &UiMesh| {
            let v = m.vertices[0].position;
            (v[0] - centre[0]).hypot(v[1] - centre[1])
        };
        assert!((radius(&early) - radius(&later)).abs() > 1.0, "the first streak moved");
    }

    #[test]
    fn the_seed_stays_in_the_unit_range() {
        for i in 0..STREAKS {
            let s = seed(i);
            assert!((0.0..1.0).contains(&s), "{s}");
        }
    }
}

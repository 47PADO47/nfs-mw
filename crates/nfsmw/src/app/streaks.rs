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

/// A mesh with nothing in it, the texture and the clip set; [`fill`] makes it the streaks.
fn empty_mesh() -> UiMesh {
    UiMesh { vertices: Vec::new(), indices: Vec::new(), texture: TEXTURE, clip: [0.0; 4] }
}

/// Rebuilds `mesh` as the streaks for a window of `size` points, at `strength` (0 to 1) and `time` seconds, reusing
/// its allocations. Returns `false`, with the mesh left as it was, when there is nothing to draw.
fn fill(mesh: &mut UiMesh, size: [f32; 2], strength: f32, time: f32) -> bool {
    if !(strength > 0.0 && strength.is_finite()) {
        return false;
    }
    let centre = [size[0] * 0.5, size[1] * 0.5];
    let reach = size[0].hypot(size[1]) * 0.5;
    mesh.vertices.clear();
    mesh.indices.clear();
    mesh.vertices.reserve(STREAKS * 4);
    mesh.indices.reserve(STREAKS * 6);
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
        let base = mesh.vertices.len() as u32;
        for position in [at(inner, 1.0), at(inner, -1.0), at(outer, -1.0), at(outer, 1.0)] {
            let colour = [alpha, alpha, alpha, alpha];
            mesh.vertices.push(UiVertex { position, uv: [0.5, 0.5], color_rgba: colour });
        }
        mesh.indices.extend_from_slice(&[base, base + 1, base + 2, base, base + 2, base + 3]);
    }
    mesh.texture = TEXTURE;
    mesh.clip = [0.0, 0.0, size[0], size[1]];
    true
}

/// The streak mesh for a window of `size` points, at `strength` (0 to 1) and `time` seconds. `None` when there is
/// nothing to draw.
#[cfg(test)]
fn mesh(size: [f32; 2], strength: f32, time: f32) -> Option<UiMesh> {
    let mut mesh = empty_mesh();
    fill(&mut mesh, size, strength, time).then_some(mesh)
}

/// Makes `composed` the HUD layer `base` with `streaks` drawn over it, reusing the allocations `composed` and the
/// layer it held last frame already have. Take the streak mesh back with `composed.meshes.pop()` to reuse it too.
fn compose_into(composed: &mut UiLayer, base: &UiLayer, streaks: UiMesh) {
    composed.assign_meshes(base);
    composed.compositions.clone_from(&base.compositions);
    composed.meshes.push(streaks);
}

/// Holds the streaks' texture and whether the last frame drew them, so the HUD layer is restored when they stop.
/// Also the buffers the streak layer is rebuilt in each frame, so a frame with streaks allocates nothing.
#[derive(Default)]
pub struct StreakOverlay {
    uploaded: bool,
    drawn: bool,
    /// The layer sent while streaks show: the HUD layer's meshes, then the streaks.
    composed: UiLayer,
    /// The streak mesh between frames.
    spare: Option<UiMesh>,
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
        let mut streaks = self.spare.take().unwrap_or_else(empty_mesh);
        if !fill(&mut streaks, size, strength, time) {
            self.spare = Some(streaks);
            if self.drawn {
                renderer.set_ui_layer_ref(base);
                self.drawn = false;
            }
            return;
        }
        compose_into(&mut self.composed, base, streaks);
        renderer.set_ui_layer_ref(&self.composed);
        self.spare = self.composed.meshes.pop();
        self.drawn = true;
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
    fn filling_again_gives_the_same_mesh_as_a_fresh_one() {
        let mut reused = mesh(SIZE, 1.0, 0.3).expect("drawn");
        assert!(fill(&mut reused, SIZE, 0.5, 0.7));
        assert_eq!(Some(reused), mesh(SIZE, 0.5, 0.7));
    }

    #[test]
    fn nothing_to_draw_leaves_the_mesh_alone() {
        let mut kept = mesh(SIZE, 1.0, 0.3).expect("drawn");
        let before = kept.clone();
        assert!(!fill(&mut kept, SIZE, 0.0, 0.7));
        assert_eq!(kept, before);
    }

    #[test]
    fn the_streaks_go_on_top_of_the_hud_layer_frame_after_frame() {
        let hud = |x: f32, quads: usize| UiLayer {
            pixels_per_point: 2.0,
            meshes: (0..quads)
                .map(|i| UiMesh {
                    vertices: vec![UiVertex { position: [x, i as f32], uv: [0.0; 2], color_rgba: [9; 4] }; 4],
                    indices: vec![0, 1, 2, 0, 2, 3],
                    texture: UiTextureId(i as u64),
                    clip: [0.0, 0.0, 10.0, 10.0],
                })
                .collect(),
            compositions: Vec::new(),
        };
        let mut composed = UiLayer::default();
        // A big HUD, then a smaller one, then a bigger: the layer always ends up as base plus streaks.
        for (x, quads) in [(1.0, 3), (2.0, 1), (3.0, 4)] {
            let base = hud(x, quads);
            compose_into(&mut composed, &base, mesh(SIZE, 1.0, 0.3).expect("drawn"));
            assert_eq!(composed.pixels_per_point, 2.0);
            assert_eq!(composed.meshes.len(), quads + 1);
            assert_eq!(composed.meshes[..quads], base.meshes[..]);
            assert_eq!(composed.meshes[quads], mesh(SIZE, 1.0, 0.3).expect("drawn"));
            composed.meshes.pop();
        }
    }

    #[test]
    fn the_seed_stays_in_the_unit_range() {
        for i in 0..STREAKS {
            let s = seed(i);
            assert!((0.0..1.0).contains(&s), "{s}");
        }
    }
}

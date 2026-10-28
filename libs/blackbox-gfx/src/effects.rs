//! Dynamic world-space triangles: grounded surface overlays, soft billboards and additive streaks.
//!
//! All are depth tested and drawn before the UI without writing depth.
//! The caller owns geometry, sorting, lifetime and budgets; buffers are reused across frames.

mod exclude;
mod ribbon;

use glam::{Mat4, Vec3, Vec4};

use crate::{BlendMode, TextureHandle};

/// A caller-ordered batch of textured world particles. Textures are owned by the caller.
#[derive(Debug, Clone)]
pub struct TexturedEffect {
    pub texture: TextureHandle,
    pub blend: BlendMode,
    pub vertices: Vec<EffectVertex>,
}

/// An emissive, turbulent volume. Local space is a unit cube; the flame flows along +Y.
/// The texture maps radial distance (U) and flame height (V, bottom to top).
#[derive(Debug, Clone)]
pub struct FlameVolume {
    pub texture: TextureHandle,
    pub to_world: Mat4,
    pub time: f32,
    pub seed: f32,
    /// Blue ignition root, heating through white into a warm yellow tail.
    pub blue: bool,
    /// Emission multiplier, independent of coverage.
    pub intensity: f32,
    pub appearance: FlameAppearance,
}

/// Per-volume tint, optical density and tip displacement in normalized local space.
#[derive(Debug, Clone)]
pub struct FlameAppearance {
    pub root_tint: Vec3,
    pub tail_tint: Vec3,
    pub density: f32,
    pub turbulence: f32,
    /// Warm flash shading with a luminous core; zero retains the reference lookup.
    pub hot_core: f32,
    /// Combined tip displacement. The first 35% stays straight; the rest bends quadratically.
    pub bend: Vec3,
    /// Optional contact plane in local coordinates, with signed distance in world metres.
    /// Positive is the flame side. Zero disables contact clipping.
    pub contact_plane: Vec4,
    /// Compensate for wider proxy bounds without changing the visible radius.
    pub radial_scale: f32,
    /// Optional RGB palette and blend weight, independent of the density lookup's channels.
    pub root_color: [f32; 4],
    pub tail_color: [f32; 4],
}

impl Default for FlameAppearance {
    fn default() -> Self {
        Self {
            root_tint: Vec3::ONE,
            tail_tint: Vec3::ONE,
            density: 1.0,
            turbulence: 1.0,
            hot_core: 0.0,
            bend: Vec3::ZERO,
            contact_plane: Vec4::ZERO,
            radial_scale: 1.0,
            root_color: [0.0; 4],
            tail_color: [0.0; 4],
        }
    }
}

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, bytemuck::Pod, bytemuck::Zeroable)]
pub struct EffectVertex {
    pub position: [f32; 3],
    pub color: [u8; 4],
    pub uv: [f32; 2],
    /// Procedural particles: age in seconds and stable variation seed.
    /// Textured particles: positive RGBA brightness multiplier in X (zero defaults to one), unused Y.
    /// Streaks: positive emission multiplier in X (zero defaults to one); glows ignore X.
    /// Streaks and glows: a Y of one takes the sun's shadow in the native renderer (Bevy ignores it).
    pub detail: [f32; 2],
}

/// World-unit distance over which an intersecting particle fades, unless the layer says otherwise.
pub const DEFAULT_SOFT_DISTANCE: f32 = 0.3;

#[derive(Debug, Clone)]
pub struct EffectLayer {
    /// Bounded emissive volumes, rendered against the completed scene depth.
    pub flames: Vec<FlameVolume>,
    /// Main-view optical distortion. Never drawn in world captures.
    pub distortion: Option<crate::ScreenDistortion>,
    /// Surface overlays with feathered edges and a subtle longitudinal pattern.
    pub surfaces: Vec<EffectVertex>,
    /// Soft circular billboards, in back-to-front order.
    pub particles: Vec<EffectVertex>,
    /// Additive streak triangles; UV x crosses the width, UV y runs from head (0) to tail (1).
    /// Overlap adds light independent of ordering within this batch.
    pub streaks: Vec<EffectVertex>,
    /// Additive circular glow billboards, using the same unit-quad UV coordinates.
    pub glows: Vec<EffectVertex>,
    pub textured: Vec<TexturedEffect>,
    /// Enable evolving procedural density and depth-softened intersections for particles.
    pub detailed_particles: bool,
    /// Distance in world units over which an intersecting particle fades.
    pub soft_distance: f32,
}

impl Default for EffectLayer {
    fn default() -> Self {
        Self {
            flames: Vec::new(),
            distortion: None,
            surfaces: Vec::new(),
            particles: Vec::new(),
            streaks: Vec::new(),
            glows: Vec::new(),
            textured: Vec::new(),
            detailed_particles: false,
            soft_distance: DEFAULT_SOFT_DISTANCE,
        }
    }
}

impl EffectLayer {
    pub fn clear(&mut self) {
        self.flames.clear();
        self.distortion = None;
        self.surfaces.clear();
        self.particles.clear();
        self.streaks.clear();
        self.glows.clear();
        self.textured.clear();
    }

    /// Append a quad, with corners in perimeter order and UVs from (0,0) to (1,1).
    pub fn quad(out: &mut Vec<EffectVertex>, corners: [Vec3; 4], color: [u8; 4]) {
        Self::particle_quad(out, corners, color, [0.0; 2]);
    }

    /// Append a particle quad with identical age/seed on each vertex.
    pub fn particle_quad(out: &mut Vec<EffectVertex>, corners: [Vec3; 4], color: [u8; 4], detail: [f32; 2]) {
        Self::particle_quad_uv(out, corners, color, detail, [[0.0; 2], [1.0; 2]]);
    }

    /// Append a particle quad sampling its selected atlas rectangle.
    pub fn particle_quad_uv(
        out: &mut Vec<EffectVertex>,
        corners: [Vec3; 4],
        color: [u8; 4],
        detail: [f32; 2],
        rect: [[f32; 2]; 2],
    ) {
        let [[u0, v0], [u1, v1]] = rect;
        let uv = [[u0, v0], [u1, v0], [u1, v1], [u0, v1]];
        for i in [0, 1, 2, 0, 2, 3] {
            out.push(EffectVertex { position: corners[i].to_array(), color, uv: uv[i], detail });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn atlas_quads_keep_the_selected_uv_rectangle_on_both_triangles() {
        let mut vertices = Vec::new();
        EffectLayer::particle_quad_uv(&mut vertices, [Vec3::ZERO; 4], [255; 4], [0.0; 2], [[0.25, 0.5], [0.5, 0.75]]);
        assert_eq!(
            vertices.iter().map(|v| v.uv).collect::<Vec<_>>(),
            [[0.25, 0.5], [0.5, 0.5], [0.5, 0.75], [0.25, 0.5], [0.5, 0.75], [0.25, 0.75]]
        );
    }

    #[test]
    fn a_new_layer_fades_intersections_over_the_default_distance() {
        let layer = EffectLayer::default();
        assert_eq!(layer.soft_distance, DEFAULT_SOFT_DISTANCE);
        assert!(!layer.detailed_particles);
        assert!(layer.streaks.is_empty());
    }

    #[test]
    fn clear_empties_all_batches_and_keeps_their_reusable_storage() {
        let mut layer = EffectLayer::default();
        let corners = [Vec3::ZERO, Vec3::X, Vec3::ONE, Vec3::Y];
        for out in [&mut layer.surfaces, &mut layer.particles, &mut layer.streaks, &mut layer.glows] {
            EffectLayer::quad(out, corners, [255; 4]);
        }
        let capacities =
            [layer.surfaces.capacity(), layer.particles.capacity(), layer.streaks.capacity(), layer.glows.capacity()];
        layer.clear();
        assert!(layer.surfaces.is_empty() && layer.particles.is_empty() && layer.streaks.is_empty());
        assert!(layer.glows.is_empty());
        assert_eq!(
            [layer.surfaces.capacity(), layer.particles.capacity(), layer.streaks.capacity(), layer.glows.capacity()],
            capacities
        );
    }
}

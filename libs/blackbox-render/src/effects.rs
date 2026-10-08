//! Dynamic world-space triangles: grounded surface overlays and soft billboards.
//!
//! Both are depth tested, alpha blended and drawn before the UI without writing depth.
//! The caller owns geometry, sorting, lifetime and budgets; buffers are reused across frames.

use glam::Vec3;

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, bytemuck::Pod, bytemuck::Zeroable)]
pub struct EffectVertex {
    pub position: [f32; 3],
    pub color: [u8; 4],
    pub uv: [f32; 2],
    /// Procedural particle age in seconds and stable variation seed.
    pub detail: [f32; 2],
}

/// World-unit distance over which an intersecting particle fades, unless the layer says otherwise.
pub const DEFAULT_SOFT_DISTANCE: f32 = 0.3;

#[derive(Debug)]
pub struct EffectLayer {
    /// Surface overlays with feathered edges and a subtle longitudinal pattern.
    pub surfaces: Vec<EffectVertex>,
    /// Soft circular billboards, in back-to-front order.
    pub particles: Vec<EffectVertex>,
    /// Enable evolving procedural density and depth-softened intersections for particles.
    pub detailed_particles: bool,
    /// Distance in world units over which an intersecting particle fades.
    pub soft_distance: f32,
}

impl Default for EffectLayer {
    fn default() -> Self {
        Self {
            surfaces: Vec::new(),
            particles: Vec::new(),
            detailed_particles: false,
            soft_distance: DEFAULT_SOFT_DISTANCE,
        }
    }
}

impl EffectLayer {
    pub fn clear(&mut self) {
        self.surfaces.clear();
        self.particles.clear();
    }

    /// Append a quad, with corners in perimeter order and UVs from (0,0) to (1,1).
    pub fn quad(out: &mut Vec<EffectVertex>, corners: [Vec3; 4], color: [u8; 4]) {
        Self::particle_quad(out, corners, color, [0.0; 2]);
    }

    /// Append a particle quad with identical age/seed on each vertex.
    pub fn particle_quad(out: &mut Vec<EffectVertex>, corners: [Vec3; 4], color: [u8; 4], detail: [f32; 2]) {
        let uv = [[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]];
        for i in [0, 1, 2, 0, 2, 3] {
            out.push(EffectVertex { position: corners[i].to_array(), color, uv: uv[i], detail });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_new_layer_fades_intersections_over_the_default_distance() {
        let layer = EffectLayer::default();
        assert_eq!(layer.soft_distance, DEFAULT_SOFT_DISTANCE);
        assert!(!layer.detailed_particles);
    }
}

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
}

#[derive(Debug, Default)]
pub struct EffectLayer {
    /// Surface overlays with feathered edges and a subtle longitudinal pattern.
    pub surfaces: Vec<EffectVertex>,
    /// Soft circular billboards, in back-to-front order.
    pub particles: Vec<EffectVertex>,
}

impl EffectLayer {
    pub fn clear(&mut self) {
        self.surfaces.clear();
        self.particles.clear();
    }

    /// Append a quad, with corners in perimeter order and UVs from (0,0) to (1,1).
    pub fn quad(out: &mut Vec<EffectVertex>, corners: [Vec3; 4], color: [u8; 4]) {
        let uv = [[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]];
        for i in [0, 1, 2, 0, 2, 3] {
            out.push(EffectVertex { position: corners[i].to_array(), color, uv: uv[i] });
        }
    }
}

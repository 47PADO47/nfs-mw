//! View-frustum culling for depth-[0, 1] clip spaces (wgpu, D3D).

use glam::{Mat4, Vec3, Vec4};

use crate::Aabb;

/// Six planes `(n, d)` with `n · p + d >= 0` inside.
#[derive(Debug, Clone, Copy)]
pub struct Frustum {
    planes: [Vec4; 6],
}

impl Frustum {
    /// Planes of a view-projection matrix whose clip space has depth in [0, 1].
    pub fn from_view_proj(m: &Mat4) -> Self {
        let rows = [m.row(0), m.row(1), m.row(2), m.row(3)];
        let normalize = |p: Vec4| p / p.truncate().length().max(f32::EPSILON);
        Self {
            planes: [
                normalize(rows[3] + rows[0]), // left
                normalize(rows[3] - rows[0]), // right
                normalize(rows[3] + rows[1]), // bottom
                normalize(rows[3] - rows[1]), // top
                normalize(rows[2]),           // near (z >= 0)
                normalize(rows[3] - rows[2]), // far (z <= w)
            ],
        }
    }

    /// Conservative: `false` only when the box is entirely outside one plane.
    pub fn intersects(&self, b: &Aabb) -> bool {
        self.planes.iter().all(|p| {
            let n = p.truncate();
            // The box corner furthest along the plane normal.
            let corner = Vec3::select(n.cmpge(Vec3::ZERO), b.max, b.min);
            n.dot(corner) + p.w >= 0.0
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn culls_boxes_behind_and_beside() {
        let proj = glam::camera::rh::proj::directx::perspective(1.0, 1.0, 0.1, 100.0);
        let view = glam::camera::rh::view::look_at_mat4(Vec3::ZERO, Vec3::new(0.0, 1.0, 0.0), Vec3::Z);
        let f = Frustum::from_view_proj(&(proj * view));
        let at = |x: f32, y: f32| Aabb::new([x - 0.5, y - 0.5, -0.5], [x + 0.5, y + 0.5, 0.5]);
        assert!(f.intersects(&at(0.0, 10.0)));
        assert!(!f.intersects(&at(0.0, -10.0)));
        assert!(!f.intersects(&at(50.0, 10.0)));
        assert!(!f.intersects(&at(0.0, 200.0)));
    }
}

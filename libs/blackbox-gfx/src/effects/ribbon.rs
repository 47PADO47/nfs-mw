//! Connected camera-facing ribbon with one head-to-tail coordinate across all segments.
use super::{EffectLayer, EffectVertex};
use glam::Vec3;

impl EffectLayer {
    /// Points run head to tail. Shared edge positions prevent gaps at turns.
    pub fn ribbon(
        out: &mut Vec<EffectVertex>,
        points: &[Vec3],
        width: f32,
        color: [u8; 4],
        camera: Vec3,
        forward: Vec3,
    ) {
        if points.len() < 2 || width <= 0.0 || !width.is_finite() || points.iter().any(|p| !p.is_finite()) {
            return;
        }
        let total: f32 = points.windows(2).map(|p| p[0].distance(p[1])).sum();
        if total < 1e-5 {
            return;
        }
        let mut along = 0.0;
        let mut previous = None;
        for (i, &point) in points.iter().enumerate() {
            if i > 0 {
                along += point.distance(points[i - 1]) / total;
            }
            let before = points[i.saturating_sub(1)];
            let after = points[(i + 1).min(points.len() - 1)];
            let tangent = (before - after).normalize_or(-forward);
            let view = (camera - point).normalize_or(-forward);
            // Start at a small emitter aperture; a broad first edge can spread across its owner.
            let root = (along * total / 0.35).clamp(0.0, 1.0);
            let radius = width * (0.12 + 0.88 * root * root * (3.0 - 2.0 * root));
            let side = tangent.cross(view).normalize_or(Vec3::Z) * radius;
            if let Some((last, last_side, start)) = previous {
                Self::particle_quad_uv(
                    out,
                    [last - last_side, last + last_side, point + side, point - side],
                    color,
                    [0.0; 2],
                    [[0.0, start], [1.0, along.min(1.0)]],
                );
            }
            previous = Some((point, side, along));
        }
    }
}

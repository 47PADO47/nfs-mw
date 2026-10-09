use blackbox_render::{EffectLayer, EffectVertex};
use glam::Vec3;

/// Camera-facing thin quad along a world-space segment; UV y runs head to tail.
pub(super) fn streak(
    out: &mut Vec<EffectVertex>,
    head: Vec3,
    tail: Vec3,
    width: f32,
    color: [u8; 4],
    camera: Vec3,
    forward: Vec3,
) {
    let axis = head - tail;
    if !head.is_finite() || !tail.is_finite() || axis.length_squared() < 1e-8 {
        return;
    }
    let view = (camera - (head + tail) * 0.5).normalize_or(-forward);
    let side = axis.cross(view).normalize_or(forward.cross(Vec3::Z).normalize_or(Vec3::Y)) * width;
    EffectLayer::quad(out, [head - side, head + side, tail + side, tail - side], color);
}

pub(super) fn color(rgba: [f32; 4], fade: f32) -> [u8; 4] {
    let mut result = rgba.map(|v| (v.clamp(0.0, 1.0) * 255.0).round() as u8);
    result[3] = (f32::from(result[3]) * fade.clamp(0.0, 1.0)).round() as u8;
    result
}

pub(super) fn noise(index: u64, salt: f32) -> f32 {
    ((index % 1_000_003) as f32 * 2.399_963 + salt).sin()
}

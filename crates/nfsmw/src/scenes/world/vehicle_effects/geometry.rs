use blackbox_gfx::{EffectLayer, EffectVertex};
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
    streak_range(out, head, tail, width, color, camera, forward, [0.0, 1.0]);
}

#[allow(clippy::too_many_arguments)]
pub(super) fn streak_range(
    out: &mut Vec<EffectVertex>,
    head: Vec3,
    tail: Vec3,
    width: f32,
    color: [u8; 4],
    camera: Vec3,
    forward: Vec3,
    range: [f32; 2],
) {
    if range[1] - range[0] < 1e-6 {
        return;
    }
    let (head, tail) = (head.lerp(tail, range[0]), head.lerp(tail, range[1]));
    let axis = head - tail;
    if !head.is_finite() || !tail.is_finite() || axis.length_squared() < 1e-8 {
        return;
    }
    let view = (camera - (head + tail) * 0.5).normalize_or(-forward);
    let side = axis.cross(view).normalize_or(forward.cross(Vec3::Z).normalize_or(Vec3::Y)) * width;
    let first = out.len();
    EffectLayer::quad(out, [head - side, head + side, tail + side, tail - side], color);
    for vertex in &mut out[first..] {
        vertex.uv[1] = range[0] + vertex.uv[1] * (range[1] - range[0]);
    }
}

pub(super) fn color(rgba: [f32; 4], fade: f32) -> [u8; 4] {
    let mut result = rgba.map(|v| (v.clamp(0.0, 1.0) * 255.0).trunc() as u8);
    result[3] = (f32::from(result[3]) * fade.clamp(0.0, 1.0)).trunc() as u8;
    result
}

pub(super) fn glow(out: &mut Vec<EffectVertex>, point: Vec3, radius: f32, color: [u8; 4], camera: Vec3, forward: Vec3) {
    let view = (camera - point).normalize_or(-forward);
    let right = view.cross(Vec3::Z).normalize_or(Vec3::Y) * radius;
    let up = right.normalize().cross(view) * radius;
    EffectLayer::quad(out, [point - right - up, point + right - up, point + right + up, point - right + up], color);
}

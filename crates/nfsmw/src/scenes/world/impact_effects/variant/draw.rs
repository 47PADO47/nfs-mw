//! The quads of a collision spark variant: a streak is a quad along the spark's velocity, a sprite a picture that faces the camera.
//! A spark is drawn as a textured effect (the batches of `ImpactEffects::textured`), so the brightness of the layer's colour goes in
//! `EffectVertex::detail`.

use blackbox_gfx::EffectVertex;
use blackbox_modding::content::{SparkKind, SparkLayer};
use glam::Vec3;

use super::spark::{Look, Spark};

/// Cells across and along the game's own streak picture.
const STREAK_WIDTH: u32 = 4;
const STREAK_HEIGHT: u32 = 16;

/// The picture of a streak that names none: white, brightest at the head (the top row, `v = 0`) and fading to nothing at the
/// tail, and soft at the sides. Returns its width, height and RGBA bytes.
pub(super) fn streak_pixels() -> (u32, u32, Vec<u8>) {
    let mut pixels = Vec::with_capacity((STREAK_WIDTH * STREAK_HEIGHT * 4) as usize);
    for row in 0..STREAK_HEIGHT {
        let along = 1.0 - (row as f32 + 0.5) / STREAK_HEIGHT as f32;
        for column in 0..STREAK_WIDTH {
            let across = 2.0 * (column as f32 + 0.5) / STREAK_WIDTH as f32 - 1.0;
            let alpha = (1.0 - across * across) * along;
            pixels.extend_from_slice(&[255, 255, 255, (alpha * 255.0).round().clamp(0.0, 255.0) as u8]);
        }
    }
    (STREAK_WIDTH, STREAK_HEIGHT, pixels)
}

/// Append the two triangles of the quad with `corners` (in the order top-left, top-right, bottom-right, bottom-left) and `uv`.
fn push_quad(out: &mut Vec<EffectVertex>, corners: [Vec3; 4], uv: [[f32; 2]; 4], look: &Look) {
    for i in [0, 1, 2, 0, 2, 3] {
        out.push(EffectVertex {
            position: corners[i].to_array(),
            color: look.color,
            uv: uv[i],
            detail: [look.brightness, 0.0],
        });
    }
}

/// Append the vertices of `spark` as the layer draws it. `right` and `up` face the camera along `forward`.
pub(super) fn push(
    out: &mut Vec<EffectVertex>,
    spark: &Spark,
    layer: &SparkLayer,
    look: &Look,
    forward: Vec3,
    right: Vec3,
    up: Vec3,
) {
    match layer.kind {
        SparkKind::Sprite => sprite(out, spark, layer, look, right, up),
        SparkKind::Streak => streak(out, spark, layer, look, forward, right),
    }
}

fn sprite(out: &mut Vec<EffectVertex>, spark: &Spark, layer: &SparkLayer, look: &Look, right: Vec3, up: Vec3) {
    let (x, y) = (right * (look.size * 0.5), up * (look.size * 0.5));
    let p = spark.point;
    push_quad(out, [p - x + y, p + x + y, p + x - y, p - x - y], spark.window(layer), look);
}

/// A quad `look.size` wide from the spark back along its velocity for `stretch` seconds of travel; one that is shorter than it is
/// wide (a spark that has stopped, or a layer with no stretch) is a square, so a streak with no `stretch` is a round dot.
fn streak(out: &mut Vec<EffectVertex>, spark: &Spark, layer: &SparkLayer, look: &Look, forward: Vec3, right: Vec3) {
    let direction = spark.velocity.normalize_or(Vec3::Z);
    let length = (spark.velocity.length() * layer.stretch).max(look.size);
    let side = direction.cross(forward).normalize_or(right) * (look.size * 0.5);
    let (head, tail) = (spark.point, spark.point - direction * length);
    if !head.is_finite() || !tail.is_finite() {
        return;
    }
    let uv = [[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]];
    push_quad(out, [head - side, head + side, tail + side, tail - side], uv, look);
}

#[cfg(test)]
#[path = "draw_tests.rs"]
mod tests;

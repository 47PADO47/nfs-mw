use super::super::fixtures::layer;
use super::super::spark::Birth;
use super::*;

/// The camera looks along +x with y to its left in the render space; `right` is -y.
const FORWARD: Vec3 = Vec3::X;
const RIGHT: Vec3 = Vec3::NEG_Y;
const UP: Vec3 = Vec3::Z;

fn spark(layer: &SparkLayer, velocity: Vec3) -> Spark {
    let birth =
        Birth { point: Vec3::new(10.0, 0.0, 1.0), normal: Vec3::Z, owner: Vec3::ZERO, strength: 1.0, hit: true };
    Spark { velocity, ..Spark::spawn(layer, 0, &birth, 1) }
}

fn look(size: f32) -> Look {
    Look { size, color: [255, 128, 0, 200], brightness: 6.0 }
}

#[test]
fn the_streak_picture_fades_from_the_head_to_the_tail_and_from_the_middle_to_the_sides() {
    let (width, height, pixels) = streak_pixels();
    assert_eq!(pixels.len(), (width * height * 4) as usize);
    let alpha = |column: u32, row: u32| pixels[((row * width + column) * 4 + 3) as usize];
    assert!(alpha(1, 0) > 200, "the head is bright: {}", alpha(1, 0));
    assert!(alpha(1, height - 1) < 30, "the tail fades out: {}", alpha(1, height - 1));
    assert!(alpha(0, 4) < alpha(1, 4) && alpha(3, 4) < alpha(2, 4), "the sides are softer");
    assert!(pixels.chunks(4).all(|pixel| pixel[..3] == [255, 255, 255]), "white: the colour is the layer's");
    assert!(pixels.chunks(4).all(|pixel| pixel[3] > 0), "no cell is fully clear, so the quad's edge is the picture's");
}

#[test]
fn a_streak_is_a_quad_along_the_velocity_as_wide_as_the_size_and_as_long_as_its_stretch() {
    let layer = SparkLayer { stretch: 0.125, ..layer() };
    let spark = spark(&layer, Vec3::new(0.0, 8.0, 0.0));
    let mut out = Vec::new();
    push(&mut out, &spark, &layer, &look(0.1), FORWARD, RIGHT, UP);
    assert_eq!(out.len(), 6, "two triangles");
    let head = spark.point;
    // 8 m/s for 0.125 s: one metre back along the velocity.
    let tail = head - Vec3::Y;
    let positions: Vec<Vec3> = out.iter().map(|vertex| Vec3::from(vertex.position)).collect();
    for corner in [head, tail] {
        let near: Vec<&Vec3> = positions.iter().filter(|p| (**p - corner).length() < 0.0501).collect();
        assert!(!near.is_empty(), "a corner pair sits half the width either side of {corner:?}");
    }
    let along: Vec<f32> = positions.iter().map(|p| p.y).collect();
    let (lowest, highest) =
        (along.iter().cloned().fold(f32::MAX, f32::min), along.iter().cloned().fold(f32::MIN, f32::max));
    assert!((highest - lowest - 1.0).abs() < 1e-4, "{lowest} {highest}");
    let width =
        positions.iter().map(|p| p.z).fold(f32::MIN, f32::max) - positions.iter().map(|p| p.z).fold(f32::MAX, f32::min);
    assert!((width - 0.1).abs() < 1e-4, "as wide as the size, in the view plane: {width}");
}

#[test]
fn the_vertices_carry_the_colour_and_the_brightness() {
    let layer = layer();
    let mut out = Vec::new();
    push(&mut out, &spark(&layer, Vec3::Y * 4.0), &layer, &look(0.1), FORWARD, RIGHT, UP);
    assert!(out.iter().all(|vertex| vertex.color == [255, 128, 0, 200] && vertex.detail == [6.0, 0.0]));
    let heads: Vec<_> = out.iter().filter(|vertex| vertex.uv[1] == 0.0).collect();
    assert_eq!(heads.len(), 3, "the head is the first row of the picture (v = 0), shared by 3 of the 6 vertices");
}

#[test]
fn a_streak_with_no_stretch_or_no_speed_is_a_square_dot() {
    let layer = SparkLayer { stretch: 0.0, ..layer() };
    for velocity in [Vec3::Y * 8.0, Vec3::ZERO] {
        let mut out = Vec::new();
        push(&mut out, &spark(&layer, velocity), &layer, &look(0.2), FORWARD, RIGHT, UP);
        assert_eq!(out.len(), 6);
        let positions: Vec<Vec3> = out.iter().map(|vertex| Vec3::from(vertex.position)).collect();
        let span = |axis: fn(&Vec3) -> f32| {
            positions.iter().map(axis).fold(f32::MIN, f32::max) - positions.iter().map(axis).fold(f32::MAX, f32::min)
        };
        let (a, b, c) = (span(|p| p.x), span(|p| p.y), span(|p| p.z));
        let mut spans = [a, b, c];
        spans.sort_by(f32::total_cmp);
        assert!(spans[0].abs() < 1e-4 && (spans[1] - 0.2).abs() < 1e-4 && (spans[2] - 0.2).abs() < 1e-4, "{spans:?}");
    }
}

#[test]
fn a_sprite_faces_the_camera_and_shows_its_cell() {
    let layer = SparkLayer { kind: SparkKind::Sprite, texture: Some("A.PNG".into()), grid: 2, fps: 0, ..layer() };
    let mut out = Vec::new();
    let spark = spark(&layer, Vec3::ZERO);
    push(&mut out, &spark, &layer, &look(0.5), FORWARD, RIGHT, UP);
    assert_eq!(out.len(), 6);
    for vertex in &out {
        let offset = Vec3::from(vertex.position) - spark.point;
        assert!(offset.dot(FORWARD).abs() < 1e-5, "in the plane across the view");
        assert!((offset.length() - 0.25 * 2f32.sqrt()).abs() < 1e-5);
    }
    let window = spark.window(&layer);
    assert_eq!(out[0].uv, window[0]);
    assert_eq!(out[1].uv, window[1]);
    assert_eq!(out[2].uv, window[2]);
    assert_eq!(out[5].uv, window[3]);
}

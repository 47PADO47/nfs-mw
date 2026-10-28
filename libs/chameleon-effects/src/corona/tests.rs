use super::*;
use crate::corona::pattern::{FlashPattern, Segment};

fn put_u16(bytes: &mut [u8], at: usize, value: u16) {
    bytes[at..at + 2].copy_from_slice(&value.to_le_bytes());
}

fn put_u32(bytes: &mut [u8], at: usize, value: u32) {
    bytes[at..at + 4].copy_from_slice(&value.to_le_bytes());
}

fn put_f32(bytes: &mut [u8], at: usize, value: f32) {
    bytes[at..at + 4].copy_from_slice(&value.to_bits().to_le_bytes());
}

fn put_row(bytes: &mut [u8], at: usize, row: usize, values: [f32; 4]) {
    for (lane, value) in values.iter().enumerate() {
        put_f32(bytes, at + 0x20 + 16 * row + 4 * lane, *value);
    }
}

/// The distance lanes of a head lamp corona: 1 up close, falling to 0.3 at 50 m and 0 at 250 m.
fn distance_block(bytes: &mut [u8], at: usize) {
    put_row(bytes, at, 0, [1.0; 4]);
    put_row(bytes, at, 1, [-2.0, -0.25, -0.25, -0.25]);
    put_row(bytes, at, 2, [-std::f32::consts::FRAC_2_PI, -0.02, -0.02, -0.02]);
    put_row(bytes, at, 3, [1.2732, 0.005, 0.005, 0.005]);
    put_row(bytes, at, 4, [std::f32::consts::FRAC_PI_2, 50.0, 50.0, 50.0]);
    put_row(bytes, at, 5, [-0.5, 0.0, 0.7, 0.7]);
    put_row(bytes, at, 6, [0.5, 1.0, 0.3, 0.3]);
    put_row(bytes, at, 7, [-0.5, 0.0, -0.3, -0.3]);
}

#[test]
fn a_lane_is_a_two_piece_ramp_clamped_to_one() {
    let mut bytes = vec![0u8; 0xB0];
    distance_block(&mut bytes, 0);
    let block = CurveBlock::parse(&bytes, 0).unwrap();
    assert!((block.lane(2, 0.0) - 1.0).abs() < 1e-5, "up close the size lane is 1");
    assert!((block.lane(2, 50.0) - 0.3).abs() < 1e-5, "at the threshold it is the near ramp's end");
    assert!(block.lane(2, 250.0).abs() < 1e-5, "far away it is 0");
    assert!((block.lane(0, std::f32::consts::FRAC_PI_2) - 0.5).abs() < 1e-3, "the angle lane rises to 0.5 side on");
    let all = block.eval([0.0, 10.0, 50.0, 250.0]);
    assert!((all[1] - 1.0).abs() < 1e-6 && (all[2] - 0.3).abs() < 1e-5 && all[3].abs() < 1e-5);
}

/// An object with one glow layer in the glow list (count at 0xD0, slot at 0xBC).
fn corona_with_glow() -> (Vec<u8>, Vec<Reference>) {
    let mut bytes = vec![0u8; 0x1C0];
    distance_block(&mut bytes, 0);
    put_f32(&mut bytes, 0xA4, 99999.0);
    put_f32(&mut bytes, 0xA8, 100.0);
    put_f32(&mut bytes, 0xAC, -0.5);
    put_u32(&mut bytes, 0xBC, 0xE0);
    put_u16(&mut bytes, 0xD0, 1);
    distance_block(&mut bytes, 0xE0);
    for (n, value) in [0.6, 0.8, 1.0, 1.0].iter().enumerate() {
        put_f32(&mut bytes, 0xE0 + 0xA0 + 4 * n, *value);
    }
    put_f32(&mut bytes, 0xE0 + 0xC8, 2.0);
    put_u16(&mut bytes, 0xE0 + 0xD0, 1);
    (bytes, vec![Reference { at: 0xE0 + 0xC0, id: 0x6D11 }])
}

#[test]
fn a_corona_is_read_with_its_layers_and_picture() {
    let (bytes, refs) = corona_with_glow();
    let corona = Corona::parse(&bytes, &refs).unwrap();
    assert_eq!((corona.far_limit, corona.cull_distance, corona.bias), (99999.0, 100.0, -0.5));
    assert!(corona.beams.is_empty() && corona.flares.is_empty());
    assert_eq!(corona.glows.len(), 1);
    let glow = &corona.glows[0];
    assert_eq!(glow.kind, LayerKind::Glow);
    assert_eq!(glow.colour, [0.6, 0.8, 1.0, 1.0]);
    assert_eq!((glow.picture, glow.depth, glow.colour_source), (Some(0x6D11), 2.0, 1));
    assert_eq!(corona.main_view().count(), 1);
    assert!(!corona.is_empty());
}

#[test]
fn a_layer_list_past_the_object_is_refused() {
    let (mut bytes, refs) = corona_with_glow();
    put_u32(&mut bytes, 0xBC, 0x1B0);
    assert!(Corona::parse(&bytes, &refs).is_err());
    put_u16(&mut bytes, 0xD0, 200);
    assert!(Corona::parse(&bytes, &refs).is_err());
}

fn pattern_object(weights: &[(f32, f32, f32)]) -> Vec<u8> {
    let mut bytes = vec![0u8; 0x18 + 0x24 * weights.len()];
    put_u32(&mut bytes, 0x10, 0x18);
    put_u16(&mut bytes, 0x14, weights.len() as u16);
    for (n, (weight, rate, offset)) in weights.iter().enumerate() {
        let at = 0x18 + 0x24 * n;
        put_f32(&mut bytes, at + 0x18, *weight);
        put_f32(&mut bytes, at + 0x1C, *rate);
        put_f32(&mut bytes, at + 0x20, *offset);
    }
    bytes
}

#[test]
fn a_pattern_object_lists_its_segments_and_a_sequence_its_steps() {
    let object = pattern_object(&[(8.0, 0.4, 0.0), (8.0, 0.5, 0.125)]);
    let segments = pattern::segments(&object).unwrap();
    assert_eq!(segments.len(), 2);
    assert_eq!((segments[1].weight, segments[1].rate, segments[1].offset), (8.0, 0.5, 0.125));
    assert_eq!(segments[0].reference_at, 0x18 + 0x0C);
    let mut seq = vec![0u8; 0x18 + 4 * 4];
    put_u32(&mut seq, 0x10, 0x18);
    put_u16(&mut seq, 0x14, 4);
    put_f32(&mut seq, 0x1C, 1.0);
    assert_eq!(pattern::sequence(&seq).unwrap(), vec![0.0, 1.0, 0.0, 0.0]);
}

#[test]
fn a_flash_pattern_plays_its_segments_in_turn() {
    let one = Segment { weight: 1.0, rate: 1.0, offset: 0.0, steps: vec![1.0, 0.0] };
    let two = Segment { weight: 1.0, rate: 1.0, offset: 0.0, steps: vec![0.0, 0.0] };
    let pattern = FlashPattern { segments: vec![one, two] };
    assert_eq!(pattern.cycle(), 2.0);
    assert_eq!(pattern.at(0.25), 1.0, "the first half of the first turn is step 0");
    assert_eq!(pattern.at(0.75), 0.0);
    assert_eq!(pattern.at(1.25), 0.0, "the second segment is dark");
    assert_eq!(pattern.at(2.25), 1.0, "the cycle repeats");
    assert_eq!(pattern.sample(0.5), vec![1.0, 0.0, 0.0, 0.0]);
    assert_eq!(FlashPattern::default().at(1.0), 0.0);
}

/// A brake lamp corona's lanes: visible on the axis, dimmer to the side; bigger and brighter with distance.
fn brake_curves(bytes: &mut [u8], at: usize) {
    put_row(bytes, at, 0, [1.0, 1.0, 1.0, 1.0]);
    put_row(bytes, at, 1, [-0.3333, -0.25, -0.25, -0.25]);
    put_row(bytes, at, 2, [-1.2732, -0.0333, -0.01, -0.01]);
    put_row(bytes, at, 3, [0.4244, 0.00833, 0.0025, 0.0025]);
    put_row(bytes, at, 4, [std::f32::consts::FRAC_PI_4, 30.0, 100.0, 100.0]);
    put_row(bytes, at, 5, [0.2, 2.5, -0.425, -0.425]);
    put_row(bytes, at, 6, [0.8, 1.5, 0.5, 0.5]);
    put_row(bytes, at, 7, [-0.8, -0.5, 0.5, 0.5]);
}

fn corona_with(curves: fn(&mut [u8], usize)) -> Corona {
    let (mut bytes, refs) = corona_with_glow();
    curves(&mut bytes, 0);
    Corona::parse(&bytes, &refs).unwrap()
}

#[test]
fn a_lamp_seen_head_on_has_its_angle_curve_turned_into_a_cosine_power() {
    let brake = corona_with(brake_curves);
    let power = brake.falloff_power().unwrap();
    assert!((power - 0.49).abs() < 0.05, "{power}");
    assert!((brake.lanes(0.0, 10.0)[0] - 1.0).abs() < 1e-4, "full visibility on the axis");
    let beam = corona_with(distance_block);
    assert_eq!(beam.falloff_power(), None, "a lamp invisible on its axis is a beam seen from the side");
}

#[test]
fn the_corona_lanes_feed_the_layer_curves_and_the_record_factors_scale_the_result() {
    let brake = corona_with(brake_curves);
    let glow = brake.glows[0].clone();
    let plain = brake.seen(&glow, &RecordFactors::default(), 0.0, 60.0);
    let doubled =
        brake.seen(&glow, &RecordFactors { size: [2.0, 2.0], intensity: 2.0, ..Default::default() }, 0.0, 60.0);
    assert!(
        (doubled.size[0] - 2.0 * plain.size[0]).abs() < 1e-5 && (doubled.size[1] - 2.0 * plain.size[1]).abs() < 1e-5
    );
    assert!((doubled.colour[1] - 2.0 * plain.colour[1]).abs() < 1e-5);
    assert_eq!(plain.alpha, glow.colour[3]);
    let side = brake.seen(&glow, &RecordFactors::default(), 1.4, 60.0);
    assert!(side.visibility < plain.visibility);
    assert!(brake.lanes(0.0, 120.0)[1] < brake.lanes(0.0, 5.0)[1] + 1.0);
}

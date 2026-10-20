use super::*;
use crate::ice::CAMERA_RECORD_HEADER_LEN;

fn key_bytes(set: impl FnOnce(&mut [u8])) -> Vec<u8> {
    let mut e = vec![0u8; CAMERA_ELEMENT_LEN];
    e[0] = 13; // kind
    e[2] = 1; // cubic eye
    e[3] = 1; // cubic look
    e[4] = 3; // scene space
    e[5] = 3;
    e[0x14..0x18].copy_from_slice(&1.0f32.to_le_bytes());
    e[0x18..0x1C].copy_from_slice(&1.0f32.to_le_bytes());
    for (at, v) in [(0x54usize, 24.0f32), (0x58, 24.0)] {
        e[at..at + 4].copy_from_slice(&v.to_le_bytes());
    }
    e[0x80] = 100;
    e[0x81] = 100;
    set(&mut e);
    e
}

fn put(e: &mut [u8], at: usize, values: &[f32]) {
    for (i, v) in values.iter().enumerate() {
        e[at + 4 * i..at + 4 * i + 4].copy_from_slice(&v.to_le_bytes());
    }
}

fn track(keys: Vec<Vec<u8>>, length: f32) -> Track {
    let mut header = vec![0u8; CAMERA_RECORD_HEADER_LEN];
    header[LENGTH_OFFSET..LENGTH_OFFSET + 4].copy_from_slice(&length.to_le_bytes());
    header[COUNT_OFFSET..COUNT_OFFSET + 2].copy_from_slice(&(keys.len() as i16).to_le_bytes());
    header[NAME_OFFSET..NAME_OFFSET + 7].copy_from_slice(b"Track 0");
    let elements = keys.iter().map(|k| k.as_slice()).collect();
    Track::from_record(&CameraRecord { header: &header, elements }).unwrap()
}

#[test]
fn a_record_reads_its_name_length_and_key_fields() {
    let t = track(vec![key_bytes(|e| put(e, 0x1C, &[1.0, 2.0, 3.0, 4.0, 5.0, 6.0]))], 3.0);
    assert_eq!(t.name, "Track 0");
    assert_eq!(t.length, 3.0);
    let key = &t.keys[0];
    assert_eq!(key.eye, [[1.0, 2.0, 3.0], [4.0, 5.0, 6.0]]);
    assert_eq!((key.space_eye, key.space_look), (Space::Scene, Space::Scene));
    assert!(key.cubic_eye && key.cubic_look);
}

#[test]
fn a_record_with_a_wrong_count_or_space_does_not_read() {
    let mut header = vec![0u8; CAMERA_RECORD_HEADER_LEN];
    header[COUNT_OFFSET] = 2;
    let key = key_bytes(|_| {});
    assert!(Track::from_record(&CameraRecord { header: &header, elements: vec![&key] }).is_none());
    assert!(Key::parse(&key_bytes(|e| e[4] = 9)).is_none());
}

#[test]
fn a_one_key_track_runs_a_straight_line_from_the_start_to_the_end_values() {
    // Tangents equal to the chord make the cubic exactly linear.
    let t = track(vec![key_bytes(|e| put(e, 0x1C, &[0.0, 0.0, 0.0, 10.0, -20.0, 30.0]))], 3.0);
    let start = t.sample(0.0).unwrap();
    let mid = t.sample(0.5).unwrap();
    let end = t.sample(1.0).unwrap();
    assert_eq!(start.eye, [0.0, 0.0, 0.0]);
    for (got, want) in mid.eye.iter().zip([5.0, -10.0, 15.0]) {
        assert!((got - want).abs() < 1e-4, "{got} vs {want}");
    }
    assert_eq!(end.eye, [10.0, -20.0, 30.0]);
}

#[test]
fn shorter_tangents_ease_the_curve_and_keep_the_end_points() {
    let t = track(
        vec![key_bytes(|e| {
            put(e, 0x1C, &[0.0, 0.0, 0.0, 10.0, 0.0, 0.0]);
            put(e, 0x14, &[0.0, 0.0]);
        })],
        1.0,
    );
    assert_eq!(t.sample(0.0).unwrap().eye[0], 0.0);
    assert_eq!(t.sample(1.0).unwrap().eye[0], 10.0);
    // Zero tangents: smoothstep, 0.5 in the middle, slower than a line at 0.25.
    assert!((t.sample(0.5).unwrap().eye[0] - 5.0).abs() < 1e-4);
    assert!(t.sample(0.25).unwrap().eye[0] < 2.5);
}

#[test]
fn a_key_with_the_cubic_flag_off_is_a_line_whatever_its_tangents() {
    let t = track(
        vec![key_bytes(|e| {
            e[2] = 0;
            put(e, 0x1C, &[0.0, 0.0, 0.0, 8.0, 0.0, 0.0]);
            put(e, 0x14, &[0.0, 0.0]);
        })],
        1.0,
    );
    assert!((t.sample(0.25).unwrap().eye[0] - 2.0).abs() < 1e-5);
}

#[test]
fn a_kind_zero_key_has_no_tangents() {
    let t = track(
        vec![key_bytes(|e| {
            e[0] = 0;
            put(e, 0x1C, &[0.0, 0.0, 0.0, 10.0, 0.0, 0.0]);
        })],
        1.0,
    );
    assert!((t.sample(0.25).unwrap().eye[0] - 1.5625).abs() < 1e-4, "smoothstep(0.25) of 10");
}

#[test]
fn times_outside_the_track_have_no_camera() {
    let t = track(vec![key_bytes(|_| {})], 3.0);
    assert!(t.sample(-0.01).is_none());
    assert!(t.sample(1.01).is_none());
    assert!(t.sample(f32::NAN).is_none());
}

#[test]
fn the_interval_is_the_last_key_not_after_the_time() {
    let first = key_bytes(|e| put(e, 0x1C, &[0.0, 0.0, 0.0, 10.0, 0.0, 0.0]));
    let second = key_bytes(|e| {
        put(e, 0x10, &[0.5]);
        put(e, 0x1C, &[100.0, 0.0, 0.0, 200.0, 0.0, 0.0]);
    });
    let t = track(vec![first, second], 2.0);
    assert!(t.sample(0.25).unwrap().eye[0] < 10.0, "inside the first interval");
    assert!((t.sample(0.5).unwrap().eye[0] - 100.0).abs() < 1e-3, "the second key starts at its own time");
    assert!(t.sample(0.75).unwrap().eye[0] > 100.0);
}

#[test]
fn lens_gives_the_field_of_view_and_is_constant_for_a_fixed_lens() {
    let t = track(vec![key_bytes(|_| {})], 3.0);
    let s = t.sample(0.4).unwrap();
    assert_eq!(s.lens, 24.0);
    let degrees = f32::from(s.fov) / 65536.0 * 360.0;
    assert!((degrees - 67.25).abs() < 0.1, "24 mm is about 67 degrees, got {degrees}");
    assert!(fov_of_lens(75.0) < fov_of_lens(24.0), "a longer lens sees a narrower field");
}

#[test]
fn dutch_is_cut_to_a_whole_angle_unit() {
    let tiny = track(vec![key_bytes(|e| put(e, 0x4C, &[0.002, -0.003]))], 1.0);
    assert_eq!(tiny.sample(0.5).unwrap().roll, 0);
    let large = track(vec![key_bytes(|e| put(e, 0x4C, &[100.0, 100.0]))], 1.0);
    assert_eq!(large.sample(0.5).unwrap().roll, 100);
}

#[test]
fn sim_speed_and_letterbox_are_percentages() {
    let slow = track(
        vec![key_bytes(|e| {
            e[0x80] = 60;
            e[0x81] = 60;
            e[0x7E] = 100;
            e[0x7F] = 100;
        })],
        1.0,
    );
    let s = slow.sample(0.3).unwrap();
    assert!((s.sim_speed - 0.6).abs() < 1e-6);
    assert!((s.letterbox - 0.125).abs() < 1e-6);
    let zero = track(
        vec![key_bytes(|e| {
            e[0x80] = 0;
            e[0x81] = 0;
        })],
        1.0,
    );
    assert_eq!(zero.sample(0.3).unwrap().sim_speed, 0.01, "never stops the world");
}

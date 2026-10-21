use crate::bits::{BitError, BitReader, BitWriter, Quant, decode_angle, encode_angle, read_quat, write_quat};
use crate::rng::SplitMix64;
use std::f32::consts::PI;
use std::f64::consts::PI as PI64;
use std::f64::consts::TAU;

/// Distance on the circle between two angles in radians.
fn wrap_distance(a: f64, b: f64) -> f64 {
    let d = (a - b).rem_euclid(TAU);
    d.min(TAU - d)
}

/// Largest per-component difference between `back` and `sign * q`.
fn component_error(back: &[f32; 4], q: &[f32; 4], sign: f32) -> f32 {
    back.iter().zip(q.iter()).map(|(b, c)| (b - sign * c).abs()).fold(0.0, f32::max)
}

/// A random unit quaternion, uniform over the box before normalising.
fn random_unit(rng: &mut SplitMix64) -> [f32; 4] {
    loop {
        let v: [f64; 4] = std::array::from_fn(|_| rng.next_f64() * 2.0 - 1.0);
        let norm = v.iter().map(|c| c * c).sum::<f64>().sqrt();
        if norm > 1e-3 {
            return std::array::from_fn(|i| (v[i] / norm) as f32);
        }
    }
}

#[test]
fn quant_endpoints_clamping_and_step() {
    let q = Quant::new(0.0, 1.0, 4);
    assert_eq!(q.encode(0.0), 0);
    assert_eq!(q.encode(1.0), 15);
    assert_eq!(q.encode(-5.0), 0);
    assert_eq!(q.encode(5.0), 15);
    assert_eq!(q.decode(0), Ok(0.0));
    assert_eq!(q.decode(15), Ok(1.0));
    assert_eq!(q.decode(16), Err(BitError::BadValue));
    assert_eq!(q.bits(), 4);
    assert!((q.step() - 1.0 / 15.0).abs() < 1e-7);
}

#[test]
fn nan_encodes_the_value_nearest_zero_in_range() {
    let q = Quant::new(-1.0, 1.0, 8);
    assert_eq!(q.encode(f32::NAN), q.encode(0.0));
    let q = Quant::new(2.0, 6.0, 8);
    assert_eq!(q.encode(f32::NAN), 0);
    let q = Quant::new(-6.0, -2.0, 8);
    assert_eq!(q.encode(f32::NAN), 255);
}

#[test]
fn quant_error_is_at_most_half_a_step() {
    let mut rng = SplitMix64(31);
    let widths = [1u32, 2, 5, 8, 10, 16, 24, 32];
    for case in 0..10_000 {
        let bits = widths[case % widths.len()];
        let min = (-1000.0 * rng.next_f64()) as f32;
        let span = (0.001 + 2000.0 * rng.next_f64()) as f32;
        let max = min + span;
        let q = Quant::new(min, max, bits);
        let x = min + (rng.next_f64() as f32) * (max - min);
        let decoded = q.decode(q.encode(x)).unwrap();
        let scale = min.abs().max(max.abs());
        let tolerance = q.step() / 2.0 + 1e-6 * scale;
        assert!((decoded - x).abs() <= tolerance, "bits {bits} range {min}..{max}: {x} -> {decoded}");
    }
}

#[test]
fn quant_round_trips_through_a_stream() {
    let q = Quant::new(-5.0, 5.0, 10);
    let mut rng = SplitMix64(41);
    let values: Vec<f32> = (0..10_000).map(|_| (rng.next_f64() * 12.0 - 6.0) as f32).collect();
    let mut buf = vec![0u8; 10_000 * 10 / 8 + 4];
    {
        let mut w = BitWriter::new(&mut buf);
        for x in &values {
            q.write(&mut w, *x).unwrap();
        }
        assert_eq!(w.bits_written(), 10_000 * 10);
    }
    let mut r = BitReader::new(&buf);
    for x in &values {
        let expected = q.decode(q.encode(*x)).unwrap();
        assert_eq!(q.read(&mut r).unwrap(), expected);
    }
}

#[test]
fn angle_landmarks() {
    assert_eq!(encode_angle(0.0, 10), 0);
    assert_eq!(encode_angle(PI, 10), 512);
    assert_eq!(encode_angle(-PI / 2.0, 10), 768);
    assert_eq!(decode_angle(0, 10), 0.0);
    assert!((decode_angle(256, 10) - (PI64 / 2.0) as f32).abs() < 1e-6);
    // Pi itself lands on the wrap point and decodes to -pi, the same angle.
    assert!((decode_angle(512, 10) + PI).abs() < 1e-6);
}

#[test]
fn angle_error_is_at_most_half_a_step_across_the_wrap() {
    let mut rng = SplitMix64(51);
    let widths = [4u32, 8, 10, 12, 16];
    let edges = [PI, -PI, PI - 1e-4, -PI + 1e-4, 3.0 * PI, -3.0 * PI, 0.0];
    for case in 0..10_000 {
        let bits = widths[case % widths.len()];
        let angle = match case < edges.len() {
            true => edges[case],
            false => ((rng.next_f64() * 2.0 - 1.0) * 4.0 * PI64) as f32,
        };
        let q = encode_angle(angle, bits);
        assert!(q < 1u32 << bits);
        let back = decode_angle(q, bits);
        assert!((-PI..=PI).contains(&back), "{back} out of range");
        let error = wrap_distance(f64::from(back), f64::from(angle));
        let tolerance = PI64 / f64::from(1u32 << bits) + 1e-5;
        assert!(error <= tolerance, "bits {bits}: {angle} -> {back}");
    }
}

#[test]
fn quaternions_round_trip_within_0_002_at_10_bits() {
    let mut rng = SplitMix64(61);
    let samples: Vec<[f32; 4]> = (0..10_000).map(|_| random_unit(&mut rng)).collect();
    let mut buf = vec![0u8; 10_000 * 4 + 8];
    {
        let mut w = BitWriter::new(&mut buf);
        for q in &samples {
            write_quat(&mut w, *q, 10).unwrap();
        }
        assert_eq!(w.bits_written(), 10_000 * 32);
    }
    let mut r = BitReader::new(&buf);
    for q in &samples {
        let back = read_quat(&mut r, 10).unwrap();
        // The decoder may return -q: both describe the same rotation.
        let error = component_error(&back, q, 1.0).min(component_error(&back, q, -1.0));
        assert!(error <= 0.002, "{q:?} -> {back:?}");
        let norm = back.iter().map(|c| c * c).sum::<f32>().sqrt();
        assert!((norm - 1.0).abs() < 1e-4, "norm {norm}");
    }
}

#[test]
fn quaternion_edge_cases() {
    // A negative w is stored with its sign flipped; the identity comes back as it went in, up to sign.
    let mut buf = [0u8; 8];
    {
        let mut w = BitWriter::new(&mut buf);
        write_quat(&mut w, [0.0, 0.0, 0.0, -1.0], 10).unwrap();
        assert_eq!(w.bits_written(), 32);
    }
    let back = read_quat(&mut BitReader::new(&buf), 10).unwrap();
    assert!(component_error(&back, &[0.0, 0.0, 0.0, 1.0], 1.0) <= 0.002);

    let mut buf = [0u8; 3];
    let mut w = BitWriter::new(&mut buf);
    assert_eq!(write_quat(&mut w, [0.0, 0.0, 0.0, 1.0], 10), Err(BitError::Overflow));
    assert_eq!(w.bits_written(), 0);
    assert_eq!(write_quat(&mut w, [0.0, 0.0, 0.0, 1.0], 0), Err(BitError::BadValue));
    let short = [0x00u8; 3];
    assert_eq!(read_quat(&mut BitReader::new(&short), 10), Err(BitError::Underflow));
}

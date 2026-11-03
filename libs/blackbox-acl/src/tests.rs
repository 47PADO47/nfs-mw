//! Round trips through the test encoder: what the decoder reads back is what was put in, to the quantisation step.

use crate::encoder::{Spec, encode};
use crate::{AclError, Qvv, Tracks};

fn axis_angle(axis: [f32; 3], angle: f32) -> [f32; 4] {
    let (s, c) = (angle * 0.5).sin_cos();
    let n = (axis[0] * axis[0] + axis[1] * axis[1] + axis[2] * axis[2]).sqrt();
    [axis[0] / n * s, axis[1] / n * s, axis[2] / n * s, c]
}

fn reference(tracks: usize) -> Vec<Qvv> {
    (0..tracks)
        .map(|t| Qvv {
            rotation: axis_angle([1.0, 2.0, 3.0], 0.1 * t as f32),
            translation: [t as f32, 0.5, -1.0],
            scale: [1.0; 3],
        })
        .collect()
}

/// Track 0 rests, track 1 is constant, tracks 2.. move; the last one also scales.
fn motion(tracks: usize, frames: usize) -> Vec<Vec<Qvv>> {
    let base = reference(tracks);
    (0..frames)
        .map(|f| {
            let time = f as f32 / frames as f32;
            (0..tracks)
                .map(|t| match t {
                    0 => base[0],
                    1 => Qvv {
                        rotation: axis_angle([0.0, 1.0, 0.0], 0.7),
                        translation: [1.0, 2.0, 3.0],
                        scale: [1.0; 3],
                    },
                    _ => Qvv {
                        rotation: axis_angle([t as f32, 1.0, 0.5], 0.8 * (time * 5.0 + t as f32).sin()),
                        translation: [time * 10.0 - 3.0, 0.5 * t as f32, (time * 3.0).cos()],
                        scale: if t == tracks - 1 { [1.0 + time, 1.0, 1.0 - 0.5 * time] } else { [1.0; 3] },
                    },
                })
                .collect()
        })
        .collect()
}

fn spec(frames: Vec<Vec<Qvv>>) -> Spec {
    Spec {
        reference: reference(frames[0].len()),
        frames,
        sample_rate: 30.0,
        bits: 14,
        raw_first: false,
        segment_len: 0,
        strip: Vec::new(),
        wrap: false,
        full_precision: false,
    }
}

fn close_q(a: [f32; 4], b: [f32; 4], tolerance: f32) -> bool {
    let sign = if a.iter().zip(&b).map(|(x, y)| x * y).sum::<f32>() < 0.0 { -1.0 } else { 1.0 };
    a.iter().zip(&b).all(|(x, y)| (x - sign * y).abs() <= tolerance)
}

fn assert_pose(got: &[Qvv], want: &[Qvv], tolerance: f32) {
    assert_eq!(got.len(), want.len());
    for (t, (g, w)) in got.iter().zip(want).enumerate() {
        assert!(close_q(g.rotation, w.rotation, tolerance), "track {t} rotation {:?} vs {:?}", g.rotation, w.rotation);
        for k in 0..3 {
            assert!(
                (g.translation[k] - w.translation[k]).abs() <= tolerance * 10.0,
                "track {t} translation {g:?} vs {w:?}"
            );
            assert!((g.scale[k] - w.scale[k]).abs() <= tolerance, "track {t} scale {g:?} vs {w:?}");
        }
    }
}

fn round_trip(spec: &Spec, tolerance: f32) {
    let bytes = encode(spec);
    let tracks = Tracks::parse(&bytes).expect("parses");
    assert_eq!(tracks.num_tracks(), spec.reference.len());
    assert_eq!(tracks.num_frames(), spec.frames.len());
    for (f, want) in spec.frames.iter().enumerate() {
        let got = tracks.decode_frame(f, Some(&spec.reference)).expect("decodes");
        assert_pose(&got, want, tolerance);
    }
}

#[test]
fn one_segment_of_quantised_tracks() {
    let s = spec(motion(7, 21));
    let bytes = encode(&s);
    let tracks = Tracks::parse(&bytes).unwrap();
    assert_eq!(tracks.num_segments(), 1);
    assert!(tracks.has_scale());
    round_trip(&s, 5e-4);
}

#[test]
fn several_segments_expand_two_ranges() {
    let mut s = spec(motion(9, 70));
    s.segment_len = 16;
    s.bits = 13;
    let bytes = encode(&s);
    assert_eq!(Tracks::parse(&bytes).unwrap().num_segments(), 5);
    round_trip(&s, 5e-4);
}

#[test]
fn a_track_stored_as_floats_is_exact() {
    let mut s = spec(motion(6, 18));
    s.raw_first = true;
    round_trip(&s, 5e-4);
    let mut exact = spec(motion(6, 18));
    exact.full_precision = true;
    round_trip(&exact, 1e-6);
}

#[test]
fn a_looping_clip_plays_its_first_pose_again_at_the_end() {
    let mut frames = motion(5, 12);
    frames.push(frames[0].clone());
    let mut s = spec(frames);
    s.wrap = true;
    let bytes = encode(&s);
    let tracks = Tracks::parse(&bytes).unwrap();
    assert!(tracks.is_wrap_optimized());
    assert_eq!(tracks.num_samples(), 12);
    assert_eq!(tracks.num_frames(), 13);
    assert!((tracks.duration() - 12.0 / 30.0).abs() < 1e-6);
    round_trip(&s, 5e-4);
}

#[test]
fn stripped_keyframes_are_rebuilt_by_interpolation() {
    // Linear motion, so the rebuilt poses equal the originals.
    let frames: Vec<Vec<Qvv>> = (0..20)
        .map(|f| {
            (0..4)
                .map(|t| Qvv {
                    rotation: axis_angle([0.0, 0.0, 1.0], 0.02 * f as f32 + t as f32 * 0.1),
                    translation: [f as f32 * 0.5, t as f32, 0.0],
                    scale: [1.0; 3],
                })
                .collect()
        })
        .collect();
    let mut s = spec(frames);
    s.reference = (0..4).map(|_| Qvv { translation: [100.0; 3], ..Qvv::IDENTITY }).collect();
    s.strip = vec![3, 4, 17];
    s.bits = 16;
    let bytes = encode(&s);
    assert!(Tracks::parse(&bytes).unwrap().has_stripped_keyframes());
    round_trip(&s, 5e-4);
}

#[test]
fn stripped_keyframes_across_segments() {
    let frames: Vec<Vec<Qvv>> = (0..40)
        .map(|f| {
            (0..3)
                .map(|t| Qvv {
                    rotation: axis_angle([1.0, 0.0, 0.0], 0.01 * f as f32),
                    translation: [f as f32 * 0.25, t as f32, 1.0],
                    scale: [1.0; 3],
                })
                .collect()
        })
        .collect();
    let mut s = spec(frames);
    s.reference = (0..3).map(|_| Qvv { translation: [100.0; 3], ..Qvv::IDENTITY }).collect();
    s.segment_len = 20;
    // A segment's last sample is always kept: the neighbour search of the sample before it stays in the segment.
    s.strip = vec![5, 6, 18, 25, 26, 27];
    s.bits = 16;
    round_trip(&s, 1e-3);
}

#[test]
fn sampling_between_frames_blends_them() {
    let s = spec(motion(6, 21));
    let bytes = encode(&s);
    let tracks = Tracks::parse(&bytes).unwrap();
    let defaults = Some(s.reference.as_slice());
    let a = tracks.decode_frame(4, defaults).unwrap();
    let b = tracks.decode_frame(5, defaults).unwrap();
    let mid = tracks.sample(4.5 / 30.0, defaults).unwrap();
    for (t, ((x, y), m)) in a.iter().zip(&b).zip(&mid).enumerate() {
        for k in 0..3 {
            let want = (x.translation[k] + y.translation[k]) * 0.5;
            assert!((m.translation[k] - want).abs() < 1e-4, "track {t}");
        }
    }
    let last = tracks.sample(100.0, defaults).unwrap();
    assert_pose(&last, &tracks.decode_frame(20, defaults).unwrap(), 1e-6);
    let first = tracks.sample(-1.0, defaults).unwrap();
    assert_pose(&first, &tracks.decode_frame(0, defaults).unwrap(), 1e-6);
}

#[test]
fn the_reference_pose_is_needed_unless_the_defaults_are_trivial() {
    let s = spec(motion(4, 10));
    let bytes = encode(&s);
    let tracks = Tracks::parse(&bytes).unwrap();
    assert!(!tracks.has_trivial_defaults());
    assert_eq!(tracks.decode_frame(0, None).unwrap_err(), AclError::MissingDefaults(4));
    assert_eq!(tracks.decode_frame(0, Some(&s.reference[..2])).unwrap_err(), AclError::MissingDefaults(4));
}

#[test]
fn damaged_buffers_are_refused() {
    let s = spec(motion(4, 10));
    let bytes = encode(&s);
    assert!(matches!(Tracks::parse(&bytes[..20]), Err(AclError::Truncated(_))));
    let mut bad = bytes.clone();
    bad[8] ^= 0xFF;
    assert!(matches!(Tracks::parse(&bad), Err(AclError::BadTag(_))));
    let mut old = bytes.clone();
    old[12] = 7;
    assert_eq!(Tracks::parse(&old).unwrap_err(), AclError::BadVersion(7));
    let tracks = Tracks::parse(&bytes).unwrap();
    assert!(matches!(tracks.decode_frame(10, Some(&s.reference)), Err(AclError::OutOfRange { .. })));
    // Cut inside the sample data: the headers parse, reading the poses fails instead of panicking.
    let mut short = bytes[..bytes.len() - 60].to_vec();
    let size = short.len() as u32;
    short[0..4].copy_from_slice(&size.to_le_bytes());
    if let Ok(cut) = Tracks::parse(&short) {
        let all = cut.decode_all(Some(&s.reference));
        assert!(all.is_err());
    }
}

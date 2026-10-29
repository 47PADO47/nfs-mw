use std::path::PathBuf;

use super::*;
use crate::rider::raw::{self, JsonBone};

fn skeleton(bones: usize) -> JsonSkeleton {
    let bone = |i: usize| JsonBone {
        name: format!("bone_{i}"),
        parent: i32::try_from(i).map_or(-1, |p| p - 1),
        t: [0.0; 3],
        q: [0.0, 0.0, 0.0, 1.0],
        s: [1.0; 3],
    };
    JsonSkeleton { bones: (0..bones).map(bone).collect(), sockets: Vec::new() }
}

fn meta(additive: &str) -> String {
    format!(
        r#"{{"name":"clip","num_frames":2,"length":0.03,"rate_scale":1.0,"additive":"{additive}","base_name":"",
        "base_frame":0,"root_motion":false,"track_to_bone":[0,1],"notifies":[]}}"#
    )
}

#[test]
fn additive_clips_are_refused_and_garbage_is_an_error() {
    let err = decode_clip(&[], meta("AAT_LocalSpaceBase").as_bytes(), &skeleton(2)).err().unwrap();
    assert!(format!("{err:#}").contains("additive"), "{err:#}");
    assert!(decode_clip(&[], b"not json", &skeleton(2)).is_err());
    let err = decode_clip(&[0; 64], meta("AAT_None").as_bytes(), &skeleton(2)).err().unwrap();
    assert!(format!("{err:#}").contains("ACL"), "{err:#}");
}

/// About the angle in radians between two rotations (the quaternions are normalised first; small angles are exact,
/// `acos` would lose them to rounding).
fn rotation_difference(a: &[f32], b: &[f32]) -> f32 {
    let unit = |q: &[f32]| {
        let length = q.iter().map(|x| x * x).sum::<f32>().sqrt();
        q.iter().map(|x| x / length).collect::<Vec<_>>()
    };
    let (a, b) = (unit(a), unit(b));
    let distance = |sign: f32| a.iter().zip(&b).map(|(x, y)| (x - sign * y).powi(2)).sum::<f32>().sqrt();
    2.0 * distance(1.0).min(distance(-1.0))
}

/// Compares the decoder with the frames CUE4Parse's native ACL library wrote for the same buffers (it leaves the
/// frames it interpolates between stripped keyframes a little short of unit length, hence the normalising; the
/// exact comparison with ACL's own decoder is `blackbox-acl`'s `real_data` test): run with
/// `R6_ACL_DIR` (the `acl/` export folder), `R6_ACL_REF` (an `R6RA` file made from the same sequences by the export
/// tool built with the native library) and `R6_SKELETON` (`skeleton.json`), `cargo test -p ride6-bikes -- --ignored`.
#[test]
#[ignore = "needs the user's RIDE 6 export"]
fn decoded_clips_match_the_native_library() {
    let var = |name: &str| PathBuf::from(std::env::var(name).unwrap_or_else(|_| panic!("set {name}")));
    let skeleton = raw::parse_skeleton(&fs::read(var("R6_SKELETON")).unwrap()).unwrap();
    let reference = raw::parse_anims(&fs::read(var("R6_ACL_REF")).unwrap()).unwrap();
    let dir = var("R6_ACL_DIR");
    let (mut compared, mut worst_angle, mut worst_distance) = (0, 0.0_f32, 0.0_f32);
    for want in &reference.clips {
        let bytes = fs::read(dir.join(format!("{}.acl", want.name))).unwrap();
        let meta = fs::read(dir.join(format!("{}.meta.json", want.name))).unwrap();
        let got = decode_clip(&bytes, &meta, &skeleton).unwrap();
        assert_eq!(got.frames, want.frames, "{}", want.name);
        let mut clip_worst = (0.0_f32, 0, 0);
        for (i, (a, b)) in got.keys.iter().zip(&want.keys).enumerate() {
            let angle = rotation_difference(&a[..4], &b[..4]);
            worst_angle = worst_angle.max(angle);
            if angle > clip_worst.0 {
                clip_worst = (angle, i / skeleton.bones.len(), i % skeleton.bones.len());
            }
            let d = (0..3).map(|k| (a[4 + k] - b[4 + k]).powi(2)).sum::<f32>().sqrt();
            worst_distance = worst_distance.max(d);
        }
        println!("{}: worst rotation {:?} (frame, bone)", want.name, clip_worst);
        for (a, b) in got.scales.iter().zip(&want.scales) {
            assert!((0..3).all(|k| (a[k] - b[k]).abs() < 1e-5), "{} scale", want.name);
        }
        compared += 1;
    }
    println!(
        "{compared} clips, worst rotation difference {worst_angle} rad, worst translation difference {worst_distance} cm"
    );
    assert!(compared > 0);
    assert!(worst_angle < 1e-4, "rotation {worst_angle}");
    assert!(worst_distance < 1e-3, "translation {worst_distance}");
}

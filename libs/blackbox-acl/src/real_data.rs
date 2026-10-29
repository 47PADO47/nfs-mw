//! Checks against buffers made by a real encoder, never committed: `ACL_REAL_DIR` holds `<name>.acl` and `<name>.txt`,
//! the frames ACL's own C++ decoder wrote for it (`ACL_REAL_DIR` is read by the ignored test only):
//!
//! ```text
//! <tracks> <samples> <rate> <wrap 0|1>
//! <frame> <track> <qx qy qz qw> <tx ty tz> <sx sy sz>        one line per frame and track
//! ```
//!
//! Default sub-tracks were written by the reference as rotation (0.5, 0.5, 0.5, 0.5), translation 7 and scale 1.

use std::fs;
use std::path::PathBuf;

use crate::{Qvv, Tracks};

const MARKER: Qvv = Qvv { rotation: [0.5; 4], translation: [7.0; 3], scale: [1.0; 3] };

fn numbers(line: &str) -> Vec<f32> {
    line.split_whitespace().map(|v| v.parse().unwrap()).collect()
}

fn max_difference(a: &[f32], b: &[f32]) -> f32 {
    a.iter().zip(b).map(|(x, y)| (x - y).abs()).fold(0.0, f32::max)
}

#[test]
#[ignore = "needs ACL_REAL_DIR with real clips and the reference decoder's output"]
fn every_clip_matches_the_reference_decoder() {
    let dir = PathBuf::from(std::env::var("ACL_REAL_DIR").expect("set ACL_REAL_DIR"));
    let mut names: Vec<_> = fs::read_dir(&dir)
        .unwrap()
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.extension().is_some_and(|x| x == "acl"))
        .collect();
    names.sort();
    let (mut clips, mut frames_total, mut worst) = (0, 0, 0.0_f32);
    let mut layouts = std::collections::BTreeSet::new();
    for path in names {
        let bytes = fs::read(&path).unwrap();
        let text = fs::read_to_string(path.with_extension("txt")).unwrap();
        let mut lines = text.lines();
        let header = numbers(lines.next().unwrap());
        let tracks = Tracks::parse(&bytes).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
        assert_eq!(tracks.num_tracks(), header[0] as usize, "{}", path.display());
        assert_eq!(tracks.num_samples(), header[1] as usize, "{}", path.display());
        assert_eq!(tracks.is_wrap_optimized(), header[3] != 0.0, "{}", path.display());
        let defaults = vec![MARKER; tracks.num_tracks()];
        let decoded = tracks.decode_all(Some(&defaults)).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
        for line in lines {
            let v = numbers(line);
            let (frame, track) = (v[0] as usize, v[1] as usize);
            let got = decoded[frame][track];
            let mut rotation = got.rotation;
            // A quaternion and its negation are the same rotation.
            if rotation.iter().zip(&v[2..6]).map(|(a, b)| a * b).sum::<f32>() < 0.0 {
                rotation = rotation.map(|c| -c);
            }
            let error = max_difference(&rotation, &v[2..6])
                .max(max_difference(&got.translation, &v[6..9]))
                .max(max_difference(&got.scale, &v[9..12]));
            assert!(error < 2e-4, "{} frame {frame} track {track}: {got:?} vs {v:?}", path.display());
            worst = worst.max(error);
            frames_total += 1;
        }
        layouts.insert(format!(
            "{:?}/{:?}/{:?} segments {} stripped {} scale {}",
            tracks.rotation_format(),
            tracks.translation_format(),
            tracks.scale_format(),
            tracks.num_segments(),
            tracks.has_stripped_keyframes(),
            tracks.has_scale()
        ));
        clips += 1;
    }
    println!("{clips} clips, {frames_total} track samples, worst difference {worst}");
    for layout in &layouts {
        println!("  {layout}");
    }
    assert!(clips > 0);
}

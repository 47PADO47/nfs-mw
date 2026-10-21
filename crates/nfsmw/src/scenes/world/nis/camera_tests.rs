use super::*;
use blackbox_chunk::ice::{Key, Space};

/// An anchor on the ground at `origin` with no turn: the scene's forward axis (+y) is the render +y axis.
fn anchor(origin: Vec3) -> Anchor {
    Anchor { origin, turn: 0 }
}

/// A one-key track whose eye runs in a straight line from x = 0 to x = 10 (scene space).
fn line_track() -> Track {
    let key = Key {
        kind: 13,
        smooth: false,
        cubic_eye: false,
        cubic_look: false,
        space_eye: Space::Scene,
        space_look: Space::Scene,
        time: 0.0,
        tangent_length: [1.0, 1.0],
        eye: [[0.0, 0.0, 0.0], [10.0, 0.0, 0.0]],
        look: [[0.0, 3.0, 0.0]; 2],
        dutch: [0.0; 2],
        lens: [24.0; 2],
        near_clip: [0.5; 2],
        aperture: [37; 2],
        letterbox: [0; 2],
        sim_speed: [100; 2],
    };
    Track { name: "Track 0".into(), start: 0.0, length: 3.0, keys: vec![key] }
}

#[test]
fn the_track_sample_runs_over_the_track_length_and_stops_at_its_end() {
    let track = line_track();
    let middle = track_sample(&track, 1.5).expect("a sample");
    assert!((middle.eye[0] - 5.0).abs() < 1e-5, "half way along the track");
    let past = track_sample(&track, 99.0).expect("the end is still a sample");
    assert!((past.eye[0] - 10.0).abs() < 1e-5, "time past the end holds the last sample");
}

#[test]
fn the_orbit_keeps_its_distance_and_its_height_and_looks_at_the_anchor() {
    let origin = Vec3::new(10.0, 20.0, 5.0);
    for fraction in [0.0, 0.5, 1.0] {
        let lens = orbit(&anchor(origin), fraction, 16.0 / 9.0);
        let from = lens.position - origin;
        assert!((from.truncate().length() - 12.0).abs() < 1e-3, "{fraction}: {from:?}");
        assert!((from.z - 3.0).abs() < 1e-4, "{fraction}: eye 3 m above the ground");
        let toward = (origin + Vec3::Z * 1.5 - lens.position).normalize();
        assert!(lens.forward.dot(toward) > 0.999, "{fraction}: looks at the anchor");
    }
}

#[test]
fn the_orbit_starts_behind_the_scene_and_sweeps_across_it() {
    // The scene faces +y, so its back is -y. The eye starts 45 degrees to one side of that and ends 45 degrees to the
    // other, so x runs from negative to positive and y stays behind.
    let origin = Vec3::ZERO;
    let start = orbit(&anchor(origin), 0.0, 1.0).position;
    let end = orbit(&anchor(origin), 1.0, 1.0).position;
    assert!(start.x < -8.0 && end.x > 8.0, "start {start:?}, end {end:?}");
    assert!(start.y < 0.0 && end.y < 0.0, "both behind the scene");
}

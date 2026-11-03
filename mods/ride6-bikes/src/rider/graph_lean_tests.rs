//! The lean clips are sweeps evaluated at `1 -/+ lean` seconds (`docs/specs/ride6-rider-knee-foot.md`).

use blackbox_skeleton::{Bone, Channel, Clip, EvalContext, Pack, Skeleton, Track, Transform};
use glam::Vec3;

use super::*;

/// A rider of two bones: the pelvis and the right knee, in a pack of every clip of every type. Each lean clip is a sweep over its 2
/// seconds: the pelvis x goes `-0.15, 0, +0.15` and the right knee x `-0.38, -0.25, -0.12` (frame 0 is the rider hung off to the
/// right with the knee out, the last frame hung off to the left), the other clips hold their own number as the pelvis x.
fn pack_with_sweeps() -> Pack {
    let mut names: Vec<&str> = graph_sets::SETS.iter().flat_map(|set| graph_sets::clip_names(set)).collect();
    names.sort_unstable();
    names.dedup();
    let bones = vec![
        Bone { name: "pelvis".into(), parent: None, rest: Transform::IDENTITY },
        Bone { name: "calf_r".into(), parent: None, rest: Transform::IDENTITY },
    ];
    let track = |bone: usize, values: Vec<Vec3>| Track {
        bone,
        rotation: Channel::None,
        translation: Channel::Uniform(values),
        scale: Channel::None,
    };
    let clips = names
        .iter()
        .enumerate()
        .map(|(i, name)| {
            let sweeping = name.contains("Rossi_standard") || name.contains("Leaning_Knee_LowSpeed");
            let tracks = match sweeping {
                true => vec![
                    track(0, [-0.15, 0.0, 0.15].map(|x| Vec3::new(x, 0.0, 0.0)).to_vec()),
                    track(1, [-0.38, -0.25, -0.12].map(|x| Vec3::new(x, 0.0, 0.0)).to_vec()),
                ],
                false => vec![
                    track(0, vec![Vec3::new(100.0 + i as f32, 0.0, 0.0); 2]),
                    track(1, vec![Vec3::new(-0.25, 0.0, 0.0); 2]),
                ],
            };
            Clip {
                name: (*name).to_owned(),
                length: 2.0,
                tracks,
                additive: name.contains("Shake"),
                root_motion: false,
                notifies: Vec::new(),
            }
        })
        .collect();
    Pack { skeleton: Skeleton::new(bones).unwrap(), sockets: Vec::new(), clips, meshes: Vec::new() }
}

/// The (pelvis x, right knee x) of the graph's pose at `set` (speed 0, 100 km/h of nothing else).
fn pose(pack: &Pack, graph: &Graph, set: &[(&str, f32)]) -> (f32, f32) {
    let mut params = vec![0.0; graph.param_names.len()];
    for (name, value) in set {
        params[graph.param_names.iter().position(|n| n == name).unwrap()] = *value;
    }
    let ctx = EvalContext { skeleton: &pack.skeleton, clips: &pack.clips, params: &params, time: 0.0 };
    let local = graph.root.evaluate(&ctx).local;
    (local[0].translation.x, local[1].translation.x)
}

#[test]
fn the_lean_clip_is_evaluated_at_the_explicit_time_the_blueprint_gives_it() {
    let pack = pack_with_sweeps();
    let graph = graph_for(&pack, &graph_sets::RACE, 0).unwrap();
    // The left clip is evaluated at `1 - lean` seconds, from the right lean at 0 to the left lean at 2.
    let left = |lean: f32| pose(&pack, &graph, &[("lean", lean), ("straight", 0.0)]).0;
    // The pelvis goes from -0.15 (hung off to the right) at lean +1 through 0 (upright) at 0 to +0.15 at lean -1.
    assert!((left(1.0) + 0.15).abs() < 1e-5, "{}", left(1.0));
    assert!(left(0.0).abs() < 1e-5);
    assert!((left(-1.0) - 0.15).abs() < 1e-5);
    assert!((left(0.5) + 0.075).abs() < 1e-5, "halfway is half the sweep");
    // The right clip at `lean + 1` seconds runs the other way.
    let right = |lean: f32| pose(&pack, &graph, &[("lean", lean), ("lean_out", 1.0), ("straight", 0.0)]).0;
    assert!((right(-1.0) + 0.15).abs() < 1e-5);
    assert!(right(0.0).abs() < 1e-5);
    assert!((right(1.0) - 0.15).abs() < 1e-5);
}

#[test]
fn the_two_clips_cross_by_the_left_or_right_signal_and_the_upright_pose_by_the_straight_one() {
    let pack = pack_with_sweeps();
    let graph = graph_for(&pack, &graph_sets::RACE, 0).unwrap();
    let both = pose(&pack, &graph, &[("lean", 0.5), ("lean_out", 0.5), ("straight", 0.0)]).0;
    let (left, right) = (-0.075, 0.075);
    assert!((both - (left + right) / 2.0).abs() < 1e-5);
    let upright = clip_pelvis(&pack, "race_standard_pose");
    assert_eq!(pose(&pack, &graph, &[("lean", 0.5), ("straight", 1.0)]).0, upright);
    let half = pose(&pack, &graph, &[("lean", 0.5), ("straight", 0.5)]).0;
    assert!((half - (upright - 0.075) / 2.0).abs() < 1e-3, "{half}");
}

fn clip_pelvis(pack: &Pack, name: &str) -> f32 {
    100.0 + pack.clip_index(name).unwrap() as f32
}

/// The inner knee is further out than upright from the lean at which a road bike's rider hangs off, and comes back as the bike
/// straightens: the right knee x (the right is -x) is -0.25 upright and -0.38 at full right lean.
#[test]
fn a_road_bike_puts_the_inner_knee_out_in_a_hard_lean_and_takes_it_back() {
    let pack = pack_with_sweeps();
    let graph = graph_for(&pack, &graph_sets::RACE, 0).unwrap();
    let knee = |lean: f32| pose(&pack, &graph, &[("lean", lean), ("straight", 0.0)]).1;
    assert!((knee(0.0) + 0.25).abs() < 1e-5, "upright: the knee is in");
    // A 50 percent lean already has the knee 6.5 cm out; every step of the lean takes it further.
    assert!(knee(0.5) < knee(0.0) - 0.06, "{} {}", knee(0.5), knee(0.0));
    assert!(knee(1.0) < knee(0.75) && knee(0.75) < knee(0.5) && knee(0.5) < knee(0.25) && knee(0.25) < knee(0.0));
    assert!((knee(1.0) + 0.38).abs() < 1e-5);
    // And back: the knee comes in as the bike straightens.
    assert!(knee(0.1) > knee(0.5));
}

#[test]
fn the_supermoto_sweep_is_the_knee_set_of_the_first_riding_style() {
    let pack = pack_with_sweeps();
    let graph = graph_for(&pack, &graph_sets::MOTARD, 0).unwrap();
    let hung = pose(&pack, &graph, &[("lean", 1.0), ("straight", 0.0)]);
    assert!((hung.0 + 0.15).abs() < 1e-5 && (hung.1 + 0.38).abs() < 1e-5, "{hung:?}");
}

#[test]
fn a_lean_clip_that_is_not_two_seconds_long_is_held_at_its_last_frame() {
    let mut pack = pack_with_sweeps();
    let index = pack.clip_index("race_Rossi_standard_Left").unwrap();
    pack.clips[index].length = 1.0;
    let graph_short = graph_for(&pack, &graph_sets::RACE, 0).unwrap();
    // One second of clip: lean 0 is its last frame (1 s), and a lean to the left is held there.
    let at = |lean: f32| pose(&pack, &graph_short, &[("lean", lean), ("straight", 0.0)]).0;
    assert_eq!(at(0.0), at(-1.0));
}

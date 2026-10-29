use blackbox_skeleton::{Bone, Channel, Clip, EvalContext, Pack, Skeleton, Track, Transform};
use glam::Vec3;

use super::*;

/// Every clip the graph names, each holding its own number as the x of the one bone's translation.
const ALL: [&str; 29] = [
    "motard_Leaning_Knee_LowSpeed_L",
    "motard_Leaning_Knee_LowSpeed_R",
    "motard_Leaning_Knee_HighSpeed_L",
    "motard_Leaning_Knee_HighSpeed_R",
    "motard_Leaning_LowSpeed_L",
    "motard_Leaning_LowSpeed_R",
    "motard_Leaning_HighSpeed_L",
    "motard_Leaning_HighSpeed_R",
    "motard_Old_School_standard_L",
    "motard_Old_School_standard_R",
    "motard_Old_School_aero_L",
    "motard_Old_School_aero_R",
    "motard_CenterPose_LowSpeed",
    "motard_CenterPose_HighSpeed",
    "motard_wheelie_min_L",
    "motard_wheelie_max_L",
    "motard_wheelie_min_R",
    "motard_wheelie_max_R",
    "motard_stoppie",
    "motard_Shake_LowSpeed",
    "motard_Shake_HighSpeed",
    "motard_aero_pose",
    "motard_old_school_standard_pose",
    "motard_wheelie_Standing_anim",
    "motard_wheelie_Standing_anim_L",
    "motard_wheelie_Standing_anim_R",
    "motard_wheelie_StandingInterpolation_anim",
    "motard_drift_pose_L",
    "motard_drift_pose_R",
];
/// The clips of `ALL` the ACL decoder provides.
const ACL_ONLY: [&str; 7] = [
    "motard_old_school_standard_pose",
    "motard_wheelie_Standing_anim",
    "motard_wheelie_Standing_anim_L",
    "motard_wheelie_Standing_anim_R",
    "motard_wheelie_StandingInterpolation_anim",
    "motard_drift_pose_L",
    "motard_drift_pose_R",
];

fn pack(without: &[&str]) -> Pack {
    let bone = Bone { name: "root".into(), parent: None, rest: Transform::IDENTITY };
    let clips = ALL
        .iter()
        .filter(|name| !without.contains(name))
        .enumerate()
        .map(|(i, name)| Clip {
            name: (*name).to_owned(),
            length: 1.0,
            tracks: vec![Track {
                bone: 0,
                rotation: Channel::None,
                translation: Channel::Constant(Vec3::new(100.0 + i as f32, 0.0, 0.0)),
                scale: Channel::None,
            }],
            additive: name.starts_with("motard_Shake"),
            root_motion: false,
            notifies: Vec::new(),
        })
        .collect();
    Pack { skeleton: Skeleton::new(vec![bone]).unwrap(), sockets: Vec::new(), clips, meshes: Vec::new() }
}

/// The x of the bone after evaluating the graph with `set` parameters changed from 0 (speed 0: no shake).
fn x(pack: &Pack, graph: &Graph, set: &[(&str, f32)]) -> f32 {
    let mut params = vec![0.0; graph.param_names.len()];
    for (name, value) in set {
        params[graph.param_names.iter().position(|n| n == name).unwrap()] = *value;
    }
    let ctx = EvalContext { skeleton: &pack.skeleton, clips: &pack.clips, params: &params, time: 0.0 };
    graph.root.evaluate(&ctx).local[0].translation.x
}

fn clip_x(pack: &Pack, name: &str) -> f32 {
    100.0 + pack.clip_index(name).unwrap() as f32
}

#[test]
fn the_standing_wheelie_blends_its_space_over_the_sitting_pose() {
    let pack = pack(&[]);
    let graph = motard_graph(&pack, 1).unwrap();
    graph.validate(&pack.clips, &pack.skeleton).unwrap();
    let standing = |steer: f32, entry: f32, alpha: f32| {
        x(&pack, &graph, &[("standing", alpha), ("steer", steer), ("standing_entry", entry)])
    };
    let sitting = x(&pack, &graph, &[]);
    assert_eq!(sitting, clip_x(&pack, "motard_CenterPose_LowSpeed"));
    assert_eq!(standing(0.0, 0.3, 0.0), sitting, "no standing weight: the sitting pose");
    assert_eq!(standing(0.0, 0.3, 1.0), clip_x(&pack, "motard_wheelie_Standing_anim"));
    assert_eq!(standing(-1.0, 0.3, 1.0), clip_x(&pack, "motard_wheelie_Standing_anim_L"));
    assert_eq!(standing(1.0, 0.3, 1.0), clip_x(&pack, "motard_wheelie_Standing_anim_R"));
    assert_eq!(standing(0.0, 0.0, 1.0), clip_x(&pack, "motard_wheelie_StandingInterpolation_anim"));
    // Halfway from the interpolation pose to the standing pose, and half the weight over the sitting one.
    let half = (clip_x(&pack, "motard_wheelie_Standing_anim")
        + clip_x(&pack, "motard_wheelie_StandingInterpolation_anim"))
        / 2.0;
    assert!((standing(0.0, 0.15, 1.0) - half).abs() < 1e-3);
    assert!((standing(0.0, 0.3, 0.5) - (sitting + clip_x(&pack, "motard_wheelie_Standing_anim")) / 2.0).abs() < 1e-3);
}

#[test]
fn the_drift_poses_take_over_on_either_side() {
    let pack = pack(&[]);
    let graph = motard_graph(&pack, 1).unwrap();
    let sitting = x(&pack, &graph, &[]);
    assert_eq!(x(&pack, &graph, &[("drift", -1.0)]), clip_x(&pack, "motard_drift_pose_L"));
    assert_eq!(x(&pack, &graph, &[("drift", 1.0)]), clip_x(&pack, "motard_drift_pose_R"));
    let half = x(&pack, &graph, &[("drift", 0.5)]);
    assert!((half - (sitting + clip_x(&pack, "motard_drift_pose_R")) / 2.0).abs() < 1e-3);
}

#[test]
fn the_old_school_style_centres_on_its_own_pose() {
    let pack = pack(&[]);
    let graph = motard_graph(&pack, 2).unwrap();
    assert_eq!(x(&pack, &graph, &[]), clip_x(&pack, "motard_old_school_standard_pose"));
    assert_eq!(x(&pack, &graph, &[("lean", -1.0)]), clip_x(&pack, "motard_Old_School_standard_L"));
    // Fully tucked: the aero centre, and the aero lean clip when leaning.
    assert_eq!(x(&pack, &graph, &[("tuck", 1.0)]), clip_x(&pack, "motard_aero_pose"));
    assert_eq!(x(&pack, &graph, &[("tuck", 1.0), ("lean", 1.0)]), clip_x(&pack, "motard_Old_School_aero_R"));
    // The other styles keep the centre poses of the leaning folder.
    let plain = motard_graph(&pack, 0).unwrap();
    assert_eq!(x(&pack, &plain, &[]), clip_x(&pack, "motard_CenterPose_LowSpeed"));
    assert_eq!(x(&pack, &plain, &[("lean", 1.0)]), clip_x(&pack, "motard_Leaning_Knee_LowSpeed_R"));
}

#[test]
fn a_pack_without_the_acl_clips_keeps_the_older_tree() {
    let pack = pack(&ACL_ONLY);
    let graph = motard_graph(&pack, 2).unwrap();
    graph.validate(&pack.clips, &pack.skeleton).unwrap();
    // The new parameters exist and do nothing; the old-school style falls back to the shared centre pose.
    assert_eq!(x(&pack, &graph, &[("drift", 1.0), ("standing", 1.0)]), clip_x(&pack, "motard_CenterPose_LowSpeed"));
}

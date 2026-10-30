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

/// A pack with every clip of every bike type, each holding its own number as the x of the one bone's translation.
fn pack_of_all_sets() -> Pack {
    let mut names: Vec<&str> = graph_sets::SETS.iter().flat_map(|set| graph_sets::clip_names(set)).collect();
    names.sort_unstable();
    names.dedup();
    let bone = Bone { name: "root".into(), parent: None, rest: Transform::IDENTITY };
    let clips = names
        .iter()
        .enumerate()
        .map(|(i, name)| Clip {
            name: (*name).to_owned(),
            length: 2.0,
            tracks: vec![Track {
                bone: 0,
                rotation: Channel::None,
                translation: Channel::Constant(Vec3::new(100.0 + i as f32, 0.0, 0.0)),
                scale: Channel::None,
            }],
            additive: name.contains("Shake"),
            root_motion: false,
            notifies: Vec::new(),
        })
        .collect();
    Pack { skeleton: Skeleton::new(vec![bone]).unwrap(), sockets: Vec::new(), clips, meshes: Vec::new() }
}

#[test]
fn every_bike_type_has_a_graph_that_leans_with_its_own_clips() {
    let pack = pack_of_all_sets();
    for set in graph_sets::SETS {
        let graph = graph_for(&pack, set, 0).unwrap();
        graph.validate(&pack.clips, &pack.skeleton).unwrap();
        assert_eq!(graph.param_names, PARAMS.map(str::to_owned), "{}", set.name);
        let lean = &set.lean[0];
        assert_eq!(x(&pack, &graph, &[]), clip_x(&pack, set.centre[0].low), "{}: centred", set.name);
        assert_eq!(x(&pack, &graph, &[("lean", -1.0)]), clip_x(&pack, lean.low[0]), "{}: left", set.name);
        assert_eq!(x(&pack, &graph, &[("lean", 1.0)]), clip_x(&pack, lean.low[1]), "{}: right", set.name);
        assert_eq!(x(&pack, &graph, &[("tuck", 1.0)]), clip_x(&pack, set.centre[0].high), "{}: tucked", set.name);
        assert_eq!(x(&pack, &graph, &[("tuck", 1.0), ("lean", 1.0)]), clip_x(&pack, lean.high[1]), "{}", set.name);
        assert_eq!(x(&pack, &graph, &[("stoppie", 1.0)]), clip_x(&pack, set.stoppie), "{}: stoppie", set.name);
    }
}

#[test]
fn the_sport_bike_leans_in_the_racing_clips_and_the_supermoto_in_its_own() {
    let pack = pack_of_all_sets();
    let race = graph_for(&pack, &graph_sets::RACE, 0).unwrap();
    let motard = graph_for(&pack, &graph_sets::MOTARD, 0).unwrap();
    assert_eq!(x(&pack, &race, &[("lean", 1.0)]), clip_x(&pack, "race_Rossi_standard_Right"));
    assert_eq!(x(&pack, &motard, &[("lean", 1.0)]), clip_x(&pack, "motard_Leaning_Knee_LowSpeed_R"));
    // The riding styles of a type pick their own clips; a style number past the last is the last.
    let schwantz = graph_for(&pack, &graph_sets::RACE, 2).unwrap();
    assert_eq!(x(&pack, &schwantz, &[("lean", -1.0)]), clip_x(&pack, "race_Schwantz_standard_Left"));
    let beyond = graph_for(&pack, &graph_sets::RACE, 9).unwrap();
    assert_eq!(x(&pack, &beyond, &[("lean", -1.0)]), clip_x(&pack, "race_Schwantz_standard_Left"));
}

#[test]
fn only_the_types_that_have_the_clips_get_a_drift_and_a_standing_wheelie() {
    let pack = pack_of_all_sets();
    let drift = |set: &ClipSet| x(&pack, &graph_for(&pack, set, 0).unwrap(), &[("drift", 1.0)]);
    assert_eq!(drift(&graph_sets::SCOOTER), clip_x(&pack, "rider_scooter_drift_pose_R"));
    assert_eq!(drift(&graph_sets::ENDURO), clip_x(&pack, "enduro_drift_pose_R"));
    // Race: no drift pose, the centre pose stays.
    assert_eq!(drift(&graph_sets::RACE), clip_x(&pack, "race_standard_pose"));
    let standing =
        |set: &ClipSet| x(&pack, &graph_for(&pack, set, 0).unwrap(), &[("standing", 1.0), ("standing_entry", 0.3)]);
    assert_eq!(standing(&graph_sets::NAKED), clip_x(&pack, "naked_wheelie_Standing_anim"));
    assert_eq!(
        standing(&graph_sets::SCOOTER),
        clip_x(&pack, "rider_scooter_standard_pose"),
        "a scooter does not stand"
    );
}

#[test]
fn the_walk_back_plays_the_start_clip_then_the_loop() {
    let pack = pack_of_all_sets();
    let graph = graph_for(&pack, &graph_sets::RACE, 0).unwrap();
    let riding = x(&pack, &graph, &[]);
    assert_eq!(x(&pack, &graph, &[("reverse", 0.0), ("reverse_loop", 1.0)]), riding, "no walk, no change");
    // The pose is the start clip until the loop takes over, then the loop clip, once the walk has taken the pose.
    assert_eq!(x(&pack, &graph, &[("reverse", 1.0)]), clip_x(&pack, "race_retro"));
    assert_eq!(x(&pack, &graph, &[("reverse", 1.0), ("reverse_loop", 1.0)]), clip_x(&pack, "race_retro_loop"));
    let half = x(&pack, &graph, &[("reverse", 0.5)]);
    assert!((half - (riding + clip_x(&pack, "race_retro")) / 2.0).abs() < 1e-3);
}

#[test]
fn a_pack_missing_a_types_clips_cannot_build_its_graph_but_a_missing_walk_only_drops_the_walk() {
    let pack = pack(&[]);
    assert!(graph_for(&pack, &graph_sets::RACE, 0).is_err(), "the motard-only pack has no race clips");
    let walkless = graph_for(&pack, &graph_sets::MOTARD, 0).unwrap();
    assert_eq!(x(&pack, &walkless, &[("reverse", 1.0)]), x(&pack, &walkless, &[]));
}

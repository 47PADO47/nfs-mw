use serde_json::{Value, json};

use super::*;

/// A physics asset of a pelvis and a thigh (one joint), in Unreal's centimetres.
fn asset() -> Value {
    json!([
        {"Type": "PhysicsAsset", "Name": "RiderGP_PhysicsAsset", "Properties": {}},
        {"Type": "SkeletalBodySetup", "Name": "SkeletalBodySetup_0", "Properties": {
            "BoneName": "pelvis", "DefaultInstance": {"MassInKgOverride": 7.0},
            "AggGeom": {"SphylElems": [{"Center": {"X": 1.0, "Y": 2.0, "Z": 3.0},
                "Rotation": {"Pitch": 0.0, "Yaw": -90.0, "Roll": 0.0}, "Radius": 11.0, "Length": 15.0}]}}},
        {"Type": "SkeletalBodySetup", "Name": "SkeletalBodySetup_1", "Properties": {
            "BoneName": "thigh_l", "DefaultInstance": {"MassInKgOverride": 7.0},
            "AggGeom": {"SphylElems": [{"Center": {"X": -25.0, "Y": 0.0, "Z": 0.0},
                "Rotation": {"Pitch": 0.0, "Yaw": 0.0, "Roll": 90.0}, "Radius": 7.0, "Length": 25.0}]}}},
        {"Type": "PhysicsConstraintTemplate", "Name": "PhysicsConstraintTemplate_0", "Properties": {"DefaultInstance": {
            "ConstraintBone1": "thigh_l", "ConstraintBone2": "pelvis",
            "PriAxis2": {"X": 0.0, "Y": 1.0, "Z": 0.0}, "SecAxis2": {"X": -1.0, "Y": 0.0, "Z": 0.0},
            "ProfileInstance": {
                "ConeLimit": {"Swing1LimitDegrees": 50.0, "Swing1Motion": "EAngularConstraintMotion::ACM_Limited",
                              "Swing2Motion": "EAngularConstraintMotion::ACM_Locked"},
                "TwistLimit": {"TwistMotion": "EAngularConstraintMotion::ACM_Free"}}}}}
    ])
}

fn blueprint() -> Value {
    json!([
        {"Type": "IgnitionSkeletalMeshComponent", "Name": "Rider", "Properties": {}},
        {"Type": "IgnitionRagdollComponent", "Name": "IgnitionRagdoll_GEN_VARIABLE", "Properties": {"StartFallConfigurations": [
            {"Key": "ERiderFallTypeEnum::VE_HighSide", "Value": {
                "FallAdditionalVelocity": {"X": 0.0, "Y": 0.0, "Z": 6.0},
                "FallVelocityMultiplierAdditional": {"X": 0.05, "Y": 0.0, "Z": -0.28},
                "FallAdditionalVelocityWorld": {"X": 0.0, "Y": 3.6, "Z": 0.0}, "FallAngularVelocityFactor": 0.5}},
            {"Key": "ERiderFallTypeEnum::VE_LowSide", "Value": {
                "FallAdditionalVelocity": {"X": 0.0, "Y": 0.0, "Z": 0.0},
                "FallVelocityMultiplierAdditional": {"X": 0.05, "Y": 0.0, "Z": 0.32},
                "FallAdditionalVelocityWorld": {"X": 0.0, "Y": 0.0, "Z": 0.0}, "FallAngularVelocityFactor": 0.5}}]}}
    ])
}

#[test]
fn a_body_is_a_capsule_on_a_bone_in_metres_and_the_riders_frame() {
    let bodies = asset::bodies(&asset()).unwrap();
    assert_eq!(bodies.len(), 2);
    let pelvis = &bodies[0];
    assert_eq!((pelvis.bone.as_str(), pelvis.mass), ("pelvis", 7.0));
    assert!((pelvis.radius - 0.11).abs() < 1e-6 && (pelvis.half_length - 0.075).abs() < 1e-6);
    // The centre is (1, 2, 3) cm in Unreal's frame: y is mirrored.
    assert!(pelvis.centre.distance(glam::Vec3::new(0.01, -0.02, 0.03)) < 1e-6);
    // A capsule's axis is its Z; a yaw leaves it there, a roll of 90 degrees lays it along the bone's y.
    assert!(pelvis.axis.distance(glam::Vec3::Z) < 1e-5, "{:?}", pelvis.axis);
    assert!(bodies[1].axis.y.abs() > 0.9999, "{:?}", bodies[1].axis);
    assert!(asset::bodies(&json!([])).is_err());
    let massless = json!([{"Type": "SkeletalBodySetup", "Properties": {"BoneName": "pelvis", "DefaultInstance": {}}}]);
    assert!(asset::bodies(&massless).unwrap_err().to_string().contains("mass"));
}

#[test]
fn a_joint_has_its_frames_and_its_limits_by_motion() {
    let bodies = asset::bodies(&asset()).unwrap();
    let joints = asset::joints(&asset(), &bodies).unwrap();
    assert_eq!(joints.len(), 1);
    let joint = &joints[0];
    assert_eq!((joint.child.as_str(), joint.parent.as_str()), ("thigh_l", "pelvis"));
    // Limited 50, locked, free.
    assert_eq!((joint.swing_z, joint.swing_y, joint.twist), (50.0, 0.0, 180.0));
    // The child's frame is the default one (the identity); the parent's has its twist axis along Unreal's y, the rider's -y.
    assert!(joint.frame_child.dot(glam::Quat::IDENTITY).abs() > 0.9999);
    let twist_axis = joint.frame_parent * glam::Vec3::X;
    assert!(twist_axis.distance(glam::Vec3::NEG_Y) < 1e-5, "{twist_axis:?}");
    // A joint between bones that have no body is refused.
    let lonely = &bodies[..1];
    assert!(asset::joints(&asset(), lonely).unwrap_err().to_string().contains("no body"));
}

#[test]
fn the_fall_configurations_are_in_metres_per_second_in_the_riders_frame() {
    let file = convert(&asset(), &blueprint()).unwrap();
    let high = &file["falls"]["high_side"];
    // 6 km/h up, a 5 percent push forward, a 3.6 km/h move to Unreal's +y (the rider's right, -y).
    assert!((high["additional"][2].as_f64().unwrap() - 6.0 / 3.6).abs() < 1e-5);
    assert_eq!(high["scale"], json!([0.05, 0.0, -0.28]), "six decimals, not the noise of a float");
    assert!((high["world"][1].as_f64().unwrap() + 1.0).abs() < 1e-5);
    assert_eq!(high["angular"], json!(0.5));
    assert_eq!(file["falls"]["low_side"]["scale"][2], json!(0.32));
    assert_eq!(file["falls"]["velocity_multiplier"], json!(1.0));
    assert!(asset::falls(&json!([])).is_err());
}

#[test]
fn the_file_is_what_the_game_reads() {
    let file = convert(&asset(), &blueprint()).unwrap();
    assert_eq!(file["bodies"].as_array().unwrap().len(), 2);
    assert!(
        (file["contact_friction"].as_f64().unwrap() - 0.66).abs() < 1e-6,
        "the mean of the rider's 0.62 and the road's 0.7"
    );
    assert_eq!(file["version"], json!(VERSION));
    assert!(file["joint_damping"].as_f64().unwrap() > 0.0);
    // Written and read back from a folder.
    let dir = std::env::temp_dir().join(format!("ride6-ragdoll-test-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let (a, b) = (dir.join("asset.json"), dir.join("bp.json"));
    std::fs::write(&a, asset().to_string()).unwrap();
    std::fs::write(&b, blueprint().to_string()).unwrap();
    let out = dir.join("RIDER");
    let summary = import(&a, &b, &out).unwrap();
    assert_eq!((summary.bodies, summary.joints), (2, 1));
    let back: Value = serde_json::from_slice(&std::fs::read(out.join("ragdoll.json")).unwrap()).unwrap();
    assert_eq!(back["bodies"][1]["bone"], json!("thigh_l"));
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn a_box_is_the_capsule_along_its_longest_side_with_the_mean_half_size_of_the_others_as_its_radius() {
    // The hand of the `_CB` asset: 9.0 x 6.2 x 8.4 cm, longest along X.
    let hand = json!([{"Type": "SkeletalBodySetup", "Properties": {
        "BoneName": "hand_l", "DefaultInstance": {"MassInKgOverride": 3.0},
        "AggGeom": {"BoxElems": [{"Center": {"X": 5.0, "Y": 0.0, "Z": 0.0}, "Rotation": {"Pitch": 0.0, "Yaw": 0.0, "Roll": 0.0},
            "X": 9.0, "Y": 6.2, "Z": 8.4}]}}}]);
    let bodies = asset::bodies(&hand).unwrap();
    let body = &bodies[0];
    // The radius is (6.2 + 8.4) / 4 = 3.65 cm and the straight part the rest of the 9 cm: 1.7 cm, half of it 0.85 cm.
    assert!((body.radius - 0.0365).abs() < 1e-6 && (body.half_length - 0.0085).abs() < 1e-6, "{body:?}");
    assert!(body.axis.x.abs() > 0.9999, "along the bone's x: {:?}", body.axis);
    // Neither a capsule nor a box is refused.
    let none = json!([{"Type": "SkeletalBodySetup", "Properties": {"BoneName": "hand_l", "DefaultInstance": {"MassInKgOverride": 3.0}, "AggGeom": {}}}]);
    assert!(asset::bodies(&none).is_err());
}

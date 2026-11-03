use super::*;
use blackbox_skeleton::Bone;

/// A rider reduced to a pelvis (0) with a two-bone spine (1, 2, 0.2 m each) whose top carries a shoulder (3, 0.1 m above it)
/// and an arm (4 elbow, 5 hand, 0.3 m each, hanging up) and a neck (6) and head (7). Forward is +x, up +z; the lean axis +y
/// takes the top of the body forward.
fn rider() -> (Skeleton, TorsoBones) {
    let up = |z: f32| Transform::from_rt(Quat::IDENTITY, Vec3::new(0.0, 0.0, z));
    let bone = |name: &str, parent: Option<usize>, rest: Transform| Bone { name: name.into(), parent, rest };
    let skeleton = Skeleton::new(vec![
        bone("pelvis", None, Transform::IDENTITY),
        bone("spine_01", Some(0), up(0.2)),
        bone("spine_02", Some(1), up(0.2)),
        bone("upperarm", Some(2), up(0.1)),
        bone("lowerarm", Some(3), up(0.3)),
        bone("hand", Some(4), up(0.3)),
        bone("neck", Some(2), up(0.2)),
        bone("head", Some(6), up(0.1)),
    ])
    .unwrap();
    (skeleton, TorsoBones { lean: vec![1, 2], head: vec![6, 7], head_share: 0.8 })
}

fn goal(tip: Vec3) -> Goal {
    Goal { root: 3, length: 0.6, tip }
}

fn component(skeleton: &Skeleton, local: &[Transform]) -> Vec<Transform> {
    let mut out = Vec::new();
    skeleton.component_transforms(local, &mut out).unwrap();
    out
}

#[test]
fn a_goal_the_arm_can_reach_needs_no_lean() {
    let (skeleton, torso) = rider();
    let local = skeleton.rest_pose().local;
    // The shoulder is 0.5 m above the pelvis; the goal is 0.5 m ahead of it (the arm is 0.6 m long, 0.558 is its reach).
    assert_eq!(solve(&skeleton, &local, &torso, Vec3::Y, &[goal(Vec3::new(0.5, 0.0, 0.5))]).unwrap(), 0.0);
    // No axis, no goals: no lean.
    assert_eq!(solve(&skeleton, &local, &torso, Vec3::ZERO, &[goal(Vec3::new(2.0, 0.0, 0.5))]).unwrap(), 0.0);
    assert_eq!(solve(&skeleton, &local, &torso, Vec3::Y, &[]).unwrap(), 0.0);
}

#[test]
fn a_goal_too_far_ahead_is_closed_by_the_smallest_lean_that_reaches_it() {
    let (skeleton, torso) = rider();
    let local = skeleton.rest_pose().local;
    let tip = Vec3::new(0.62, 0.0, 0.35);
    let goals = [goal(tip)];
    let angle = solve(&skeleton, &local, &torso, Vec3::Y, &goals).unwrap();
    assert!(angle > 0.1 && angle < MAX_LEAN, "{angle}");
    let shoulder = |angle: f32| {
        let mut turned = local.clone();
        bend(&skeleton, &mut turned, &torso, Vec3::Y, angle).unwrap();
        component(&skeleton, &turned)[3].translation
    };
    let reach = REACH_FRACTION * 0.6;
    assert!(shoulder(angle).distance(tip) <= reach + 1e-4, "reaches: {}", shoulder(angle).distance(tip));
    assert!(shoulder(angle - 0.01).distance(tip) > reach, "and no more than needed");
    assert!(shoulder(angle).x > shoulder(0.0).x, "the shoulder went forward");
}

#[test]
fn the_lean_is_held_to_its_limit_and_never_goes_backwards() {
    let (skeleton, torso) = rider();
    let local = skeleton.rest_pose().local;
    // Three metres ahead: no lean reaches it, the body folds as far as it may.
    let far = solve(&skeleton, &local, &torso, Vec3::Y, &[goal(Vec3::new(3.0, 0.0, 0.5))]).unwrap();
    assert!(far > 0.9 && far <= MAX_LEAN, "{far}");
    // A goal out of reach behind the rider would need a lean back: the nearest forward angle is none.
    assert_eq!(solve(&skeleton, &local, &torso, Vec3::Y, &[goal(Vec3::new(-0.9, 0.0, 0.5))]).unwrap(), 0.0);
    assert_eq!(solve(&skeleton, &local, &torso, Vec3::Y, &[goal(Vec3::splat(f32::NAN))]).unwrap(), 0.0);
}

#[test]
fn the_spine_turns_about_its_own_joints_and_the_head_turns_back_to_look_ahead() {
    let (skeleton, torso) = rider();
    let mut local = skeleton.rest_pose().local;
    let rest = component(&skeleton, &local);
    bend(&skeleton, &mut local, &torso, Vec3::Y, 0.8).unwrap();
    let bent = component(&skeleton, &local);
    assert!(bent[0].translation.distance(rest[0].translation) < 1e-6, "the pelvis stays");
    assert!(bent[1].translation.distance(rest[1].translation) < 1e-6, "the first spine bone turns about its own joint");
    // The spine's two bones share the lean: the second joint has turned 0.4 and the shoulder's frame 0.8.
    assert!(bent[1].rotation.angle_between(Quat::from_rotation_y(0.4)) < 1e-5);
    assert!(bent[2].rotation.angle_between(Quat::from_rotation_y(0.8)) < 1e-5);
    // The second joint is 0.2 m along the first bone, which leans 0.4 rad: x = 0.2 sin 0.4.
    assert!((bent[2].translation.x - 0.2 * 0.4_f32.sin()).abs() < 1e-5, "{:?}", bent[2].translation);
    // The head ends up pitched by (1 - head_share) of the lean: 0.8 * 0.2 = 0.16 rad forward.
    assert!(bent[7].rotation.angle_between(Quat::from_rotation_y(0.16)) < 1e-5, "{:?}", bent[7].rotation);
    // Lean and no lean are the same pose at zero.
    let mut same = skeleton.rest_pose().local;
    bend(&skeleton, &mut same, &torso, Vec3::Y, 0.0).unwrap();
    for (a, b) in same.iter().zip(&skeleton.rest_pose().local) {
        assert!(a.rotation.angle_between(b.rotation) < 1e-6);
    }
}

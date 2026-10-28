//! The rider's pose for one frame: the animation graph, the body's shift, then the hands on the grips and the feet on the
//! pegs by two-bone IK (`docs/specs/ride6-rider-animation.md`, sections 5 to 7).
//!
//! Three frames are in play, all with the axes of the car (x forward, y left, z up):
//!
//! - the **rider's** component space, where the skeleton's bones are;
//! - the bike **skeleton's** frame, where `attach.json` measured the seat, the grips and the pegs;
//! - the **model's** frame, the one the bike's solids are drawn in, which is the skeleton's moved by [`Frame::offset`].
//!
//! [`Frame`] joins them; the handlebar turns the grips about the steering axis of the model's front wheel.

use blackbox_skeleton::{Clip, Graph, Skeleton, Transform, TwoBoneChain, apply_two_bone};
use glam::{Mat4, Vec3};

use super::attach::Attach;

/// The frames the rider is placed in.
#[derive(Debug, Clone, Copy)]
pub struct Frame {
    /// The rider's component space to the model's frame: the seat, then the move from the skeleton's frame.
    pub to_model: Mat4,
    /// The model's frame to the rider's component space.
    pub to_rider: Mat4,
    /// The move from the bike skeleton's frame to the model's.
    pub offset: Vec3,
}

impl Frame {
    /// The frame of a rider whose root sits at `seat` of the bike skeleton, with the skeleton `offset` from the model.
    pub fn new(seat: &Transform, offset: Vec3) -> Self {
        let to_model =
            Mat4::from_translation(offset) * Mat4::from_rotation_translation(seat.rotation, seat.translation);
        Self { to_model, to_rider: to_model.inverse(), offset }
    }
}

/// A limb with its bones found in the rider's skeleton.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Limb {
    pub chain: TwoBoneChain,
    pub pole: usize,
    /// The index of the target (in `Attach::targets`) it reaches for.
    pub target: usize,
    /// Where the tip sits against the target in the authored pose, rider space.
    pub offset: Vec3,
}

/// The names of `attach.json` resolved against the rider's skeleton.
#[derive(Debug, Clone)]
pub struct Resolved {
    pub limbs: Vec<Limb>,
    /// The bone the body's shift moves.
    pub shift_bone: usize,
}

impl Resolved {
    /// Finds every bone and target the file names; says which one is missing.
    pub fn new(attach: &Attach, skeleton: &Skeleton) -> Result<Self, String> {
        let bone = |name: &str| skeleton.find(name).ok_or_else(|| format!("the rider has no bone {name}"));
        let mut limbs = Vec::new();
        for limb in &attach.limbs {
            let target = attach.target_index(&limb.target).ok_or_else(|| {
                format!("limb {} reaches for {}, which attach.json does not place", limb.name, limb.target)
            })?;
            limbs.push(Limb {
                chain: TwoBoneChain { root: bone(&limb.root)?, mid: bone(&limb.mid)?, tip: bone(&limb.tip)? },
                pole: bone(&limb.pole)?,
                target,
                offset: Vec3::from_array(limb.tip_offset),
            });
        }
        Ok(Self { limbs, shift_bone: bone(&attach.inertia.bone)? })
    }
}

/// The turn of the handlebar: the steering axis (unit) and a point on it, in the model's frame, and the angle (rad,
/// positive to the left, as the front wheel's).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Steering {
    pub axis: Vec3,
    pub point: Vec3,
    pub angle: f32,
}

impl Steering {
    /// The matrix that turns a point of the model by the handlebar's angle.
    pub fn matrix(&self) -> Mat4 {
        Mat4::from_translation(self.point)
            * Mat4::from_axis_angle(self.axis, self.angle)
            * Mat4::from_translation(-self.point)
    }
}

/// Where each target of `attach` is now, in the rider's component space: the grips turned with the handlebar, the pegs where
/// they are.
pub fn rider_targets(attach: &Attach, frame: &Frame, steering: Option<Steering>) -> Vec<Vec3> {
    let turn = steering.map_or(Mat4::IDENTITY, |s| s.matrix());
    attach
        .targets
        .iter()
        .map(|target| {
            let in_model = Vec3::from_array(target.position) + frame.offset;
            let moved = if target.steers { turn.transform_point3(in_model) } else { in_model };
            frame.to_rider.transform_point3(moved)
        })
        .collect()
}

/// Move the bone `bone` by `shift` (a vector in the rider's component space) without turning it: the translation of the
/// local transform changes by the shift as seen from the bone's parent.
fn shift_bone(
    skeleton: &Skeleton,
    local: &mut [Transform],
    bone: usize,
    shift: Vec3,
) -> Result<(), blackbox_skeleton::Error> {
    let mut component = Vec::new();
    skeleton.component_transforms(local, &mut component)?;
    let in_parent = match skeleton.bones()[bone].parent {
        Some(parent) => component[parent].rotation.inverse() * shift,
        None => shift,
    };
    local[bone].translation += in_parent;
    Ok(())
}

/// The component transforms of the rider for one frame: the graph's pose at `time` for `params`, the shift of the body
/// (`shift`, rider space), then each limb's tip brought to its target plus the limb's offset. `targets` come from
/// [`rider_targets`].
pub fn animate(
    skeleton: &Skeleton,
    clips: &[Clip],
    graph: &Graph,
    (params, time): (&[f32], f32),
    resolved: &Resolved,
    (targets, shift): (&[Vec3], Vec3),
) -> Result<Vec<Transform>, blackbox_skeleton::Error> {
    let mut pose = graph.evaluate(skeleton, clips, params, time);
    if shift != Vec3::ZERO {
        shift_bone(skeleton, &mut pose.local, resolved.shift_bone, shift)?;
    }
    let mut component = Vec::new();
    skeleton.component_transforms(&pose.local, &mut component)?;
    for limb in &resolved.limbs {
        let Some(target) = targets.get(limb.target) else { continue };
        let pole = component[limb.pole].translation;
        apply_two_bone(skeleton, &mut pose.local, limb.chain, *target + limb.offset, pole, 1.0, None)?;
    }
    skeleton.component_transforms(&pose.local, &mut component)?;
    Ok(component)
}

#[cfg(test)]
mod tests {
    use super::super::attach::tests::SAMPLE;
    use super::*;
    use blackbox_skeleton::{Bone, Node, TimeMode};
    use glam::Quat;

    /// A rider reduced to what the file names: a root, a pelvis, one arm (upperarm_l > lowerarm_l > hand_l, each 0.3 m up)
    /// and its pole bone.
    fn skeleton() -> Skeleton {
        let up = Transform::from_rt(Quat::IDENTITY, Vec3::new(0.0, 0.0, 0.3));
        let bone = |name: &str, parent: Option<usize>, rest: Transform| Bone { name: name.into(), parent, rest };
        Skeleton::new(vec![
            bone("root", None, Transform::IDENTITY),
            bone("pelvis", Some(0), Transform::from_rt(Quat::IDENTITY, Vec3::new(0.0, 0.0, 0.1))),
            bone("upperarm_l", Some(1), up),
            bone("lowerarm_l", Some(2), up),
            bone("hand_l", Some(3), up),
            bone("PV_arm_l", Some(0), Transform::from_rt(Quat::IDENTITY, Vec3::new(0.5, 0.0, 0.3))),
        ])
        .unwrap()
    }

    fn graph() -> (Graph, Vec<Clip>) {
        let clip = Clip {
            name: "rest".into(),
            length: 1.0,
            tracks: Vec::new(),
            additive: false,
            root_motion: false,
            notifies: Vec::new(),
        };
        (Graph { param_names: Vec::new(), root: Node::Clip { clip: 0, time: TimeMode::Hold } }, vec![clip])
    }

    fn attach() -> Attach {
        Attach::parse(SAMPLE.as_bytes()).unwrap()
    }

    fn frame() -> Frame {
        Frame::new(&attach().rider_root_in_bike.transform(), Vec3::new(0.0016, 0.0, 0.0797))
    }

    #[test]
    fn the_names_of_the_file_are_found_in_the_skeleton_or_the_missing_one_is_named() {
        let resolved = Resolved::new(&attach(), &skeleton()).unwrap();
        assert_eq!(resolved.limbs.len(), 1);
        assert_eq!(resolved.limbs[0].chain, TwoBoneChain { root: 2, mid: 3, tip: 4 });
        assert_eq!(resolved.limbs[0].pole, 5);
        assert_eq!(resolved.limbs[0].target, 0);
        assert_eq!(resolved.shift_bone, 1);
        let mut broken = attach();
        broken.limbs[0].tip = "claw".into();
        assert!(Resolved::new(&broken, &skeleton()).unwrap_err().contains("claw"));
        let mut lost = attach();
        lost.limbs[0].target = "elsewhere".into();
        assert!(Resolved::new(&lost, &skeleton()).unwrap_err().contains("elsewhere"));
    }

    #[test]
    fn the_frames_agree_and_undo_each_other() {
        let frame = frame();
        let point = Vec3::new(0.3, -0.2, 0.5);
        assert!(frame.to_rider.transform_point3(frame.to_model.transform_point3(point)).distance(point) < 1e-5);
        // The seat's quarter turn about z, then the move: the rider's origin is at the seat plus the offset.
        let origin = frame.to_model.transform_point3(Vec3::ZERO);
        assert!(origin.distance(Vec3::new(-0.34 + 0.0016, 0.0, 0.91 + 0.0797)) < 1e-5, "{origin:?}");
    }

    #[test]
    fn a_grip_turns_with_the_handlebar_about_the_steering_axis_and_a_peg_does_not() {
        let attach = attach();
        let frame = frame();
        let axis = Vec3::new(-0.4384, 0.0, 0.8988).normalize();
        let steering = Steering { axis, point: Vec3::new(0.84, 0.0, 0.0), angle: 0.3 };
        let straight = rider_targets(&attach, &frame, None);
        let turned = rider_targets(&attach, &frame, Some(steering));
        assert!(straight[1].distance(turned[1]) < 1e-6, "the peg stays");
        assert!(straight[0].distance(turned[0]) > 0.02, "the grip moves");
        // The grip stays as far from the axis as it was.
        let in_model = |p: Vec3| frame.to_model.transform_point3(p);
        let distance = |p: Vec3| (p - steering.point).cross(axis).length();
        assert!((distance(in_model(straight[0])) - distance(in_model(turned[0]))).abs() < 1e-4);
        // The grip is behind and above the axis point, so a turn to the left (positive) takes it to the right.
        let moved = in_model(turned[0]) - in_model(straight[0]);
        assert!(moved.y < 0.0, "{moved:?}");
        let identity = Steering { angle: 0.0, ..steering };
        assert!(rider_targets(&attach, &frame, Some(identity))[0].distance(straight[0]) < 1e-6);
    }

    #[test]
    fn the_hand_is_brought_to_its_target_plus_the_authored_offset() {
        let (graph, clips) = graph();
        let skeleton = skeleton();
        let resolved = Resolved::new(&attach(), &skeleton).unwrap();
        // Reachable: the shoulder is 0.4 m above the root's origin and the arm 0.6 m long.
        let target = Vec3::new(0.2, 0.1, 0.8);
        let wanted = target + resolved.limbs[0].offset;
        let component = animate(&skeleton, &clips, &graph, (&[], 0.0), &resolved, (&[target], Vec3::ZERO)).unwrap();
        assert!(component[4].translation.distance(wanted) < 1e-3, "{:?} vs {wanted:?}", component[4].translation);
        // The bones keep their lengths.
        assert!((component[3].translation.distance(component[2].translation) - 0.3).abs() < 1e-3);
        assert!((component[4].translation.distance(component[3].translation) - 0.3).abs() < 1e-3);
    }

    #[test]
    fn a_target_out_of_reach_stretches_the_arm_towards_it_without_breaking_it() {
        let (graph, clips) = graph();
        let skeleton = skeleton();
        let resolved = Resolved::new(&attach(), &skeleton).unwrap();
        let component =
            animate(&skeleton, &clips, &graph, (&[], 0.0), &resolved, (&[Vec3::new(3.0, 0.0, 0.4)], Vec3::ZERO))
                .unwrap();
        let shoulder = component[2].translation;
        let hand = component[4].translation;
        assert!(hand.distance(shoulder) <= 0.6 + 1e-3);
        assert!(hand.x > shoulder.x + 0.4, "it reaches towards the target: {hand:?}");
    }

    #[test]
    fn the_shift_moves_the_pelvis_and_what_hangs_on_it_but_not_the_pole() {
        let (graph, clips) = graph();
        let skeleton = skeleton();
        let mut attach = attach();
        attach.limbs.clear();
        let resolved = Resolved::new(&attach, &skeleton).unwrap();
        let rest = animate(&skeleton, &clips, &graph, (&[], 0.0), &resolved, (&[], Vec3::ZERO)).unwrap();
        let shifted =
            animate(&skeleton, &clips, &graph, (&[], 0.0), &resolved, (&[], Vec3::new(0.0, -0.05, 0.0))).unwrap();
        assert!(shifted[1].translation.distance(rest[1].translation + Vec3::new(0.0, -0.05, 0.0)) < 1e-6);
        assert!(shifted[4].translation.distance(rest[4].translation + Vec3::new(0.0, -0.05, 0.0)) < 1e-6);
        assert!(shifted[5].translation.distance(rest[5].translation) < 1e-6, "the pole is on the root");
    }

    #[test]
    fn a_target_the_file_did_not_place_leaves_the_limb_alone() {
        let (graph, clips) = graph();
        let skeleton = skeleton();
        let resolved = Resolved::new(&attach(), &skeleton).unwrap();
        let none = animate(&skeleton, &clips, &graph, (&[], 0.0), &resolved, (&[], Vec3::ZERO)).unwrap();
        let mut at_rest = Vec::new();
        skeleton.component_transforms(&skeleton.rest_pose().local, &mut at_rest).unwrap();
        assert!(none[4].translation.distance(at_rest[4].translation) < 1e-6);
    }
}

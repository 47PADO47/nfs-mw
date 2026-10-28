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
use glam::{Mat4, Quat, Vec3};

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

/// How the rider's body sits on the bike beyond the authored pose: his centre of mass moved by `shift` (a vector in the rider's
/// component space, metres) and his body turned by `rotation` (a rotation in the same space) about the bone that carries it.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Body {
    pub shift: Vec3,
    pub rotation: Quat,
}

impl Body {
    /// The body as authored: not moved, not turned.
    pub const REST: Self = Self { shift: Vec3::ZERO, rotation: Quat::IDENTITY };
}

/// The body of the rider from the physics: `shift` is his displacement from the rest place in the car's axes (x forward,
/// y left, z up) and `rotation` his body's turn against the bike's (roll to the right, pitch nose up, yaw to the right,
/// radians). In the car's axes (a right-handed frame with x forward, y left, z up) a roll to the right is a turn about +x, a
/// pitch nose-up a turn about -y and a yaw to the right a turn about -z.
pub fn body(frame: &Frame, shift: Vec3, rotation: Vec3) -> Body {
    let turn = Vec3::new(rotation.x, -rotation.y, -rotation.z);
    let in_rider = frame.to_rider.transform_vector3(turn);
    let rotation = match in_rider.is_finite() {
        true => Quat::from_scaled_axis(in_rider),
        false => Quat::IDENTITY,
    };
    Body { shift: frame.to_rider.transform_vector3(shift), rotation }
}

/// Move the bone `bone` by `body.shift` and turn it by `body.rotation` about its own place, without moving its parent: the
/// local transform changes so that the bone's component transform is the shifted and turned one, and what hangs on it follows.
fn move_bone(
    skeleton: &Skeleton,
    local: &mut [Transform],
    bone: usize,
    body: Body,
) -> Result<(), blackbox_skeleton::Error> {
    let mut component = Vec::new();
    skeleton.component_transforms(local, &mut component)?;
    let parent_rotation = match skeleton.bones()[bone].parent {
        Some(parent) => component[parent].rotation,
        None => Quat::IDENTITY,
    };
    local[bone].translation += parent_rotation.inverse() * body.shift;
    local[bone].rotation =
        (parent_rotation.inverse() * body.rotation * parent_rotation * local[bone].rotation).normalize();
    Ok(())
}

/// The component transforms of the rider for one frame: the graph's pose at `time` for `params`, the shift and turn of the body
/// (`body`, rider space), then each limb's tip brought to its target plus the limb's offset. `targets` come from
/// [`rider_targets`].
pub fn animate(
    skeleton: &Skeleton,
    clips: &[Clip],
    graph: &Graph,
    (params, time): (&[f32], f32),
    resolved: &Resolved,
    (targets, body): (&[Vec3], Body),
) -> Result<Vec<Transform>, blackbox_skeleton::Error> {
    let mut pose = graph.evaluate(skeleton, clips, params, time);
    if body != Body::REST {
        move_bone(skeleton, &mut pose.local, resolved.shift_bone, body)?;
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
        let component = animate(&skeleton, &clips, &graph, (&[], 0.0), &resolved, (&[target], Body::REST)).unwrap();
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
            animate(&skeleton, &clips, &graph, (&[], 0.0), &resolved, (&[Vec3::new(3.0, 0.0, 0.4)], Body::REST))
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
        let rest = animate(&skeleton, &clips, &graph, (&[], 0.0), &resolved, (&[], Body::REST)).unwrap();
        let shifted = animate(
            &skeleton,
            &clips,
            &graph,
            (&[], 0.0),
            &resolved,
            (&[], Body { shift: Vec3::new(0.0, -0.05, 0.0), ..Body::REST }),
        )
        .unwrap();
        assert!(shifted[1].translation.distance(rest[1].translation + Vec3::new(0.0, -0.05, 0.0)) < 1e-6);
        assert!(shifted[4].translation.distance(rest[4].translation + Vec3::new(0.0, -0.05, 0.0)) < 1e-6);
        assert!(shifted[5].translation.distance(rest[5].translation) < 1e-6, "the pole is on the root");
    }

    #[test]
    fn a_target_the_file_did_not_place_leaves_the_limb_alone() {
        let (graph, clips) = graph();
        let skeleton = skeleton();
        let resolved = Resolved::new(&attach(), &skeleton).unwrap();
        let none = animate(&skeleton, &clips, &graph, (&[], 0.0), &resolved, (&[], Body::REST)).unwrap();
        let mut at_rest = Vec::new();
        skeleton.component_transforms(&skeleton.rest_pose().local, &mut at_rest).unwrap();
        assert!(none[4].translation.distance(at_rest[4].translation) < 1e-6);
    }

    /// A rider whose root is `pelvis` > `spine` and one bone far out to the left (+y of the car's frame).
    fn arm_skeleton() -> Skeleton {
        let bone = |name: &str, parent: Option<usize>, t: Vec3| Bone {
            name: name.into(),
            parent,
            rest: Transform::from_rt(Quat::IDENTITY, t),
        };
        Skeleton::new(vec![
            bone("root", None, Vec3::ZERO),
            bone("pelvis", Some(0), Vec3::new(0.0, 0.0, 0.1)),
            bone("hand_l", Some(1), Vec3::new(0.0, 0.5, 0.0)),
            bone("hand_r", Some(1), Vec3::new(0.0, -0.5, 0.0)),
            bone("head", Some(1), Vec3::new(0.0, 0.0, 0.6)),
        ])
        .unwrap()
    }

    fn arm_pose(body: Body) -> Vec<Transform> {
        let (graph, clips) = graph();
        let skeleton = arm_skeleton();
        let resolved = Resolved { limbs: Vec::new(), shift_bone: 1 };
        animate(&skeleton, &clips, &graph, (&[], 0.0), &resolved, (&[], body)).unwrap()
    }

    /// The car's own frame in the rider's space for the tests: the identity.
    fn plain_frame() -> Frame {
        Frame::new(&Transform::IDENTITY, Vec3::ZERO)
    }

    #[test]
    fn hanging_off_to_the_inside_moves_the_body_and_its_bones_that_way_and_mirrors() {
        let rest = arm_pose(Body::REST);
        // 10 cm to the right (car axes: y is left, so -y), and the same to the left.
        let right = arm_pose(body(&plain_frame(), Vec3::new(0.0, -0.1, 0.0), Vec3::ZERO));
        let left = arm_pose(body(&plain_frame(), Vec3::new(0.0, 0.1, 0.0), Vec3::ZERO));
        for bone in [1, 2, 3, 4] {
            let moved = right[bone].translation - rest[bone].translation;
            assert!(moved.distance(Vec3::new(0.0, -0.1, 0.0)) < 1e-6, "bone {bone}: {moved:?}");
            let mirrored = left[bone].translation - rest[bone].translation;
            assert!((moved + mirrored).length() < 1e-6, "mirror image");
        }
        assert_eq!(right[0].translation, rest[0].translation, "the root stays on the seat");
    }

    #[test]
    fn the_body_turned_to_the_right_swings_the_head_to_the_right_about_the_pelvis() {
        let rest = arm_pose(Body::REST);
        let right = arm_pose(body(&plain_frame(), Vec3::ZERO, Vec3::new(0.2, 0.0, 0.0)));
        let left = arm_pose(body(&plain_frame(), Vec3::ZERO, Vec3::new(-0.2, 0.0, 0.0)));
        // The pelvis stays where it is; the head (0.6 m above it) moves toward -y (right) by about 0.6 sin(0.2).
        assert!(right[1].translation.distance(rest[1].translation) < 1e-6);
        let swing = right[4].translation - rest[4].translation;
        assert!(swing.y < -0.1 && (swing.y + 0.6 * 0.2_f32.sin()).abs() < 1e-5, "{swing:?}");
        let other = left[4].translation - rest[4].translation;
        assert!((swing.y + other.y).abs() < 1e-6 && (swing.z - other.z).abs() < 1e-6, "mirror image");
        // The arms swing about the pelvis too: the left hand rises, the right one drops, when the body rolls to the right.
        assert!(right[2].translation.z > rest[2].translation.z && right[3].translation.z < rest[3].translation.z);
    }

    #[test]
    fn a_pitch_or_a_yaw_turns_about_the_axes_of_the_cars_frame() {
        let rest = arm_pose(Body::REST);
        // Nose up: the head goes back (-x of the car).
        let up = arm_pose(body(&plain_frame(), Vec3::ZERO, Vec3::new(0.0, 0.2, 0.0)));
        assert!(up[4].translation.x < rest[4].translation.x - 0.05, "{:?}", up[4].translation);
        // Yaw to the right: the left hand (+y) moves forward and the right hand back.
        let yaw = arm_pose(body(&plain_frame(), Vec3::ZERO, Vec3::new(0.0, 0.0, 0.2)));
        assert!(
            yaw[2].translation.x > rest[2].translation.x + 0.05 && yaw[3].translation.x < rest[3].translation.x - 0.05
        );
        // A body that is not moved or turned is the rest pose.
        assert_eq!(body(&plain_frame(), Vec3::ZERO, Vec3::ZERO).rotation, Quat::IDENTITY);
        assert_eq!(body(&plain_frame(), Vec3::ZERO, Vec3::splat(f32::NAN)).rotation, Quat::IDENTITY);
    }
}

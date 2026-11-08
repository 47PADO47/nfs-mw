//! The rider's pose for one frame: the animation graph, the body sat on the seat and leaned to the grips, then the hands on the
//! grips and the feet on the pegs by two-bone IK (`docs/specs/ride6-rider-animation.md`, sections 5 to 7;
//! `docs/specs/ride6-rider-render-integration.md`, section 8).
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
use super::reach::{self, Goal, TorsoBones};
use super::seat::Seat;

/// The frames the rider is placed in.
#[derive(Debug, Clone, Copy)]
pub struct Frame {
    /// The rider's component space to the model's frame: the seat, then the move from the skeleton's frame.
    pub to_model: Mat4,
    /// The model's frame to the rider's component space.
    pub to_rider: Mat4,
    /// The rider's component space to the bike skeleton's frame (the seat's turn and place): where `attach.json` and the seat's
    /// surface are measured.
    pub to_bike: Mat4,
    /// The turn of [`Self::to_rider`].
    pub rider_turn: Quat,
    /// The move from the bike skeleton's frame to the model's.
    pub offset: Vec3,
}

impl Frame {
    /// The frame of a rider whose root sits at `seat` of the bike skeleton, with the skeleton `offset` from the model.
    pub fn new(seat: &Transform, offset: Vec3) -> Self {
        let to_bike = Mat4::from_rotation_translation(seat.rotation, seat.translation);
        let to_model = Mat4::from_translation(offset) * to_bike;
        Self { to_model, to_rider: to_model.inverse(), to_bike, rider_turn: seat.rotation.inverse(), offset }
    }

    /// The axis in the rider's component space about which a forward lean of the body is a positive turn (the car's left, so
    /// the top of the body goes forward).
    pub fn lean_axis(&self) -> Vec3 {
        self.to_rider.transform_vector3(Vec3::Y).normalize_or_zero()
    }

    /// Up, as a direction of the rider's component space.
    pub fn up(&self) -> Vec3 {
        self.to_rider.transform_vector3(Vec3::Z).normalize_or_zero()
    }
}

/// The names `attach.json` gives the rider's legs, left and right: the ones [`Goals::release`] lets off the pegs.
const LEG_NAMES: [&str; 2] = ["left_leg", "right_leg"];

/// A limb with its bones found in the rider's skeleton.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Limb {
    pub chain: TwoBoneChain,
    pub pole: usize,
    /// The index of the target (in `Attach::targets`) it reaches for.
    pub target: usize,
    /// Where the tip sits against the target in the authored pose, rider space: used when the target has no turn.
    pub offset: Vec3,
    /// Where the tip sits in the target's own frame (a hand on its grip socket, a foot against its peg socket).
    pub frame: Option<Transform>,
    /// Where the tip is against the grip bone it holds: the tip is put at the target's `anchor` composed with this.
    pub anchor: Option<Transform>,
    /// Whether the limb's root hangs on the bones the body leans with (an arm does, a leg does not).
    pub on_torso: bool,
    /// Which entry of [`Goals::release`] lets this limb off its target: the rider's legs, left (0) and right (1), found by the
    /// names `left_leg` and `right_leg` of `attach.json`.
    pub release: Option<usize>,
}

/// The names of `attach.json` resolved against the rider's skeleton.
#[derive(Debug, Clone)]
pub struct Resolved {
    pub limbs: Vec<Limb>,
    /// The bone the body's shift moves.
    pub shift_bone: usize,
    /// The bones the body leans with to reach the grips, if the file names them.
    pub torso: Option<TorsoBones>,
    /// The buttocks' contact with the seat, if the file and the bike have what it needs.
    pub seat: Option<Seat>,
}

impl Resolved {
    /// Finds every bone and target the file names; says which one is missing.
    pub fn new(attach: &Attach, skeleton: &Skeleton) -> Result<Self, String> {
        let bone = |name: &str| skeleton.find(name).ok_or_else(|| format!("the rider has no bone {name}"));
        let torso = attach.torso.as_ref().map(|torso| TorsoBones::new(torso, skeleton)).transpose()?;
        let mut limbs = Vec::new();
        for limb in &attach.limbs {
            let target = attach.target_index(&limb.target).ok_or_else(|| {
                format!("limb {} reaches for {}, which attach.json does not place", limb.name, limb.target)
            })?;
            let chain = TwoBoneChain { root: bone(&limb.root)?, mid: bone(&limb.mid)?, tip: bone(&limb.tip)? };
            limbs.push(Limb {
                chain,
                pole: bone(&limb.pole)?,
                target,
                offset: Vec3::from_array(limb.tip_offset),
                frame: limb.tip_frame.map(|frame| frame.transform()),
                anchor: limb.tip_anchor.map(|anchor| anchor.transform()),
                on_torso: torso.as_ref().is_some_and(|t| t.lean.iter().any(|&b| skeleton.is_descendant(chain.root, b))),
                release: LEG_NAMES.iter().position(|name| *name == limb.name),
            });
        }
        Ok(Self { limbs, shift_bone: bone(&attach.inertia.bone)?, torso, seat: None })
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

/// A point a limb reaches for, in the rider's component space: where it is and, if the bike's file gives it, how it is turned.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Placed {
    pub position: Vec3,
    pub rotation: Option<Quat>,
    /// Where the grip bone it is held on is and how it is turned, if the bike's file says.
    pub anchor: Option<(Vec3, Quat)>,
}

/// Where each target of `attach` is now, in the rider's component space: the grips turned with the handlebar, the pegs where
/// they are.
pub fn rider_targets(attach: &Attach, frame: &Frame, steering: Option<Steering>) -> Vec<Placed> {
    let turn = steering.map_or(Mat4::IDENTITY, |s| s.matrix());
    let turned = steering.map_or(Quat::IDENTITY, |s| Quat::from_axis_angle(s.axis, s.angle));
    attach
        .targets
        .iter()
        .map(|target| {
            let place = |point: Vec3| {
                let in_model = point + frame.offset;
                let moved = match target.steers {
                    true => turn.transform_point3(in_model),
                    false => in_model,
                };
                frame.to_rider.transform_point3(moved)
            };
            let rotation = target.rotation.map(|[x, y, z, w]| {
                let own = Quat::from_xyzw(x, y, z, w).normalize();
                let in_model = match target.steers {
                    true => turned * own,
                    false => own,
                };
                frame.rider_turn * in_model
            });
            Placed {
                position: place(Vec3::from_array(target.position)),
                rotation,
                anchor: target.anchor.zip(target.anchor_rotation).map(|(at, [x, y, z, w])| {
                    let own = Quat::from_xyzw(x, y, z, w).normalize();
                    let in_model = if target.steers { turned * own } else { own };
                    (place(Vec3::from_array(at)), frame.rider_turn * in_model)
                }),
            }
        })
        .collect()
}

/// Roll the right wrist by `wrist` (0 to 1, the throttle): the hand turns about the grip, which stays in the fist. The angle is the one
/// of the file's offset at full throttle (`Attach::wrist_throttle`, the blueprint's `Hand_R_WristDownRot`, 30 degrees); its
/// translation is not played (it moved the hand 5 cm off the grip and under it, which a user's playtest showed:
/// `docs/specs/ride6-rider-hands.md`, section 2). A file without the offset is left as it is.
///
/// The grip bone holds the hand (`anchor`): the bone's own y axis is the grip's cylinder (measured on four bikes, within a millimetre
/// and a degree of the grip mesh's own axis, [`GRIP_AXIS`]) and its origin lies on that axis, so turning the bone about it takes the
/// hand round the grip without moving the grip. The roll takes the knuckles toward the rider (the throttle opens). Without a bone
/// (older files) the grip socket's frame turns by the file's rotation, about the socket's own axis, which is the bar's.
pub fn roll_wrist(attach: &Attach, targets: &mut [Placed], wrist: f32) {
    let Some(offset) = attach.wrist_throttle else { return };
    let Some(placed) = attach.target_index("right_grip").and_then(|index| targets.get_mut(index)) else { return };
    let share = wrist.clamp(0.0, 1.0);
    let [x, y, z, w] = offset.rotation;
    let rolled = Quat::from_xyzw(x, y, z, w).normalize();
    if let Some((at, bone)) = placed.anchor {
        // +y of the right bone points to the left, along the grip toward the rider: the roll toward him is a turn the other way.
        let roll = Quat::from_axis_angle(GRIP_AXIS, -Quat::IDENTITY.angle_between(rolled) * share);
        placed.anchor = Some((at, bone * roll));
        return;
    }
    let Some(turn) = placed.rotation else { return };
    placed.rotation = Some(turn * Quat::IDENTITY.slerp(rolled, share));
}

/// The grip bone's own axis along the grip, in the bone's frame as the importer writes it (the car's axes: the bone's y; the exporter's
/// bones have it on z, which the car's axes turn into y).
const GRIP_AXIS: Vec3 = Vec3::Y;

/// Where the tip of `limb` is to be for its target `placed`, and the turn it takes there (`None`: the arm's own).
pub fn tip_goal(limb: &Limb, placed: &Placed) -> (Vec3, Option<Quat>) {
    // A hand holds the bike's grip the way the authored pose holds the reference bike's: it keeps its place and turn against the grip
    // bone (the bike's own hand socket is only near it: up to 6 cm off on a sport bike's clip-ons).
    if let (Some(held), Some((at, turn))) = (limb.anchor, placed.anchor) {
        return (at + turn * held.translation, Some(turn * held.rotation));
    }
    match (limb.frame, placed.rotation) {
        (Some(frame), Some(turn)) => (placed.position + turn * frame.translation, Some(turn * frame.rotation)),
        _ => (placed.position + limb.offset, None),
    }
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

/// What the pose is to meet: where the limbs reach, how the body moves and the frames that join the rider to the bike.
#[derive(Debug, Clone, Copy)]
pub struct Goals<'a> {
    /// From [`rider_targets`].
    pub targets: &'a [Placed],
    pub body: Body,
    pub frame: &'a Frame,
    /// How far each leg (left, right) is let off its peg, 0 (on it) to 1 (the leg as the animation poses it): the blueprint's
    /// `OverrideFootIK_Leaning_L/R`.
    pub release: [f32; 2],
}

/// The component transforms of the rider for one frame: the graph's pose at `time` for `params`, the body lowered onto the seat,
/// shifted and turned by the physics, leaned forward until the arms reach, then each limb's tip brought to its target.
pub fn animate(
    skeleton: &Skeleton,
    clips: &[Clip],
    graph: &Graph,
    (params, time): (&[f32], f32),
    resolved: &Resolved,
    goals: &Goals,
) -> Result<Vec<Transform>, blackbox_skeleton::Error> {
    let mut pose = graph.evaluate(skeleton, clips, params, time);
    let mut component = Vec::new();
    let mut body = goals.body;
    // Sat on the seat of this bike: the buttocks rest on its surface whatever height the skeleton's socket is at.
    if let Some(seat) = &resolved.seat {
        skeleton.component_transforms(&pose.local, &mut component)?;
        if let Some(drop) = seat.drop(&component, &goals.frame.to_bike) {
            body.shift -= goals.frame.up() * drop;
        }
    }
    if body != Body::REST {
        move_bone(skeleton, &mut pose.local, resolved.shift_bone, body)?;
    }
    skeleton.component_transforms(&pose.local, &mut component)?;
    // A bike whose grips are farther than the arms reach (a racer's low bars against the upright reference pose) gets the
    // body leaned forward until they can (the full-body solve of the original does this with the spine).
    if let Some(torso) = &resolved.torso {
        let reach_for: Vec<Goal> = resolved
            .limbs
            .iter()
            .filter(|limb| limb.on_torso)
            .filter_map(|limb| {
                let placed = goals.targets.get(limb.target)?;
                let at = |bone: usize| component[bone].translation;
                let length =
                    at(limb.chain.mid).distance(at(limb.chain.root)) + at(limb.chain.tip).distance(at(limb.chain.mid));
                Some(Goal { root: limb.chain.root, length, tip: tip_goal(limb, placed).0 })
            })
            .collect();
        let axis = goals.frame.lean_axis();
        let lean = reach::solve(skeleton, &pose.local, torso, axis, &reach_for)?;
        if lean > 0.0 {
            reach::bend(skeleton, &mut pose.local, torso, axis, lean)?;
            skeleton.component_transforms(&pose.local, &mut component)?;
        }
    }
    for limb in &resolved.limbs {
        let Some(placed) = goals.targets.get(limb.target) else { continue };
        let (position, turn) = tip_goal(limb, placed);
        let pole = component[limb.pole].translation;
        let release = limb.release.map_or(0.0, |side| goals.release[side].clamp(0.0, 1.0));
        let own = pose.local[limb.chain.tip].rotation;
        apply_two_bone(skeleton, &mut pose.local, limb.chain, position, pole, 1.0 - release, turn)?;
        if release > 0.0 {
            // The foot's own turn comes back with the leg: the peg's turn at 0, the animation's at 1.
            let pegged = pose.local[limb.chain.tip].rotation;
            pose.local[limb.chain.tip].rotation = pegged.slerp(own, release);
        }
    }
    skeleton.component_transforms(&pose.local, &mut component)?;
    Ok(component)
}

#[cfg(test)]
#[path = "pose_tests.rs"]
mod tests;

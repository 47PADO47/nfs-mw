//! The rider's fall posed by animation (`docs/specs/ride6-rider-fall-animation.md`).
//!
//! A limp ragdoll is not what the reference shows: its rider's bodies are *driven* to the pose of a fall animation while the pelvis
//! is free to carry the physics. Here the animation is [`blackbox_skeleton::fall`], the state machine of `falls.json` (launch clips by
//! the kind of fall, the tumble loops by how he lies, the get-up clips once he is still); each physics step it is told what the
//! ragdoll is doing, and the pose it gives is the target the ragdoll's joints are driven to. Once the get-up begins the body is no
//! longer simulated: the clip plays on the ground where he lies.

use blackbox_skeleton::{FallInput, FallPlayer, FallSet, Pack, Pose, Skeleton, Transform};
use blackbox_vehicle::Ground;
use blackbox_vehicle::bike::FallKind;
use game_install::GameDir;
use glam::{Quat, Vec3};

use super::fall::Tumble;
use super::fall_space::{quat_to_physics, quat_to_render, to_render};

/// The share of the way to its target pose a driven joint goes in each solver pass. A modelling choice: the reference drives each body with
/// a strength curve of 1,000,000 (`Ragdoll_strength_Race`), a stiffness so far above the weight of the body that the pose is held
/// as if rigid; half the way per pass (16 passes in a step) holds it within a few degrees.
pub const DRIVE_GAIN: f32 = 0.5;
/// How far below the pelvis the ground is looked for (m); a pelvis higher than that counts as that high.
const GROUND_REACH: f32 = 3.0;
/// How far above the pelvis the ground ray starts (m).
const LOOK_ABOVE: f32 = 0.4;
/// How long (s) after the bike is stood up the rider is let lie still before he is put back on it even if he has not yet begun to get up.
/// A modelling choice: the bike stands up where it is, the rider may still be sliding.
pub const RETURN_GRACE: f32 = 2.0;
/// The longest the rider is kept off the bike after it is stood up (s), whatever he is doing. A modelling choice.
pub const RETURN_LIMIT: f32 = 8.0;

/// The name `falls.json` gives a kind of fall.
pub fn kind_name(kind: FallKind) -> &'static str {
    match kind {
        FallKind::HighSide => "high_side",
        FallKind::LowSide => "low_side",
        FallKind::Generic => "generic",
    }
}

/// The fall animation of the rider whose files are in `folder`, if it has one that fits his pack.
pub fn load_set(dir: &GameDir, folder: &str, pack: &Pack) -> Option<FallSet> {
    let name = format!("{folder}/falls.json");
    if !dir.exists(&name) {
        log::info!("rider: {folder} has no falls.json; the rider falls limp");
        return None;
    }
    let read = || -> anyhow::Result<FallSet> {
        let set: FallSet = serde_json::from_slice(&dir.read(&name)?)?;
        set.validate(&pack.clips, &pack.skeleton)?;
        Ok(set)
    };
    match read() {
        Ok(set) => Some(set),
        Err(error) => {
            log::warn!("rider: {name} is not used: {error:#}");
            None
        }
    }
}

/// A joint of the ragdoll that follows the animation: the bones its two bodies are on and the rotations from their body frames to
/// the bones' (the bodies' axes).
#[derive(Debug, Clone, Copy)]
pub struct DriveLink {
    pub joint: usize,
    pub parent_bone: usize,
    pub child_bone: usize,
    pub parent_axis: Quat,
    pub child_axis: Quat,
}

/// Where the get-up puts the rider: the rider space to the render space, level, and the origin the pose is given from.
#[derive(Debug, Clone, Copy)]
pub struct GetUp {
    pub align: Transform,
}

/// The animation of a tumble in progress.
#[derive(Debug, Clone)]
pub struct Anim {
    pub player: FallPlayer,
    /// The local pose the bodies are driven to (rider space): the fall animation's, mixed with the riding pose at first.
    pub target: Vec<Transform>,
    pub drives: Vec<DriveLink>,
    /// The body (index into the links) the others hang from: the one that is nobody's child.
    pub root: usize,
    /// Where the front and the right of the body point, in the frame of the root's bone.
    pub front: Vec3,
    pub right: Vec3,
    pub getup: Option<GetUp>,
    /// Whether the get-up has played to its end.
    pub done: bool,
}

impl Anim {
    pub fn new(
        set: &FallSet,
        kind: FallKind,
        riding: &[Transform],
        drives: Vec<DriveLink>,
        root: usize,
        rest: (Vec3, Vec3),
    ) -> Self {
        Self {
            player: FallPlayer::new(set, kind_name(kind)),
            target: riding.to_vec(),
            drives,
            root,
            front: rest.0,
            right: rest.1,
            getup: None,
            done: false,
        }
    }
}

/// The rotation of each bone in the component space of `local`.
fn component_rotations(skeleton: &Skeleton, local: &[Transform]) -> Vec<Quat> {
    let mut out: Vec<Quat> = Vec::with_capacity(local.len());
    for (i, bone) in skeleton.bones().iter().enumerate() {
        out.push(match bone.parent {
            Some(parent) => (out[parent] * local[i].rotation).normalize(),
            None => local[i].rotation,
        });
    }
    out
}

impl Tumble {
    /// One step of the animation: tell it what the ragdoll does, take the pose it asks for and drive the joints to it; from the
    /// moment he is still enough, begin the get-up.
    pub fn animate(&mut self, set: &FallSet, pack: &Pack, dt: f32, ground: &dyn Ground) {
        let Some(mut anim) = self.anim.take() else { return };
        let skeleton = &pack.skeleton;
        if anim.getup.is_some() {
            anim.player.step(set, &pack.clips, dt, FallInput::default());
            anim.target = anim.player.pose(set, skeleton, &pack.clips).local;
            anim.done = anim.player.finished(set, &pack.clips);
            self.anim = Some(anim);
            return;
        }
        let input = self.fall_input(&anim, ground);
        anim.player.step(set, &pack.clips, dt, input);
        let fall = anim.player.pose(set, skeleton, &pack.clips);
        let riding = Pose { local: self.locals.clone() };
        anim.target = riding.blend(&fall, anim.player.fade(set)).local;
        let rotations = component_rotations(skeleton, &anim.target);
        for link in &anim.drives {
            let relative = rotations[link.parent_bone].inverse() * rotations[link.child_bone];
            self.ragdoll.drive(link.joint, link.parent_axis.inverse() * quat_to_physics(relative) * link.child_axis);
        }
        if anim.player.resting() {
            self.begin_get_up(&mut anim, skeleton);
        }
        self.anim = Some(anim);
    }

    /// What the animation reads of the ragdoll: how the pelvis lies (the front and the right of the body against up), how fast it moves
    /// and how far above the ground it is.
    fn fall_input(&self, anim: &Anim, ground: &dyn Ground) -> FallInput {
        let link = self.links[anim.root];
        let rotation = quat_to_render(self.ragdoll.orientation(anim.root) * link.axis.inverse());
        let position = self.ragdoll.position(anim.root);
        // The ray starts a little above the pelvis, as the ragdoll's own contacts do: a road is single sided.
        let above = Vec3::Y * LOOK_ABOVE;
        let airborne = ground
            .hit(position + above, Vec3::NEG_Y, GROUND_REACH + LOOK_ABOVE)
            .map_or(GROUND_REACH, |hit| (hit.distance - LOOK_ABOVE).max(0.0));
        let input = FallInput {
            face: (rotation * anim.front).z,
            side: (rotation * anim.right).z,
            speed: self.ragdoll.velocity(anim.root).length(),
            airborne,
        };
        log::debug!(
            "rider fall: face {:.2} side {:.2} speed {:.2} m/s, {:.2} m up",
            input.face,
            input.side,
            input.speed,
            input.airborne
        );
        input
    }

    /// Stop simulating: the get-up plays in the frame that puts the pose he is driven to on the pelvis where it lies, set upright
    /// (the ground is level for a body that gets up).
    fn begin_get_up(&mut self, anim: &mut Anim, skeleton: &Skeleton) {
        let mut component = Vec::new();
        if skeleton.component_transforms(&anim.target, &mut component).is_err() {
            return;
        }
        let link = self.links[anim.root];
        let bone = component[link.bone];
        let rotation = quat_to_render(self.ragdoll.orientation(anim.root) * link.axis.inverse());
        let position = to_render(self.ragdoll.position(anim.root)) - rotation * link.centre;
        let full = rotation * bone.rotation.inverse();
        let level = (Quat::from_rotation_arc((full * Vec3::Z).normalize_or(Vec3::Z), Vec3::Z) * full).normalize();
        let align = Transform::from_rt(level, position - level * bone.translation);
        log::info!("rider fall: the get-up begins");
        anim.getup = Some(GetUp { align });
    }

    /// Whether the rider may be put back on the bike after `waited` seconds of waiting for it: at once when there is no animation
    /// to wait for, when the get-up has played, or when he has waited too long.
    pub fn may_return(&self, waited: f32) -> bool {
        let Some(anim) = &self.anim else { return true };
        match (&anim.getup, anim.done) {
            (Some(_), done) => done || waited >= RETURN_LIMIT,
            (None, _) => waited >= RETURN_GRACE,
        }
    }

    /// The component transforms of the get-up (render space, relative to `origin`), if he is in it.
    pub fn get_up_pose(&self, skeleton: &Skeleton, origin: Vec3) -> Option<Vec<Transform>> {
        let anim = self.anim.as_ref()?;
        let getup = anim.getup?;
        let mut component = Vec::new();
        skeleton.component_transforms(&anim.target, &mut component).ok()?;
        Some(
            component
                .into_iter()
                .map(|t| {
                    let world = getup.align.compose(t);
                    Transform { translation: world.translation - origin, ..world }
                })
                .collect(),
        )
    }

    /// Whether the body is no longer simulated.
    pub fn frozen(&self) -> bool {
        self.anim.as_ref().is_some_and(|anim| anim.getup.is_some())
    }

    /// The local transform bone `i` has when it follows its parent: the animation's, or the riding pose's without one.
    pub fn local(&self, i: usize) -> Transform {
        self.anim.as_ref().map_or(self.locals[i], |anim| anim.target[i])
    }
}

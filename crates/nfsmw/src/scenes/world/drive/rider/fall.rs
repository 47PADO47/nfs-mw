//! The rider coming off the bike in a fall (`docs/specs/ride6-rider-fall.md`).
//!
//! The bike's dynamics hand the rider over at the fall ([`RiderRelease`]). From then on he is a ragdoll: one capsule body per bone
//! of the rider's physics asset (`ragdoll.json`), joined with the asset's limits, thrown with the velocity the bike had, falling
//! and sliding over the ground ([`blackbox_vehicle::ragdoll`]). Each frame the simulated bones take the bodies' places and what
//! hangs on them follows its parent as it was in the riding pose, which is how a limp body moves under physical animation. When
//! the bike is stood up again he is put back on it with a short blend. A rider without `ragdoll.json` keeps to his bike.

use blackbox_skeleton::{Skeleton, Transform};
use blackbox_vehicle::Ground;
use blackbox_vehicle::bike::{FallKind, RiderRelease};
use blackbox_vehicle::ragdoll::{Joint, JointLimits, Ragdoll, RagdollSpec, Segment};
use game_install::GameDir;
use glam::{Mat4, Quat, Vec3};

use super::fall_file::{self, FallConfig, Falls, RagdollFile};
use super::fall_space::{quat_to_physics, quat_to_render, to_physics, to_render};

/// How long the rider takes to get back on the bike once it is stood up (s). A modelling choice.
const RETURN_TIME: f32 = 0.25;

/// A body of the ragdoll and the bone it moves.
#[derive(Debug, Clone, Copy)]
struct Link {
    bone: usize,
    /// The capsule's centre in the bone's frame (render axes), and the rotation from the body's frame (axis along Y, physics
    /// axes) to the bone's frame.
    centre: Vec3,
    axis: Quat,
}

/// A rider in the air and on the ground.
#[derive(Debug, Clone)]
struct Tumble {
    ragdoll: Ragdoll,
    links: Vec<Link>,
    /// The bones' local transforms in the riding pose the fall began from: what hangs on a body follows it like that.
    locals: Vec<Transform>,
    /// The rider space in the render space when the fall began: where a bone with no body and no parent stays.
    seat: Transform,
}

#[derive(Debug, Clone, Default)]
enum State {
    #[default]
    Riding,
    Down(Box<Tumble>),
    /// Back on the bike: the pose he left the ground in (render space, absolute) and how far the blend has got (0 to 1).
    Returning {
        from: Vec<Transform>,
        progress: f32,
    },
}

/// The rider's fall.
#[derive(Debug, Clone, Default)]
pub(super) struct Fall {
    file: Option<RagdollFile>,
    /// The riding pose of the last frame, rider space.
    riding: Vec<Transform>,
    state: State,
    origin: Vec3,
}

impl Fall {
    /// A fall with the ragdoll of `file` (`None`: the rider keeps to his bike).
    pub fn new(file: Option<RagdollFile>) -> Self {
        Self { file, ..Self::default() }
    }

    /// The fall of the rider whose files are in `folder`: its `ragdoll.json`, if it has one that reads.
    pub fn load(dir: &GameDir, folder: &str) -> Self {
        let name = format!("{folder}/ragdoll.json");
        if !dir.exists(&name) {
            log::info!("rider: {folder} has no ragdoll.json; the rider keeps to his bike when it falls");
            return Self::default();
        }
        let file = dir.read(&name).map_err(anyhow::Error::from).and_then(|bytes| RagdollFile::parse(&bytes));
        match file {
            Ok(file) => Self::new(Some(file)),
            Err(error) => {
                log::warn!("rider: {name} is not used: {error:#}");
                Self::default()
            }
        }
    }

    /// Whether the rider is off the bike now (in the air, on the ground, or getting back on).
    pub fn active(&self) -> bool {
        !matches!(self.state, State::Riding)
    }

    /// Whether the rider is down (not yet getting back on).
    pub fn is_down(&self) -> bool {
        matches!(self.state, State::Down(_))
    }

    /// Whether the rider has a ragdoll to fall with.
    pub fn can_fall(&self) -> bool {
        self.file.is_some()
    }

    /// Remember the riding pose of this frame (rider space): the fall begins from the last one.
    pub fn remember(&mut self, component: &[Transform]) {
        self.riding.clear();
        self.riding.extend_from_slice(component);
    }

    /// The matrix that places the instance of the rider while he is off the bike.
    pub fn instance(&self) -> Mat4 {
        Mat4::from_translation(self.origin)
    }

    /// The rider is thrown off: build his ragdoll in the pose he had a frame ago (`to_world` takes the rider space to the render
    /// space, `bike` is the bike's frame in it) and give it the velocity of the release. False when there is nothing to throw.
    pub fn start(&mut self, skeleton: &Skeleton, (to_world, bike): (Mat4, Quat), release: &RiderRelease) -> bool {
        let Some(file) = &self.file else { return false };
        if self.riding.len() != skeleton.len() || self.is_down() {
            return false;
        }
        let Some(tumble) = Tumble::new(file, skeleton, &self.riding, (to_world, bike), release) else { return false };
        self.state = State::Down(Box::new(tumble));
        true
    }

    /// One physics step of the ragdoll (or of the blend back).
    pub fn step(&mut self, dt: f32, ground: &dyn Ground) {
        match &mut self.state {
            State::Down(tumble) => tumble.ragdoll.step(dt, ground),
            State::Returning { progress, .. } => {
                *progress += dt / RETURN_TIME;
                if *progress >= 1.0 {
                    self.state = State::Riding;
                }
            }
            State::Riding => {}
        }
    }

    /// The bike is stood up and the rider gets back on it.
    pub fn stop(&mut self, skeleton: &Skeleton) {
        let State::Down(tumble) = &self.state else { return };
        let from = tumble.pose(skeleton, self.origin).into_iter().map(|t| shifted(t, self.origin)).collect();
        self.state = State::Returning { from, progress: 0.0 };
    }

    /// The component transforms of the rider off the bike this frame, if he is, relative to the point [`Fall::instance`] places
    /// the instance at: from the ragdoll while he is down, a blend from where he lay to the riding pose `riding` (rider space,
    /// `to_world` the matrix that places the rider space on the bike) as he gets back on.
    pub fn pose(&mut self, skeleton: &Skeleton, riding: &[Transform], to_world: Mat4) -> Option<Vec<Transform>> {
        match &self.state {
            State::Riding => None,
            State::Down(tumble) => {
                let origin = to_render(tumble.ragdoll.centre());
                self.origin = origin;
                Some(tumble.pose(skeleton, origin))
            }
            State::Returning { from, progress } => {
                let (_, turn, seat) = to_world.to_scale_rotation_translation();
                let k = progress.clamp(0.0, 1.0);
                let k = k * k * (3.0 - 2.0 * k);
                let component = from
                    .iter()
                    .zip(riding)
                    .map(|(a, b)| {
                        let b = Transform::from_rt(turn * b.rotation, turn * b.translation + seat);
                        Transform {
                            rotation: a.rotation.slerp(b.rotation, k),
                            translation: a.translation.lerp(b.translation, k) - seat,
                            scale: a.scale.lerp(b.scale, k),
                        }
                    })
                    .collect();
                self.origin = seat;
                Some(component)
            }
        }
    }
}

/// A transform of the render space moved by `origin` (the opposite of taking it relative to an origin).
fn shifted(t: Transform, origin: Vec3) -> Transform {
    Transform { translation: t.translation + origin, ..t }
}

impl Tumble {
    /// The ragdoll for the bodies of `file` in the riding pose `riding`, thrown as `release` says.
    fn new(
        file: &RagdollFile,
        skeleton: &Skeleton,
        riding: &[Transform],
        (to_world, bike): (Mat4, Quat),
        release: &RiderRelease,
    ) -> Option<Self> {
        let (_, turn, seat) = to_world.to_scale_rotation_translation();
        let bone_world = |b: usize| (turn * riding[b].rotation, turn * riding[b].translation + seat);
        let mut links = Vec::new();
        let mut segments = Vec::new();
        let mut names = Vec::new();
        for body in &file.bodies {
            let Some(bone) = skeleton.find(&body.bone) else {
                log::warn!("rider: the ragdoll has a body on {}, which the rider does not have", body.bone);
                continue;
            };
            let (rotation, origin) = bone_world(bone);
            let (centre, axis) = (Vec3::from_array(body.centre), Vec3::from_array(body.axis).normalize());
            let to_y = Quat::from_rotation_arc(Vec3::Y, to_physics(axis));
            segments.push(Segment {
                mass: body.mass,
                radius: body.radius,
                half_length: body.half_length,
                position: to_physics(origin + rotation * centre),
                orientation: quat_to_physics(rotation) * to_y,
            });
            links.push(Link { bone, centre, axis: to_y });
            names.push(body.bone.as_str());
        }
        let index = |name: &str| names.iter().position(|n| *n == name);
        let mut joints = Vec::new();
        for joint in &file.joints {
            let (Some(child), Some(parent)) = (index(&joint.child), index(&joint.parent)) else { continue };
            let (Some(frame_child), Some(frame_parent)) =
                (fall_file::frame(joint.frame_child), fall_file::frame(joint.frame_parent))
            else {
                continue;
            };
            let (c, p) = (links[child], links[parent]);
            // The shared point is the child bone's origin, seen from each body.
            let (p_rotation, p_origin) = bone_world(p.bone);
            let in_parent = p_rotation.inverse() * (bone_world(c.bone).1 - p_origin);
            joints.push(Joint {
                parent,
                child,
                anchor_parent: p.axis.inverse() * (to_physics(in_parent) - to_physics(p.centre)),
                anchor_child: c.axis.inverse() * -to_physics(c.centre),
                frame_parent: p.axis.inverse() * quat_to_physics(frame_parent),
                frame_child: c.axis.inverse() * quat_to_physics(frame_child),
                limits: JointLimits {
                    swing_z: fall_file::limit(joint.swing_z_deg),
                    swing_y: fall_file::limit(joint.swing_y_deg),
                    twist: fall_file::limit(joint.twist_deg),
                },
            });
        }
        let spec = RagdollSpec {
            segments,
            joints,
            friction: file.contact_friction,
            linear_damping: file.linear_damping,
            angular_damping: file.angular_damping,
            joint_damping: file.joint_damping,
            ..RagdollSpec::default()
        };
        let (velocity, spin) = launch(release, &file.falls, bike);
        let ragdoll = Ragdoll::new(&spec, velocity, spin);
        let locals = (0..skeleton.len())
            .map(|i| match skeleton.bones()[i].parent {
                Some(parent) => riding[parent].inverse().compose(riding[i]),
                None => riding[i],
            })
            .collect();
        Some(Self { ragdoll, links, locals, seat: Transform::from_rt(turn, seat) })
    }

    /// The component transforms of the rider, relative to `origin`: the bones with a body where the body is, the others after
    /// their parents in the riding pose.
    fn pose(&self, skeleton: &Skeleton, origin: Vec3) -> Vec<Transform> {
        let mut out: Vec<Transform> = Vec::with_capacity(skeleton.len());
        for (i, bone) in skeleton.bones().iter().enumerate() {
            let simulated = self.links.iter().position(|l| l.bone == i);
            let transform = match (simulated, bone.parent) {
                (Some(body), _) => {
                    let link = self.links[body];
                    let rotation = quat_to_render(self.ragdoll.orientation(body) * link.axis.inverse());
                    let centre = to_render(self.ragdoll.position(body));
                    Transform::from_rt(rotation, centre - rotation * link.centre - origin)
                }
                (None, Some(parent)) => out[parent].compose(self.locals[i]),
                (None, None) => {
                    let placed = self.seat.compose(self.locals[i]);
                    Transform { translation: placed.translation - origin, ..placed }
                }
            };
            out.push(transform);
        }
        out
    }
}

/// What one physics step tells the rider about his fall: whether the bike let him go in this step (the release, once), whether he
/// is off the bike, and the length of the step.
pub struct FallStep {
    pub release: Option<RiderRelease>,
    pub off: bool,
    pub dt: f32,
}

impl super::RiderRig {
    /// One physics step of the fall: the rider is thrown when the bike lets him go, falls and slides while he is off it, and gets
    /// back on when it is stood up. `world` is the bike's frame in the render space.
    pub(super) fn fall_step(&mut self, step: FallStep, world: Mat4, ground: &dyn Ground) {
        let (_, bike, _) = world.to_scale_rotation_translation();
        if let Some(release) = step.release
            && self.fall.can_fall()
            && !self.fall.start(&self.pack.skeleton, (world * self.frame.to_model, bike), &release)
        {
            log::warn!("rider: the fall could not be started (the ragdoll does not fit the rider)");
        }
        if self.fall.is_down() && !step.off {
            self.fall.stop(&self.pack.skeleton);
        }
        self.fall.step(step.dt, ground);
    }
}

/// The velocity (render space turned to physics space) and the spin (physics space) the rider leaves with: the bike's, times the
/// multiplier, changed by the configuration of the kind of fall (`falls`, in the bike's frame `bike`); no spin for a fall that has
/// no configuration.
fn launch(release: &RiderRelease, falls: &Falls, bike: Quat) -> (Vec3, Vec3) {
    let velocity = to_render(release.velocity) * falls.velocity_multiplier;
    let config: Option<&FallConfig> = match release.kind {
        FallKind::HighSide => Some(&falls.high_side),
        FallKind::LowSide => Some(&falls.low_side),
        FallKind::Generic => None,
    };
    let Some(config) = config else { return (to_physics(velocity), Vec3::ZERO) };
    let local = bike.inverse() * velocity;
    let scaled = bike * (Vec3::from_array(config.scale) * local);
    let changed = velocity + bike * Vec3::from_array(config.additional) + Vec3::from_array(config.world) + scaled;
    (to_physics(changed), release.angular_velocity * config.angular)
}

#[cfg(test)]
#[path = "fall_tests.rs"]
mod tests;

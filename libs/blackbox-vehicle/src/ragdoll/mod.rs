//! A tumbling articulated body: capsule segments joined by swing-and-twist joints, falling and sliding over the ground.
//!
//! Generic and free of any game: the caller describes the segments (mass, size, pose), the joints (anchors, constraint frames,
//! limits) and the friction, gives the starting velocities and steps it. It is meant for a rider thrown off a bike, but a
//! crash-test dummy or a fallen pedestrian is the same thing. Physics space as the rest of the crate (x right, y up, z forward).
//!
//! The solver is position-based dynamics with sub-steps: each sub-step predicts the free motion, brings the joints' anchors
//! together, holds the joint angles to their limits, pushes the end spheres of the capsules out of the ground, takes the velocities from the
//! move and applies Coulomb friction to the contact points (see `body.rs`, `joint.rs` and `contact.rs`). Restitution is zero, as the
//! reference rider's material has none (`docs/specs/ride6-rider-fall.md`).
//!
//! ```
//! use blackbox_vehicle::ragdoll::{Ragdoll, RagdollSpec, Segment};
//! use blackbox_vehicle::{FIXED_STEP, FlatGround};
//! use glam::{Quat, Vec3};
//!
//! let segment = Segment { mass: 70.0, radius: 0.15, half_length: 0.3, position: Vec3::new(0.0, 1.0, 0.0), orientation: Quat::IDENTITY };
//! let spec = RagdollSpec { segments: vec![segment], joints: vec![], ..RagdollSpec::default() };
//! let mut body = Ragdoll::new(&spec, Vec3::new(0.0, 0.0, 10.0), Vec3::ZERO);
//! let ground = FlatGround::new(0.0);
//! for _ in 0..600 {
//!     body.step(FIXED_STEP, &ground);
//! }
//! assert!(body.position(0).z > 3.0 && body.at_rest());
//! ```

mod body;
mod contact;
mod joint;
#[cfg(test)]
mod tests;

pub use body::Segment;
pub use joint::{Joint, JointLimits};

use glam::{Quat, Vec3};

use crate::ground::Ground;
use body::Body;

/// Sub-steps per step. A modelling choice: 8 of a 60 Hz step is 480 Hz, where an 11-segment body with thin limbs is stable.
const SUBSTEPS: usize = 8;
/// How fast the room a joint was given beyond its limits at the start closes (1/s as a time constant, s). A modelling choice: a
/// rider seated on a bike has his joints bent beyond what the limits allow when he lies down, and they ease into range.
const RELAX_TIME: f32 = 0.3;
/// The speed under which a segment is still (m/s, counting the turn at its radius), and how long all must be still to be at rest.
const STILL_SPEED: f32 = 0.05;
const STILL_TIME: f32 = 0.5;

/// What a ragdoll is made of.
#[derive(Clone, Debug, PartialEq)]
pub struct RagdollSpec {
    pub segments: Vec<Segment>,
    pub joints: Vec<Joint>,
    /// The friction coefficient of the segments against the ground (before the ground's own multiplier).
    pub friction: f32,
    /// Damping of the linear and of the angular velocity (1/s).
    pub linear_damping: f32,
    pub angular_damping: f32,
    /// Gravity (m/s^2, signed).
    pub gravity: f32,
    /// The rate (1/s) at which the angular velocity of a joint's child against its parent decays: what keeps a limp limb from
    /// swinging for ever.
    pub joint_damping: f32,
}

impl Default for RagdollSpec {
    fn default() -> Self {
        Self {
            segments: Vec::new(),
            joints: Vec::new(),
            friction: 0.6,
            linear_damping: 0.01,
            angular_damping: 0.05,
            gravity: -9.81,
            joint_damping: 0.0,
        }
    }
}

/// A ragdoll in motion.
#[derive(Clone, Debug)]
pub struct Ragdoll {
    bodies: Vec<Body>,
    joints: Vec<Joint>,
    /// The room each joint still has beyond its limits (swing about Z, swing about Y, twist; rad).
    slack: Vec<Vec3>,
    friction: f32,
    damping: (f32, f32),
    joint_damping: f32,
    gravity: f32,
    still: f32,
    contacts: Vec<contact::Contact>,
}

impl Ragdoll {
    /// A ragdoll of `spec` in the poses the segments are given, all moving with the linear velocity `velocity` (the velocity of
    /// every point of the body, as for a body that was riding) and the angular velocity `spin` about the centre of mass of the
    /// whole (world frame).
    pub fn new(spec: &RagdollSpec, velocity: Vec3, spin: Vec3) -> Self {
        let mut bodies: Vec<Body> = spec.segments.iter().map(Body::new).collect();
        let mass: f32 = spec.segments.iter().map(|s| s.mass.max(1e-3)).sum();
        let centre = match mass > 0.0 {
            true => spec.segments.iter().map(|s| s.position * s.mass.max(1e-3)).sum::<Vec3>() / mass,
            false => Vec3::ZERO,
        };
        for body in &mut bodies {
            body.v = velocity + spin.cross(body.x - centre);
            body.w = spin;
        }
        let joints: Vec<Joint> = spec
            .joints
            .iter()
            .filter(|j| j.parent != j.child && j.parent < bodies.len() && j.child < bodies.len())
            .copied()
            .collect();
        let slack = joints
            .iter()
            .map(|j| {
                let (dev, _) = joint::deviation(&bodies, j);
                let limit = Vec3::new(j.limits.swing_z, j.limits.swing_y, j.limits.twist);
                (dev.abs() - limit).max(Vec3::ZERO)
            })
            .collect();
        Self {
            bodies,
            joints,
            slack,
            friction: spec.friction,
            damping: (spec.linear_damping, spec.angular_damping),
            joint_damping: spec.joint_damping,
            gravity: spec.gravity,
            still: 0.0,
            contacts: Vec::new(),
        }
    }

    /// Advance by `dt` seconds over `ground`.
    pub fn step(&mut self, dt: f32, ground: &dyn Ground) {
        if !dt.is_finite() || dt <= 0.0 {
            return;
        }
        let h = dt / SUBSTEPS as f32;
        // The room closes a little each sub-step: a step's worth at once would be a jolt the velocities then carry on.
        let relax = (-h / RELAX_TIME).exp();
        for _ in 0..SUBSTEPS {
            self.slack.iter_mut().for_each(|s| *s *= relax);
            self.bodies.iter_mut().for_each(|b| b.predict(h, self.gravity, self.damping));
            self.solve_joints();
            contact::resolve(&mut self.bodies, ground, self.friction, &mut self.contacts);
            self.solve_joints();
            self.bodies.iter_mut().for_each(|b| b.settle(h));
            contact::friction(&mut self.bodies, &self.contacts, h);
            let rate = 1.0 - (-self.joint_damping * h).exp();
            self.joints.iter().for_each(|joint| joint::damp(&mut self.bodies, joint, rate));
        }
        let moving = self.bodies.iter().any(|b| b.v.length() + b.w.length() * b.radius > STILL_SPEED);
        self.still = match moving {
            true => 0.0,
            false => self.still + dt,
        };
    }

    fn solve_joints(&mut self) {
        for (joint, slack) in self.joints.iter().zip(&self.slack) {
            joint::solve_point(&mut self.bodies, joint);
            joint::solve_limits(&mut self.bodies, joint, *slack);
        }
    }

    /// Add to the velocity and the angular velocity of segment `i` (an impulse the host gives it, divided by its mass).
    pub fn kick(&mut self, i: usize, velocity: Vec3, spin: Vec3) {
        let Some(body) = self.bodies.get_mut(i) else { return };
        body.v += velocity;
        body.w += spin;
    }

    /// The number of segments.
    pub fn len(&self) -> usize {
        self.bodies.len()
    }

    pub fn is_empty(&self) -> bool {
        self.bodies.is_empty()
    }

    /// The centre of mass of segment `i` (world).
    pub fn position(&self, i: usize) -> Vec3 {
        self.bodies[i].x
    }

    /// The orientation of segment `i`'s body frame (world).
    pub fn orientation(&self, i: usize) -> Quat {
        self.bodies[i].q
    }

    /// The velocity of the centre of mass of segment `i` (m/s).
    pub fn velocity(&self, i: usize) -> Vec3 {
        self.bodies[i].v
    }

    /// The centre of mass of the whole body (world).
    pub fn centre(&self) -> Vec3 {
        let mass: f32 = self.bodies.iter().map(|b| 1.0 / b.inv_mass).sum();
        self.bodies.iter().map(|b| b.x / b.inv_mass).sum::<Vec3>() / mass.max(f32::EPSILON)
    }

    /// The velocity of the centre of mass of the whole body (m/s).
    pub fn centre_velocity(&self) -> Vec3 {
        let mass: f32 = self.bodies.iter().map(|b| 1.0 / b.inv_mass).sum();
        self.bodies.iter().map(|b| b.v / b.inv_mass).sum::<Vec3>() / mass.max(f32::EPSILON)
    }

    /// Whether every segment has been still for a while.
    pub fn at_rest(&self) -> bool {
        self.still >= STILL_TIME
    }

    /// Whether the state is usable (no NaN).
    pub fn is_finite(&self) -> bool {
        self.bodies.iter().all(|b| b.x.is_finite() && b.q.is_finite() && b.v.is_finite() && b.w.is_finite())
    }

    /// How far each joint is beyond its limits now, the largest of the three angles (rad); 0 inside. Used by the tests.
    pub fn joint_excess(&self, i: usize) -> f32 {
        let Some(joint) = self.joints.get(i) else { return 0.0 };
        let (dev, _) = joint::deviation(&self.bodies, joint);
        let limit = Vec3::new(joint.limits.swing_z, joint.limits.swing_y, joint.limits.twist);
        (dev.abs() - limit).max(Vec3::ZERO).max_element()
    }

    /// How far apart the two anchors of joint `i` are (m).
    pub fn joint_gap(&self, i: usize) -> f32 {
        let Some(joint) = self.joints.get(i) else { return 0.0 };
        let (p, c) = (&self.bodies[joint.parent], &self.bodies[joint.child]);
        p.point(joint.anchor_parent).distance(c.point(joint.anchor_child))
    }
}

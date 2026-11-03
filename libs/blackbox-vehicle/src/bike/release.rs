//! The rider leaving the bike in a fall (`docs/specs/ride6-rider-fall.md`).
//!
//! At the moment the fall tests fire the rider lets go for good: his mass and his spring stop acting on the frame (the bike goes on
//! under its own mass, on its collision box and its tyres), he stops holding the levers and the bars, and he is handed to the host
//! as a [`RiderRelease`]: where his centre of mass is and how fast it moves, taken from the state of the bike a few steps before
//! the fall was noticed. The host throws him on the ground as a free body ([`crate::ragdoll`]). Placing the bike again puts him
//! back on it.

use glam::{Quat, Vec3};

use super::{Bike, rider};

/// How many steps before the fall was noticed the rider's velocity is taken from.
pub const VELOCITY_FRAMES: usize = 2;

/// How the bike was lost; the kinds of the fall tests (`ride6-chassis-round6.md`, section 3).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum FallKind {
    /// Anything the other two do not describe: a lean or a pitch in the air, a collision.
    #[default]
    Generic,
    /// The bike went down on its side: too much lean, too fast a roll.
    LowSide,
    /// The bike's heading and its velocity parted far enough that the tyre gripped again and threw it over.
    HighSide,
}

/// The rider as the bike lets him go.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RiderRelease {
    pub kind: FallKind,
    /// The rider's centre of mass (world) and the velocity (m/s) and angular velocity (rad/s) of the bike at his place, from
    /// [`VELOCITY_FRAMES`] steps before the fall was noticed.
    pub position: Vec3,
    pub velocity: Vec3,
    pub angular_velocity: Vec3,
    /// The bike's orientation when he left it.
    pub orientation: Quat,
}

/// What the bike remembers of the rider's leaving.
#[derive(Clone, Copy, Debug, Default)]
pub(super) struct Departure {
    /// The rider is off the bike.
    pub off: bool,
    pub kind: FallKind,
    /// The release the host has not taken yet.
    pub pending: Option<RiderRelease>,
    /// The velocity of the centre of gravity and the angular velocity of the last two steps, the older first.
    history: [(Vec3, Vec3); VELOCITY_FRAMES],
}

impl Bike {
    /// The rider as the bike let him go in the fall that just happened, once; `None` otherwise.
    pub fn take_rider_release(&mut self) -> Option<RiderRelease> {
        self.departure.pending.take()
    }

    /// Whether the rider is off the bike (from the fall until the bike is placed again).
    pub fn rider_off(&self) -> bool {
        self.departure.off
    }

    /// How the bike was lost, if it was.
    pub fn fall_kind(&self) -> Option<FallKind> {
        self.departure.off.then_some(self.departure.kind)
    }

    /// Remember the motion of this step, for the next fall's release.
    pub(super) fn remember_motion(&mut self) {
        let h = &mut self.departure.history;
        h[0] = h[1];
        h[1] = (self.body.linear_velocity, self.body.angular_velocity);
    }

    /// The rider lets go: out of the bike's mass and centre of gravity, and into a [`RiderRelease`].
    pub(super) fn release_rider(&mut self, kind: FallKind) {
        if self.departure.off {
            return;
        }
        let rotation = self.body.rotation();
        let offset = rider::rider_position(&self.spec.chassis, self.placement) + self.rider_body.deflection;
        let place = self.body.position + rotation * offset;
        // The velocity the bike had at his place, a few steps back (the reference takes the buffered state of two frames before).
        let (velocity, spin) = self.departure.history[0];
        let arm = place - self.body.world_cog();
        let release = RiderRelease {
            kind,
            position: place,
            velocity: velocity + spin.cross(arm),
            angular_velocity: spin,
            orientation: self.body.orientation,
        };
        self.departure = Departure { off: true, kind, pending: Some(release), ..self.departure };
        // The bike alone: its mass and its centre of gravity. The point of the frame that is the new centre of gravity keeps the
        // velocity it had.
        let old = self.body.cog();
        let new = rider::bike_cog(&self.spec.chassis);
        self.body.linear_velocity += self.body.angular_velocity.cross(rotation * (new - old));
        self.body.set_cog(new);
        self.body.set_mass(self.spec.chassis.mass);
        self.rider_body.reset();
        self.rider_rotation.reset();
    }

    /// The rider is back on the bike (it was placed again): its mass and centre of gravity are the two together.
    pub(super) fn restore_rider(&mut self) {
        self.departure = Departure::default();
        self.body.set_mass(self.spec.total_mass());
        self.body.set_cog(rider::combined_cog(&self.spec.chassis, self.spec.rider.base_offset, Vec3::ZERO));
    }

    /// With nobody holding the bars the handlebar swings back to the middle.
    pub(super) fn free_handlebar(&mut self, dt: f32) {
        let steering = &self.spec.steering;
        let angle = self.steer.step(0.0, steering.response, steering.damping_ratio, dt);
        let limit = steering.max_angle;
        if angle.abs() > limit {
            self.steer.reset(angle.clamp(-limit, limit));
        }
    }
}

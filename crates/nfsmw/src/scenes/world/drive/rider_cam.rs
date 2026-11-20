//! A bike's cameras on its thrown rider, and the grace after a reset (`viewer/camera/rider_follow.rs`,
//! `docs/specs/ride6-bike-cameras.md` section 8). Every physics step the rider's place is given to the follower; the chase camera,
//! the original views and the helmet camera read its blend.

use glam::Vec3;

use super::rider::Rider;
use super::{CarPose, Drive};
use crate::viewer::camera::original::BodyInput;
use crate::viewer::camera::rider_follow::{Bike, Phase, RiderCam};
use crate::viewer::camera::{Followed, direction};

/// How far above the rider's pelvis the original views anchor while they follow him (m): the original views put the camera `h`
/// above an anchor that sits at a bike's centre of gravity, about 0.35 m over a lying rider's pelvis. A modelling choice, picked in
/// offscreen pictures.
const ORIGINAL_ANCHOR_RAISE_M: f32 = 0.35;

/// One physics step of the follower: the rider's phase from the fall (`off`: the bike's dynamics say he is off it) and his place.
pub(super) fn step(cam: &mut RiderCam, rider: &Rider, off: bool, poses: (&CarPose, &CarPose), dt: f32) {
    let (previous, current) = poses;
    let located = rider.off_bike();
    let phase = match (located, off) {
        (Some((phase, _)), _) => phase,
        (None, true) => Phase::Fallen,
        (None, false) => Phase::Riding,
    };
    let forward = current.rotation * Vec3::X;
    let bike = Bike {
        position: current.position,
        heading: forward.y.atan2(forward.x),
        velocity: (current.position - previous.position) / dt,
    };
    cam.step(phase, located.map(|(_, point)| point), bike, dt);
}

impl Drive {
    /// How much the cameras are on the rider rather than the bike, 0 to 1 (always 0 for a car and a rider who is on the bike).
    pub fn rider_follow(&self) -> f32 {
        self.rider_cam.weight()
    }

    /// Whether the cameras are on the rider at all, even partly.
    pub fn rider_followed(&self) -> bool {
        self.rider_cam.active()
    }

    /// Whether the player cannot be busted: his bike's rider is off it, or it has just been reset or remounted
    /// (`RiderCam::bust_grace`). Never for a car.
    pub fn bust_grace(&self) -> bool {
        self.rider_cam.bust_grace()
    }

    /// A reset by hand of a bike (or its flip reset): the grace starts. Nothing for a car.
    pub(super) fn note_bike_reset(&mut self) {
        if self.sim.as_ref().is_some_and(|sim| sim.is_bike()) {
            self.rider_cam.note_reset();
        }
    }

    /// The bike was placed again and the camera is cut to it: the follower goes back to the bike at once.
    pub(super) fn cut_rider_cam(&mut self) {
        self.rider_cam.cut_to_bike();
    }

    /// What the custom chase camera follows this step: the bike, or the rider (his pelvis, his heading) as much as the follower
    /// says.
    pub(super) fn followed(&self) -> Followed {
        let pose = self.current;
        let forward = pose.rotation * Vec3::X;
        let bike = forward.y.atan2(forward.x);
        Followed {
            position: self.rider_cam.chase_point(pose.position, self.chase.fit.look_height),
            heading: self.rider_cam.heading(bike),
            speed: self.telemetry.speed_mps * (1.0 - self.rider_cam.weight()),
        }
    }

    /// The bike's body as the original views take it, moved towards the rider as the follower says (`alpha` between the last two
    /// steps for the frame drawn, none for the physics' own view).
    pub(super) fn blend_rider_body(&self, body: &mut BodyInput, alpha: Option<f32>) {
        let Some(focus) = self.rider_cam.focus_at(alpha.unwrap_or(1.0)) else { return };
        let weight = self.rider_cam.weight();
        let bike = body.forward.y.atan2(body.forward.x);
        let ahead = direction(self.rider_cam.heading(bike), 0.0);
        let anchor = focus + Vec3::Z * ORIGINAL_ANCHOR_RAISE_M;
        body.position = body.position.lerp(anchor, weight);
        body.forward = body.forward.lerp(ahead, weight).normalize_or(ahead);
        body.up = body.up.lerp(Vec3::Z, weight).normalize_or(Vec3::Z);
        body.velocity = body.velocity.lerp(self.rider_cam.velocity(), weight);
        body.speed *= 1.0 - weight;
        body.drift *= 1.0 - weight;
    }
}

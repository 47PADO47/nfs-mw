//! What the host tells the camera model each frame, and what it gets back.
//!
//! **Space.** Right-handed, metres, seconds: **x forward, y left, z up** (the mod ABI's `BbVec3` space). A level car
//! pointing along +x has `forward = [1, 0, 0]` and `up = [0, 0, 1]`. Nothing here knows the 2012 axes.

use crate::math::{WORLD_UP, finite_f32, finite_or, normalize_or};
use crate::params::Vec3;

/// The longest frame time the model integrates; a longer one (a hitch, a pause) is cut to this.
pub const MAX_DT: f32 = 1.0 / 20.0;

/// The car for one rendered frame, read after the car has moved.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CarMotion {
    /// The car's reference point on the ground (the origin of the 2012 car data: between the axles, at ground
    /// level), world metres.
    pub position: Vec3,
    /// Unit vector along the car's nose.
    pub forward: Vec3,
    /// Unit vector out of the car's roof.
    pub up: Vec3,
    /// World velocity in m/s.
    pub velocity: Vec3,
    /// Speed in m/s (>= 0). Not derived from `velocity` so the host can pass a signed-free gauge speed.
    pub speed: f32,
    /// Steering input, -1..1, **positive = left**.
    pub steer: f32,
    /// Throttle, 0..1.
    pub throttle: f32,
    /// Brake, 0..1.
    pub brake: f32,
    /// Yaw rate of the car about world up in rad/s, positive = turning left.
    pub yaw_rate: f32,
    /// True while the wheels are on the ground.
    pub grounded: bool,
    /// Collision strength this frame, 0..1 (0 = none). Starts the camera's impact shake.
    pub impact: f32,
    /// This frame's duration in seconds; the model cuts it to [`MAX_DT`].
    pub dt: f32,
}

impl CarMotion {
    /// A car standing still at the origin, level and pointing along +x: a base for tests and hosts.
    pub fn at_rest() -> Self {
        Self {
            position: [0.0; 3],
            forward: [1.0, 0.0, 0.0],
            up: WORLD_UP,
            velocity: [0.0; 3],
            speed: 0.0,
            steer: 0.0,
            throttle: 0.0,
            brake: 0.0,
            yaw_rate: 0.0,
            grounded: true,
            impact: 0.0,
            dt: 1.0 / 60.0,
        }
    }

    /// The same with every number made safe: finite, unit vectors, inputs in range, `dt` in `0..=MAX_DT`.
    pub(crate) fn sanitized(&self) -> Self {
        let forward = normalize_or(finite_or(self.forward, [1.0, 0.0, 0.0]), [1.0, 0.0, 0.0]);
        let up = normalize_or(finite_or(self.up, WORLD_UP), WORLD_UP);
        Self {
            position: finite_or(self.position, [0.0; 3]),
            forward,
            up,
            velocity: finite_or(self.velocity, [0.0; 3]),
            speed: finite_f32(self.speed, 0.0).max(0.0),
            steer: finite_f32(self.steer, 0.0).clamp(-1.0, 1.0),
            throttle: finite_f32(self.throttle, 0.0).clamp(0.0, 1.0),
            brake: finite_f32(self.brake, 0.0).clamp(0.0, 1.0),
            yaw_rate: finite_f32(self.yaw_rate, 0.0),
            grounded: self.grounded,
            impact: finite_f32(self.impact, 0.0).clamp(0.0, 1.0),
            dt: finite_f32(self.dt, 0.0).clamp(0.0, MAX_DT),
        }
    }
}

/// A camera pose for the renderer, in the same host space as [`CarMotion`].
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CameraPose {
    pub position: Vec3,
    /// Unit view direction.
    pub forward: Vec3,
    /// Unit vector out of the top of the picture, perpendicular to `forward`.
    pub up: Vec3,
    /// Vertical field of view in degrees, for the aspect ratio the model was built with.
    pub vertical_fov_deg: f32,
    /// How hard the camera is buffeted by speed, 0..1 (for audio or extra effects; already applied to the pose).
    pub shake: f32,
}

/// Which camera the model shows.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum CameraView {
    /// The chase camera behind the car.
    #[default]
    Chase,
    /// The in-car view (what the 2012 game calls bonnet).
    Bonnet,
    /// The bumper view.
    Bumper,
    /// The rear-view mirror's camera, looking back.
    RearView,
}

impl CameraView {
    pub const ALL: [CameraView; 4] = [Self::Chase, Self::Bonnet, Self::Bumper, Self::RearView];
}

/// Knobs of the model that are not in the car data: the host's screen, and the scales of effects whose strength the
/// data does not give in physical units. **[unconfirmed]** defaults.
#[derive(Debug, Clone, PartialEq)]
pub struct CameraOptions {
    /// Width over height of the picture the field of view is for (the data's field of view is horizontal at 16:9).
    pub aspect: f32,
    /// Degrees of shake per unit of the data's high-speed shake amplitude at full speed.
    pub shake_scale_deg: f32,
    /// Shake frequency in Hz when the data's shake rate is 1.
    pub shake_base_hz: f32,
    /// Time constant (s) that smooths the acceleration estimated from speed changes.
    pub accel_filter_s: f32,
    /// How far the car extends behind its reference point, for the rear-view camera (metres).
    pub rear_extent_m: f32,
    /// Added to the bonnet camera's eye and target, host axes relative to the car (e.g. from the car's own model).
    pub bonnet_anchor: Vec3,
    /// Added to the bumper camera's eye and target.
    pub bumper_anchor: Vec3,
}

impl Default for CameraOptions {
    fn default() -> Self {
        Self {
            aspect: 16.0 / 9.0,
            shake_scale_deg: 0.25,
            shake_base_hz: 8.0,
            accel_filter_s: 0.15,
            rear_extent_m: 2.2,
            bonnet_anchor: [0.0; 3],
            bumper_anchor: [0.0; 3],
        }
    }
}

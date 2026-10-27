//! The cameras that ride on the car: the bonnet (in-car) view, the bumper view and the rear-view mirror camera.
//!
//! Their data (`CameraGameplayBonnet`, `CameraGameplayBumper`, `CameraRearView`) holds a mount position, a point to
//! look at, two field-of-view numbers and a few small dynamics numbers; the roles are **[unconfirmed]** readings (see
//! `docs/formats/chameleon-camera.md`). The model mounts the camera rigidly in the car frame, shifts it a little
//! under acceleration (`sway_m`), widens the field of view with speed, and reports the wind buffeting as a value.

use crate::effects::AccelFilter;
use crate::math::{MPS_TO_MPH, WORLD_UP, add, cross, lerp, normalize_or, ramp, scale, sub, vertical_fov_deg};
use crate::motion::{CameraOptions, CameraPose, CarMotion};
use crate::params::{MountedCamera, RearView, Vec3};

/// A camera fixed in the car's frame: the bonnet or the bumper view.
#[derive(Debug, Clone)]
pub struct MountedRig {
    camera: MountedCamera,
    anchor: Vec3,
    /// The speed in mph where the data's speed effects reach full (the chase camera's `full_speed_mph`).
    full_speed_mph: f32,
    options: CameraOptions,
    accel: AccelFilter,
}

impl MountedRig {
    /// `anchor` is added to the eye and the target (host axes, relative to the car); pass `[0.0; 3]` for none.
    pub fn new(camera: &MountedCamera, anchor: Vec3, full_speed_mph: f32, options: &CameraOptions) -> Self {
        Self {
            camera: camera.clone(),
            anchor,
            full_speed_mph: full_speed_mph.max(1.0),
            options: options.clone(),
            accel: AccelFilter::default(),
        }
    }

    /// Forget the acceleration history and return the rest pose.
    pub fn reset(&mut self, motion: &CarMotion) -> CameraPose {
        let m = motion.sanitized();
        self.accel.reset(m.speed);
        self.pose(&m)
    }

    /// Advance by one frame and return the pose.
    pub fn step(&mut self, motion: &CarMotion) -> CameraPose {
        let m = motion.sanitized();
        self.accel.update(m.speed, m.dt, self.options.accel_filter_s);
        self.pose(&m)
    }

    /// The wind buffeting for a speed in m/s: 0 at rest, rising with the square of the speed up to the data's
    /// `wind_shake` at the full speed. The host may use it for a shake or a sound.
    pub fn wind_shake(&self, speed_mps: f32) -> f32 {
        let speed01 = ramp(speed_mps.max(0.0) * MPS_TO_MPH, 0.0, self.full_speed_mph);
        self.camera.wind_shake * speed01 * speed01
    }

    fn pose(&self, m: &CarMotion) -> CameraPose {
        let (forward, left, up) = car_frame(m);
        let speed01 = ramp(m.speed * MPS_TO_MPH, 0.0, self.full_speed_mph);
        // Sway: the head (camera) lags the car, so it moves back when the car speeds up and forward when it brakes.
        let full_accel = 10.0;
        let sway = -self.camera.sway_m * (self.accel.value() / full_accel).clamp(-1.0, 1.0);
        let eye = add(self.camera.eye, self.anchor);
        let target = add(self.camera.target, self.anchor);
        let world = |local: Vec3| {
            add(m.position, add(add(scale(forward, local[0]), scale(left, local[1])), scale(up, local[2])))
        };
        let position = world([eye[0] + sway, eye[1], eye[2]]);
        let look = normalize_or(sub(world(target), position), forward);
        let side = normalize_or(cross(up, look), left);
        let [low, high] = self.camera.fov_range_deg();
        CameraPose {
            position,
            forward: look,
            up: normalize_or(cross(look, side), up),
            vertical_fov_deg: vertical_fov_deg(lerp(low, high, speed01), self.options.aspect),
            shake: self.wind_shake(m.speed),
        }
    }
}

/// The camera of the rear-view mirror: at the car's rear end, looking back.
#[derive(Debug, Clone)]
pub struct RearViewRig {
    camera: RearView,
    options: CameraOptions,
}

impl RearViewRig {
    pub fn new(camera: &RearView, options: &CameraOptions) -> Self {
        Self { camera: camera.clone(), options: options.clone() }
    }

    /// The mirror camera's pose: `options.rear_extent_m` behind the car's reference point and `offset[2]` above
    /// it, looking along the car's tail. The field of view is the data's (horizontal at the mirror's own aspect).
    pub fn pose(&self, motion: &CarMotion) -> CameraPose {
        let m = motion.sanitized();
        let (forward, left, up) = car_frame(&m);
        let behind = self.options.rear_extent_m - self.camera.offset[0];
        let mut position = add(m.position, scale(forward, -behind));
        position = add(position, add(scale(left, self.camera.offset[1]), scale(up, self.camera.offset[2])));
        let back = scale(forward, -1.0);
        CameraPose {
            position,
            forward: back,
            up,
            vertical_fov_deg: vertical_fov_deg(self.camera.fov_deg, self.camera.aspect),
            shake: 0.0,
        }
    }
}

/// The car's frame as orthonormal `(forward, left, up)`.
fn car_frame(m: &CarMotion) -> (Vec3, Vec3, Vec3) {
    let forward = m.forward;
    let left = normalize_or(cross(m.up, forward), cross(WORLD_UP, forward));
    let left = normalize_or(left, [0.0, 1.0, 0.0]);
    let up = normalize_or(cross(forward, left), WORLD_UP);
    (forward, left, up)
}

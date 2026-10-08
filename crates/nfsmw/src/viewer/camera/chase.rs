//! A camera that follows a car: it sits behind it, turns slowly with its heading, backs off and
//! widens its view with speed, and is pushed in by whatever stands between it and the car.

use glam::{Mat4, Vec3};

use super::{direction, view_proj};

/// Distance behind the car at rest and at [`FAST`], metres.
const NEAR: f32 = 5.8;
const FAR: f32 = 8.8;
/// Height above the car at rest and at speed, metres.
const LOW: f32 = 1.9;
const HIGH: f32 = 2.5;
/// Field of view at rest and at speed, degrees.
const FOV_SLOW: f32 = 60.0;
const FOV_FAST: f32 = 78.0;
/// The speed (m/s) at which the camera is fully backed off and widened (about 250 km/h).
const FAST: f32 = 70.0;
/// How quickly the heading and the position catch up, per second.
const TURN_RATE: f32 = 4.0;
const FOLLOW_RATE: f32 = 14.0;
/// Where the camera looks: above the car's origin, and ahead of it by this much per m/s.
const LOOK_HEIGHT: f32 = 1.1;
const LOOK_AHEAD: f32 = 0.05;
/// Keep this far from walls and the ground.
const CLEARANCE: f32 = 0.35;

pub struct ChaseCamera {
    pub position: Vec3,
    target: Vec3,
    /// Heading the camera looks along, radians from +X.
    yaw: f32,
    fov_degrees: f32,
    placed: bool,
}

/// The car as the camera needs to know it.
#[derive(Debug, Clone, Copy)]
pub struct Followed {
    pub position: Vec3,
    /// Heading of the car, radians from +X.
    pub heading: f32,
    /// Speed in metres per second.
    pub speed: f32,
}

impl Default for ChaseCamera {
    fn default() -> Self {
        Self { position: Vec3::ZERO, target: Vec3::X, yaw: 0.0, fov_degrees: FOV_SLOW, placed: false }
    }
}

fn wrap(angle: f32) -> f32 {
    (angle + std::f32::consts::PI).rem_euclid(std::f32::consts::TAU) - std::f32::consts::PI
}

fn approach(rate: f32, dt: f32) -> f32 {
    1.0 - (-rate * dt).exp()
}

impl ChaseCamera {
    pub fn fov_degrees(&self) -> f32 {
        self.fov_degrees
    }

    pub fn forward(&self) -> Vec3 {
        (self.target - self.position).normalize_or_zero()
    }

    /// Jump to the car (after a teleport) instead of swinging round to it.
    pub fn snap(&mut self) {
        self.placed = false;
    }

    /// Follow `car` for `dt` seconds. `blocked(from, to)` says how far along the segment (0..1) the
    /// first obstacle is, if any.
    pub fn update(&mut self, car: Followed, dt: f32, blocked: impl Fn(Vec3, Vec3) -> Option<f32>) {
        let speed01 = (car.speed / FAST).clamp(0.0, 1.0);
        let look = car.position + Vec3::Z * LOOK_HEIGHT;
        if !self.placed {
            self.yaw = car.heading;
        } else {
            self.yaw += wrap(car.heading - self.yaw) * approach(TURN_RATE, dt);
        }
        let behind = -direction(self.yaw, 0.0);
        let distance = NEAR + (FAR - NEAR) * speed01;
        let desired = look + behind * distance + Vec3::Z * (LOW + (HIGH - LOW) * speed01 - LOOK_HEIGHT);
        self.position = if self.placed { self.position.lerp(desired, approach(FOLLOW_RATE, dt)) } else { desired };
        self.placed = true;

        // Pushed in by walls and buildings between the car and the camera, and kept above the road.
        let length = (self.position - look).length();
        if let Some(t) = blocked(look, self.position)
            && length > 0.0
        {
            let kept = (t * length - CLEARANCE).max(1.5);
            self.position = look + (self.position - look) / length * kept;
        }
        if let Some(t) = blocked(self.position + Vec3::Z * 2.0, self.position - Vec3::Z * CLEARANCE) {
            self.position.z = self.position.z + 2.0 - t * (2.0 + CLEARANCE) + CLEARANCE;
        }

        self.target = look + direction(car.heading, 0.0) * (car.speed * LOOK_AHEAD);
        self.fov_degrees = FOV_SLOW + (FOV_FAST - FOV_SLOW) * speed01;
    }

    pub fn view_proj(&self, aspect: f32) -> Mat4 {
        view_proj(self.position, self.target, self.fov_degrees, aspect, 0.3)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn car(heading: f32, speed: f32) -> Followed {
        Followed { position: Vec3::new(100.0, 50.0, 10.0), heading, speed }
    }

    fn open(_: Vec3, _: Vec3) -> Option<f32> {
        None
    }

    #[test]
    fn starts_behind_the_car() {
        let mut cam = ChaseCamera::default();
        cam.update(car(0.0, 0.0), 1.0 / 60.0, open);
        assert!(cam.position.x < 100.0 - 5.0 && (cam.position.y - 50.0).abs() < 1e-3);
        assert!(cam.position.z > 10.0);
        assert!(cam.forward().x > 0.9, "looks at the car along its heading");
    }

    #[test]
    fn swings_round_slowly_and_follows_speed() {
        let mut cam = ChaseCamera::default();
        cam.update(car(0.0, 0.0), 1.0 / 60.0, open);
        let rest = (cam.position - Vec3::new(100.0, 50.0, 10.0)).length();
        // The car turns left a quarter turn: after one frame the camera has hardly moved round.
        cam.update(car(std::f32::consts::FRAC_PI_2, 0.0), 1.0 / 60.0, open);
        assert!(cam.position.y - 50.0 < 1.0);
        for _ in 0..600 {
            cam.update(car(std::f32::consts::FRAC_PI_2, 70.0), 1.0 / 60.0, open);
        }
        // Now it is behind the car (at lower y), further back, with a wider view.
        assert!(cam.position.y < 50.0 - 7.0 && (cam.position.x - 100.0).abs() < 0.5, "{:?}", cam.position);
        assert!((cam.position - Vec3::new(100.0, 50.0, 10.0)).length() > rest + 2.0);
        assert!((cam.fov_degrees() - FOV_FAST).abs() < 1e-3);
    }

    #[test]
    fn heading_wraps_the_short_way() {
        let mut cam = ChaseCamera::default();
        let near_pi = std::f32::consts::PI - 0.1;
        cam.update(car(near_pi, 0.0), 1.0 / 60.0, open);
        cam.update(car(-near_pi, 0.0), 1.0 / 60.0, open);
        // The shortest way from +3.04 to -3.04 passes through pi, not through zero.
        assert!(wrap(cam.yaw - near_pi).abs() < 0.1, "yaw {}", cam.yaw);
    }

    #[test]
    fn walls_push_the_camera_in() {
        let mut cam = ChaseCamera::default();
        // Something solid halfway between the car and where the camera wants to be.
        let wall = |from: Vec3, to: Vec3| (from.z > 5.0 && (to - from).length() > 4.0).then_some(0.5);
        cam.update(car(0.0, 0.0), 1.0 / 60.0, wall);
        let d = (cam.position - Vec3::new(100.0, 50.0, 11.1)).length();
        assert!((1.5..4.0).contains(&d), "{d}");
    }

    #[test]
    fn snap_replaces_the_smoothing() {
        let mut cam = ChaseCamera::default();
        cam.update(car(0.0, 0.0), 1.0 / 60.0, open);
        cam.snap();
        let far = Followed { position: Vec3::new(5000.0, 0.0, 10.0), heading: 1.0, speed: 0.0 };
        cam.update(far, 1.0 / 60.0, open);
        assert!((cam.position.x - 5000.0).abs() < 10.0);
    }
}

//! A camera that follows a car: it sits behind it, turns slowly with its heading, backs off and
//! widens its view with speed, and is pushed in by whatever stands between it and the car.

use glam::{Mat4, Vec3};

use super::{direction, view_proj};

/// Distance behind the car at rest and at [`FAST`], metres.
const NEAR: f32 = 5.6;
const FAR: f32 = 7.0;
/// Height above the car at rest and at speed, metres.
const LOW: f32 = 1.9;
const HIGH: f32 = 2.2;
/// Field of view at rest and at speed, degrees.
const FOV_SLOW: f32 = 60.0;
const FOV_FAST: f32 = 68.0;
/// The speed (m/s) at which the camera is fully backed off and widened (about 215 km/h); faster
/// does not go further.
const FAST: f32 = 60.0;
/// How quickly the heading and the offset from the car catch up, per second.
const TURN_RATE: f32 = 4.0;
const FOLLOW_RATE: f32 = 10.0;
/// How quickly the camera is pushed in by an obstacle, and how slowly it comes back out, per second.
const PUSH_IN_RATE: f32 = 9.0;
const PULL_OUT_RATE: f32 = 1.5;
/// Where the camera looks: above the car's origin, and ahead of it by this much per m/s.
const LOOK_HEIGHT: f32 = 1.1;
const LOOK_AHEAD: f32 = 0.04;
/// Radians of orbit per unit of look input (mouse pixels, or stick pixels per second times seconds).
const LOOK_SENSITIVITY: f32 = 0.003;
/// How high above the car's level the camera may be orbited, and how far below it, radians.
const LOOK_MAX_ELEVATION: f32 = 1.2;
const LOOK_MIN_ELEVATION: f32 = -0.15;
/// Seconds without look input before the camera swings back behind the car, and how quickly, per second.
const RECENTRE_AFTER: f32 = 1.2;
const RECENTRE_RATE: f32 = 3.0;
/// Keep this far from walls and the ground.
const CLEARANCE: f32 = 0.4;
/// The camera never comes closer to the car than this because of an obstacle.
const MIN_DISTANCE: f32 = 2.8;

pub struct ChaseCamera {
    pub position: Vec3,
    target: Vec3,
    /// Heading the camera looks along, radians from +X.
    yaw: f32,
    fov_degrees: f32,
    /// Where the camera wants to be relative to the car's look point, before obstacles.
    offset: Vec3,
    /// How much of that offset an obstacle leaves, 0..1: shrinks fast, recovers slowly.
    reach: f32,
    placed: bool,
    /// How far the player has orbited the camera round the car and up from behind it, radians.
    orbit_yaw: f32,
    orbit_elevation: f32,
    /// Seconds since the last look input.
    idle: f32,
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
        Self {
            position: Vec3::ZERO,
            target: Vec3::X,
            yaw: 0.0,
            fov_degrees: FOV_SLOW,
            offset: Vec3::ZERO,
            reach: 1.0,
            placed: false,
            orbit_yaw: 0.0,
            orbit_elevation: 0.0,
            idle: 0.0,
        }
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
        self.orbit_yaw = 0.0;
        self.orbit_elevation = 0.0;
    }

    /// Orbit the camera round the car by the player's look input (`x` turns it to the right of the
    /// view, `y` is positive downwards, both in the units of the fly camera's mouse look).
    /// With no input for a while the camera swings back behind the car.
    pub fn look(&mut self, x: f32, y: f32, dt: f32) {
        if x != 0.0 || y != 0.0 {
            self.idle = 0.0;
            self.orbit_yaw = wrap(self.orbit_yaw - x * LOOK_SENSITIVITY);
            self.orbit_elevation =
                (self.orbit_elevation + y * LOOK_SENSITIVITY).clamp(LOOK_MIN_ELEVATION, LOOK_MAX_ELEVATION);
            return;
        }
        self.idle += dt;
        if self.idle < RECENTRE_AFTER {
            return;
        }
        let back = approach(RECENTRE_RATE, dt);
        self.orbit_yaw -= self.orbit_yaw * back;
        self.orbit_elevation -= self.orbit_elevation * back;
    }

    /// Whether the player has the camera turned away from behind the car.
    #[cfg(test)]
    fn orbited(&self) -> bool {
        self.orbit_yaw.abs() > 0.02 || self.orbit_elevation.abs() > 0.02
    }

    /// Follow `car` for `dt` seconds. `blocked(from, to)` says how far along the segment (0..1) the
    /// first obstacle is, if any.
    ///
    /// The camera keeps a smoothed offset from the car, so it does not lag behind at speed; an
    /// obstacle between them shortens that offset quickly, but it grows back slowly, so passing a
    /// post or a rail never makes the camera jump in and out.
    pub fn update(&mut self, car: Followed, dt: f32, blocked: impl Fn(Vec3, Vec3) -> Option<f32>) {
        let speed01 = (car.speed.abs() / FAST).clamp(0.0, 1.0);
        let look = car.position + Vec3::Z * LOOK_HEIGHT;
        if !self.placed {
            self.yaw = car.heading;
        } else {
            self.yaw += wrap(car.heading - self.yaw) * approach(TURN_RATE, dt);
        }
        let behind = -direction(self.yaw + self.orbit_yaw, 0.0);
        let distance = NEAR + (FAR - NEAR) * speed01;
        let (up, flat) = self.orbit_elevation.sin_cos();
        let wanted =
            behind * (distance * flat) + Vec3::Z * (LOW + (HIGH - LOW) * speed01 - LOOK_HEIGHT + distance * up);
        self.offset = if self.placed { self.offset.lerp(wanted, approach(FOLLOW_RATE, dt)) } else { wanted };

        let length = self.offset.length();
        let allowed = match blocked(look, look + self.offset) {
            Some(t) if length > 0.0 => ((t * length - CLEARANCE).max(MIN_DISTANCE.min(length)) / length).min(1.0),
            _ => 1.0,
        };
        let rate = if allowed < self.reach { PUSH_IN_RATE } else { PULL_OUT_RATE };
        self.reach = if self.placed { self.reach + (allowed - self.reach) * approach(rate, dt) } else { allowed };
        self.placed = true;
        self.position = look + self.offset * self.reach;

        // Looking round the car, the camera keeps its eyes on the car rather than on the road ahead.
        let ahead = 1.0 - (self.orbit_yaw.abs() / 0.5).min(1.0);
        self.target = look + direction(car.heading, 0.0) * (car.speed.max(0.0) * LOOK_AHEAD * ahead);
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

    /// The camera's horizontal distance behind the car at full speed.
    const FAR_AT_FAST: f32 = FAR;

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
            cam.update(car(std::f32::consts::FRAC_PI_2, 60.0), 1.0 / 60.0, open);
        }
        // Now it is behind the car (at lower y), further back, with a wider view.
        assert!(cam.position.y < 50.0 - 6.5 && (cam.position.x - 100.0).abs() < 0.5, "{:?}", cam.position);
        assert!((cam.position - Vec3::new(100.0, 50.0, 10.0)).length() > rest + 1.0);
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
        assert!((MIN_DISTANCE - 0.1..4.0).contains(&d), "{d}");
    }

    #[test]
    fn speed_stops_widening_the_view_and_backing_off() {
        let run = |speed: f32| {
            let mut cam = ChaseCamera::default();
            for _ in 0..300 {
                cam.update(car(0.0, speed), 1.0 / 60.0, open);
            }
            ((cam.position - Vec3::new(100.0, 50.0, 10.0)).length(), cam.fov_degrees())
        };
        let (d_fast, fov_fast) = run(FAST);
        let (d_faster, fov_faster) = run(FAST * 3.0);
        assert!((d_fast - d_faster).abs() < 1e-3 && (fov_fast - fov_faster).abs() < 1e-3);
        assert!(d_fast < 8.5 && fov_fast <= 70.0, "{d_fast} m, {fov_fast} degrees");
    }

    #[test]
    fn the_camera_does_not_lag_behind_a_fast_car() {
        let mut cam = ChaseCamera::default();
        let mut x = 0.0;
        for _ in 0..300 {
            x += 60.0 / 60.0;
            let c = Followed { position: Vec3::new(x, 0.0, 10.0), heading: 0.0, speed: 60.0 };
            cam.update(c, 1.0 / 60.0, open);
        }
        let gap = x - cam.position.x;
        assert!((gap - FAR_AT_FAST).abs() < 0.3, "{gap} m behind at full speed");
    }

    #[test]
    fn a_passing_post_does_not_make_the_camera_jump() {
        let mut cam = ChaseCamera::default();
        for _ in 0..120 {
            cam.update(car(0.0, 10.0), 1.0 / 60.0, open);
        }
        let free = (cam.position - Vec3::new(100.0, 50.0, 10.0)).length();
        // A post stands in the way for three frames, very near the car.
        let mut steps = vec![];
        for frame in 0..180 {
            let post = (60..63).contains(&frame);
            cam.update(car(0.0, 10.0), 1.0 / 60.0, |_, _| post.then_some(0.2));
            steps.push((cam.position - Vec3::new(100.0, 50.0, 10.0)).length());
        }
        let jump = steps.windows(2).map(|w| (w[1] - w[0]).abs()).fold(0.0, f32::max);
        assert!(jump < 0.6, "the camera moved {jump} m in one frame");
        assert!(steps.iter().all(|&d| d >= MIN_DISTANCE - 1e-3), "never closer than the minimum");
        // It comes back out gently, not at once.
        assert!(steps[65] < free && steps[179] > steps[70]);
    }

    #[test]
    fn look_input_orbits_the_camera_and_it_swings_back() {
        let mut cam = ChaseCamera::default();
        let settle = |cam: &mut ChaseCamera, frames: usize| {
            for _ in 0..frames {
                cam.look(0.0, 0.0, 1.0 / 60.0);
                cam.update(car(0.0, 0.0), 1.0 / 60.0, open);
            }
        };
        settle(&mut cam, 120);
        // Mouse right by 520 pixels: a quarter turn or so about the car, to its left side.
        cam.look(520.0, 0.0, 1.0 / 60.0);
        settle(&mut cam, 2);
        assert!(cam.orbited());
        settle(&mut cam, 60);
        let rel = cam.position - Vec3::new(100.0, 50.0, 10.0);
        assert!(rel.x.abs() < rel.y.abs(), "now beside the car: {rel:?}");
        // Idle: after a few seconds it is behind the car again.
        settle(&mut cam, 600);
        assert!(!cam.orbited());
        assert!(cam.position.x < 100.0 - 5.0 && (cam.position.y - 50.0).abs() < 0.3, "{:?}", cam.position);
    }

    #[test]
    fn looking_down_lifts_the_camera_and_is_limited() {
        let mut cam = ChaseCamera::default();
        cam.update(car(0.0, 0.0), 1.0 / 60.0, open);
        let low = cam.position.z;
        cam.look(0.0, 10_000.0, 1.0 / 60.0);
        for _ in 0..120 {
            cam.update(car(0.0, 0.0), 1.0 / 60.0, open);
        }
        assert!(cam.position.z > low + 3.0, "{} -> {}", low, cam.position.z);
        assert!(cam.position.z < 10.0 + 1.1 + (LOW - LOOK_HEIGHT) + NEAR, "capped: {}", cam.position.z);
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

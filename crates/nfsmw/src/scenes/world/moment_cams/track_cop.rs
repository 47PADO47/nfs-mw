//! The cop-tracking camera of the pursuit-start shot: from the gameplay camera, the view turns to the cop that sees the
//! player and zooms onto it, swinging the eye round when the cop is behind it. Spec: `docs/specs/game-moment-camera.md`
//! section 4. Render space (x forward, y left, z up).

use glam::Vec3;

use super::Scene;

/// How fast the three paths run (share per second): the eye's in 0.78 s, the look's and the lens's in 1.31 s.
const EYE_RATE: f32 = 1.275;
const LOOK_RATE: f32 = 0.765;
/// Half the width the lens ends up keeping in frame at the cop (m), and the lens's limits (16-bit angle units).
const FRAME_HALF: f32 = 3.0;
const FOV_MIN: f32 = 1000.0;
const FOV_MAX: f32 = 13100.0;
/// Only cops nearer than this to the player are looked at (m).
pub const COP_RANGE: f32 = 300.0;
/// Where a swung eye ends: this far beyond the old look point, away from the cop (m)...
const SWING: f32 = 5.0;
/// ...at most this high above it, plus [`SWING_LIFT`].
const SWING_HEIGHT: f32 = 2.25;
const SWING_LIFT: f32 = 0.25;

/// A cubic Bézier path.
#[derive(Debug, Clone, Copy, PartialEq)]
struct Bezier([Vec3; 4]);

impl Bezier {
    fn still(at: Vec3) -> Self {
        Self([at; 4])
    }

    fn at(&self, t: f32) -> Vec3 {
        let [p0, p1, p2, p3] = self.0;
        let s = 1.0 - t;
        p0 * (s * s * s) + p1 * (3.0 * s * s * t) + p2 * (3.0 * s * t * t) + p3 * (t * t * t)
    }
}

/// The gameplay camera at the shot's start.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Camera {
    pub eye: Vec3,
    pub target: Vec3,
    /// 16-bit angle units.
    pub fov: f32,
}

/// A shot being played.
#[derive(Debug, Clone, PartialEq)]
pub struct TrackCop {
    eye: Bezier,
    look: Bezier,
    /// The lens path in degrees, carried in `x`.
    lens: Bezier,
    eye_t: f32,
    look_t: f32,
}

impl TrackCop {
    /// The shot from `camera` towards `cop`, or a still view of the camera when there is no cop to look at.
    pub fn start(camera: Camera, cop: Option<Vec3>, scene: &dyn Scene) -> Self {
        let degrees = |fov: f32| fov * 360.0 / 65536.0;
        let lens_now = Vec3::new(degrees(camera.fov), 0.0, 0.0);
        let mut shot = Self {
            eye: Bezier::still(camera.eye),
            look: Bezier::still(camera.target),
            lens: Bezier::still(lens_now),
            eye_t: 0.0,
            look_t: 0.0,
        };
        let Some(cop) = cop else { return shot };
        shot.look.0[3] = cop;
        shot.eye = swing(camera, cop, scene).unwrap_or(shot.eye);
        let distance = shot.eye.0[3].distance(cop);
        let fov = (2.0 * FRAME_HALF.atan2(distance) / std::f32::consts::TAU * 65536.0).clamp(FOV_MIN, FOV_MAX);
        shot.lens.0[3] = Vec3::new(degrees(fov), 0.0, 0.0);
        shot
    }

    /// Runs the paths `dt` seconds of real time.
    pub fn advance(&mut self, dt: f32) {
        self.eye_t = (self.eye_t + dt * EYE_RATE).min(1.0);
        self.look_t = (self.look_t + dt * LOOK_RATE).min(1.0);
    }

    /// The eye, the point looked at and the lens (16-bit angle units).
    pub fn view(&self) -> (Vec3, Vec3, f32) {
        let fov = self.lens.at(self.look_t).x * 65536.0 / 360.0;
        (self.eye.at(self.eye_t), self.look.at(self.look_t), fov)
    }
}

/// The eye's path when the cop is on the camera's side of the old look point: an arc round to a point beyond it, away
/// from the cop. `None` keeps the eye still (the camera only turns and zooms).
fn swing(camera: Camera, cop: Vec3, scene: &dyn Scene) -> Option<Bezier> {
    let back = camera.eye - camera.target;
    let away = camera.target - cop;
    let flat = Vec3::new(away.x, away.y, 0.0);
    let across = flat / flat.length().max(0.1) * SWING;
    let height = back.z.max(away.z).min(SWING_HEIGHT) + SWING_LIFT;
    let end = camera.target + Vec3::new(across.x, across.y, height);
    let cosine = back.normalize_or_zero().dot((end - camera.target).normalize_or_zero());
    if cosine >= 0.0 {
        return None;
    }
    let turn = (1.0 - cosine) * 0.5;
    let turn = match back.x * across.y - back.y * across.x < 0.0 {
        true => -turn,
        false => turn,
    };
    let first = camera.eye + Vec3::new(-back.y, back.x, 0.0) * turn;
    let second = end + Vec3::new(across.y, -across.x, 0.0) * turn;
    (scene.clear(first, camera.target) && scene.clear(second, camera.target))
        .then_some(Bezier([camera.eye, first, second, end]))
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Open;

    impl Scene for Open {
        fn clear(&self, _: Vec3, _: Vec3) -> bool {
            true
        }
        fn ground(&self, _: Vec3) -> Option<f32> {
            Some(0.0)
        }
        fn road_ahead(&self, _: Vec3, _: Vec3, _: f32) -> Option<(Vec3, [Vec3; 2])> {
            None
        }
    }

    /// A chase camera 6 m behind and 2 m above a car at the origin looking forward (+x).
    fn camera() -> Camera {
        Camera { eye: Vec3::new(-6.0, 0.0, 2.0), target: Vec3::ZERO, fov: 65.0 / 360.0 * 65536.0 }
    }

    #[test]
    fn a_cop_ahead_is_framed_by_turning_and_zooming_from_where_the_camera_is() {
        let mut shot = TrackCop::start(camera(), Some(Vec3::new(40.0, 5.0, 0.0)), &Open);
        shot.advance(2.0);
        let (eye, look, fov) = shot.view();
        assert_eq!(eye, camera().eye, "the eye stays: the cop is in front of the camera");
        assert_eq!(look, Vec3::new(40.0, 5.0, 0.0), "the look ends on the cop");
        let degrees = fov * 360.0 / 65536.0;
        let expected = 2.0 * (3.0f32).atan2(eye.distance(look)).to_degrees();
        assert!((degrees - expected).abs() < 0.05, "{degrees} vs {expected}");
    }

    #[test]
    fn a_cop_behind_swings_the_eye_round_beyond_the_car() {
        let mut shot = TrackCop::start(camera(), Some(Vec3::new(-40.0, 0.0, 0.0)), &Open);
        shot.advance(0.4);
        let (middle, ..) = shot.view();
        shot.advance(1.0);
        let (end, ..) = shot.view();
        assert!((end - Vec3::new(5.0, 0.0, 2.25)).length() < 1e-4, "5 m beyond the car, away from the cop: {end:?}");
        assert!(middle.y.abs() > 0.5, "the eye arcs round the side, not through the car: {middle:?}");
    }

    #[test]
    fn with_no_cop_the_view_holds() {
        let mut shot = TrackCop::start(camera(), None, &Open);
        shot.advance(1.0);
        let (eye, look, fov) = shot.view();
        assert_eq!((eye, look), (camera().eye, camera().target));
        assert!((fov - camera().fov).abs() < 1e-2);
    }
}

//! The car-tracking camera of the jump and crash shots: a fixed eye placed off the car's path ahead, looking at the car
//! with a lens that keeps about 6.6 m of it in frame. Spec: `docs/specs/game-moment-camera.md` section 3. Render space
//! (x forward, y left, z up).

use glam::Vec3;

use super::Scene;

/// The world runs at this share of real time while the shot plays.
pub const SIM_SPEED: f32 = 0.2;
/// Half the width the lens keeps in frame at the car (m).
const FRAME_HALF: f32 = 3.3;
/// The lens's limits, 16-bit angle units (about 4.4 and 72 degrees).
const FOV_MIN: f32 = 800.0;
const FOV_MAX: f32 = 13100.0;
/// The longest lead the eye may be placed ahead of the car (m).
const MAX_LEAD: f32 = 150.0;
/// The second prediction, for the eye beside the path (s).
const SIDE_LEAD: f32 = 1.5;
/// How far beside the predicted car the side eyes stand (m), and their height above the ground.
const SIDE_OFFSET: f32 = 4.0;
const SIDE_HEIGHT: f32 = 2.0;
/// A new eye must stand farther than this from the last shot's (m).
const NEW_EYE: f32 = 3.0;
/// The eye is raised this much at the end (m).
const EYE_LIFT: f32 = 0.5;

/// Why the shot plays, which picks its framing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Framing {
    /// A jump or a crash outside a chase.
    Plain = 0,
    /// The player is being chased.
    Pursued = 1,
    /// A crash into a roadblock.
    Roadblock = 2,
}

/// Per framing: how far ahead (s) the eye is placed along the road, how high above the ground, and the point looked at
/// in the car's axes (forward, left, up).
const LEAD: [f32; 3] = [0.5, 1.3, 1.0];
const HEIGHT: [f32; 3] = [0.1, 3.0, 3.5];
const LOOK: [Vec3; 3] = [Vec3::new(1.2, 0.0, -0.2), Vec3::new(-4.0, 0.0, -0.2), Vec3::new(1.2, 0.0, 1.0)];

/// The player's car at the shot's start or now.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Car {
    pub position: Vec3,
    pub velocity: Vec3,
    pub forward: Vec3,
    pub left: Vec3,
    pub up: Vec3,
}

/// A shot being played.
#[derive(Debug, Clone, PartialEq)]
pub struct TrackCar {
    pub eye: Vec3,
    framing: Framing,
}

impl TrackCar {
    /// Places the eye for a shot of `car`. `previous` is the last car shot's eye and `camera` the gameplay camera's.
    pub fn start(framing: Framing, car: &Car, previous: Option<Vec3>, camera: Vec3, scene: &dyn Scene) -> Self {
        let index = framing as usize;
        let speed = car.velocity.length();
        let lead = |secs: f32| match speed * secs > MAX_LEAD {
            true => MAX_LEAD / speed,
            false => secs,
        };
        let p = car.position;
        let near = p + car.velocity * 0.1;
        let ahead = p + car.velocity * lead(LEAD[index]);
        let beside = p + car.velocity * lead(SIDE_LEAD);
        let road = scene.road_ahead(p, car.forward, speed * lead(LEAD[index]));
        let edge = |centre: Vec3, edge: Vec3| {
            let across = edge - centre;
            let width = Vec3::new(across.x, across.y, 0.0).length();
            let share = match width > 10.0 {
                true => 0.9 * 10.0 / width,
                false => 0.9,
            };
            centre + across * share
        };
        let raise =
            |at: Vec3, height: f32| Vec3::new(at.x, at.y, scene.ground(at).map_or(at.z, |g| at.z.max(g)) + height);
        let (left, right) = match road {
            Some((centre, [l, r])) => (raise(edge(centre, l), HEIGHT[index]), raise(edge(centre, r), HEIGHT[index])),
            None => (ahead, ahead),
        };
        let side = |at: Vec3| Vec3::new(at.x, at.y, scene.ground(at).unwrap_or(at.z) + SIDE_HEIGHT);
        let left_side = side(beside + car.left * SIDE_OFFSET);
        let right_side = side(beside - car.left * SIDE_OFFSET);
        let usable = |eye: Vec3| {
            let sees = scene.clear(eye, p) || (framing != Framing::Roadblock && scene.clear(eye, near));
            sees && previous.is_none_or(|last| last.distance(eye) > NEW_EYE)
        };
        let score = |eye: Vec3, from: Vec3, weight: f32, ok: bool| match ok && usable(eye) {
            true => weight * from.distance(eye),
            false => 0.0,
        };
        let on_road = road.is_some();
        let a = score(left, ahead, 1.0, on_road);
        let b = score(right, ahead, 1.0, on_road);
        let c = score(left_side, beside, 0.1, true);
        let d = score(right_side, beside, 0.1, true);
        // The side with the higher total wins; on it, the road edge unless the eye beside the path scores more. With no
        // usable eye the shot looks from 2 m above the gameplay camera, or from the camera itself when that is hidden.
        let none = a == 0.0 && b == 0.0 && c == 0.0 && d == 0.0;
        let raised = camera + Vec3::Z * 2.0;
        let eye = match (none, c + a > d + b, a > c, b > d) {
            (true, ..) if scene.clear(raised, p) => raised,
            (true, ..) => camera,
            (false, true, true, _) => left,
            (false, true, false, _) => left_side,
            (false, false, _, true) => right,
            (false, false, _, false) => right_side,
        };
        Self { eye: eye + Vec3::Z * EYE_LIFT, framing }
    }

    /// The point looked at and the lens (16-bit angle units) for the car where it is now.
    pub fn view(&self, car: &Car) -> (Vec3, f32) {
        let offset = LOOK[self.framing as usize];
        let target = car.position + car.forward * offset.x + car.left * offset.y + car.up * offset.z;
        let distance = self.eye.distance(car.position).max(1.0);
        let fov = 2.0 * FRAME_HALF.atan2(distance) / std::f32::consts::TAU * 65536.0;
        (target, fov.clamp(FOV_MIN, FOV_MAX))
    }

    /// Whether the shot should end now: the car is out of the eye's sight in the shot's last second.
    pub fn lost(&self, car: &Car, remaining: f32, scene: &dyn Scene) -> bool {
        remaining < 1.0 && !scene.clear(self.eye, car.position)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A flat open world on the ground plane with a 10 m road whose left edge is at +5 y.
    struct Open;

    impl Scene for Open {
        fn clear(&self, _: Vec3, _: Vec3) -> bool {
            true
        }
        fn ground(&self, _: Vec3) -> Option<f32> {
            Some(0.0)
        }
        fn road_ahead(&self, at: Vec3, forward: Vec3, ahead: f32) -> Option<(Vec3, [Vec3; 2])> {
            let centre = Vec3::new(at.x + forward.x * ahead, 0.0, 0.0);
            Some((centre, [centre + Vec3::Y * 5.0, centre - Vec3::Y * 5.0]))
        }
    }

    fn car() -> Car {
        Car { position: Vec3::ZERO, velocity: Vec3::X * 30.0, forward: Vec3::X, left: Vec3::Y, up: Vec3::Z }
    }

    #[test]
    fn the_eye_stands_at_the_road_edge_ahead_and_above_it() {
        let shot = TrackCar::start(Framing::Pursued, &car(), None, Vec3::ZERO, &Open);
        assert!((shot.eye.x - 39.0).abs() < 1e-3, "1.3 s ahead at 30 m/s: {:?}", shot.eye);
        assert!((shot.eye.y.abs() - 4.5).abs() < 1e-3, "at 90 % of the half width: {:?}", shot.eye);
        assert!((shot.eye.z - 3.5).abs() < 1e-3, "3 m above the ground plus the lift: {:?}", shot.eye);
    }

    #[test]
    fn a_new_eye_keeps_away_from_the_last_one() {
        let first = TrackCar::start(Framing::Plain, &car(), None, Vec3::ZERO, &Open);
        let second = TrackCar::start(Framing::Plain, &car(), Some(first.eye), Vec3::ZERO, &Open);
        assert!(second.eye.distance(first.eye) > NEW_EYE, "{:?} after {:?}", second.eye, first.eye);
    }

    #[test]
    fn the_lens_keeps_six_metres_of_the_car_in_frame() {
        let shot = TrackCar { eye: Vec3::new(0.0, 20.0, 0.0), framing: Framing::Plain };
        let (target, fov) = shot.view(&car());
        assert_eq!(target, Vec3::new(1.2, 0.0, -0.2));
        let degrees = fov / 65536.0 * 360.0;
        assert!((degrees - 18.74).abs() < 0.05, "{degrees}");
        let close = TrackCar { eye: Vec3::new(0.0, 2.0, 0.0), framing: Framing::Plain };
        assert_eq!(close.view(&car()).1, FOV_MAX);
    }
}

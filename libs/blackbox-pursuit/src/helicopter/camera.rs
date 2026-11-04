//! `AvoidCamera`: the helicopter never flies through the game camera (spec `docs/specs/ai-helicopter.md` section 4.1).
//!
//! A sphere stands 4 m in front of the camera. The segment from the helicopter to where its velocity takes it in 0.2 s is
//! tested against it; on a hit the destination is moved onto the sphere's tangent plane and the velocity is kicked away.
//!
//! [ours] The segment and the kick use the helicopter's velocity relative to the camera. The spec gives them in world
//! velocity, but the remake runs the controller every physics step: a camera following the car at 80 m/s caught the
//! helicopter in its sphere step after step, and each kick (up to 3.45 times the inward speed) compounded the last, so
//! the helicopter overtaking the camera was flung off at 390 m/s. Relative to the camera the kick only cancels the
//! closing speed, which keeps it out of the sphere without adding speed.

use glam::Vec3;

/// The sphere's radius, metres (`CameraRadiusToAvoid`).
pub const RADIUS: f32 = 7.0;
/// How far in front of the camera the sphere stands, metres.
const AHEAD: f32 = 4.0;
/// How far ahead the tested segment reaches, seconds of velocity.
const LEAD: f32 = 0.2;
/// The constant part of the kick.
const KICK: f32 = 1.2;
/// Inside the sphere but farther than this from its centre, the radius shrinks to just inside the helicopter.
const CENTRE_ZONE: f32 = 2.0;

/// The game camera as the helicopter sees it: its place and the way it looks.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Camera {
    pub position: Vec3,
    pub forward: Vec3,
    /// How fast the camera moves (zero when unknown): the kick is taken relative to it.
    pub velocity: Vec3,
}

/// What the avoidance changes.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Avoided {
    pub dest: Vec3,
    pub velocity: Vec3,
}

/// The first point where the segment `from` to `to` enters the sphere (`centre`, `radius`), if it does.
fn enter(from: Vec3, to: Vec3, centre: Vec3, radius: f32) -> Option<Vec3> {
    let d = to - from;
    let f = from - centre;
    let a = d.length_squared();
    if a < 1e-12 {
        return None;
    }
    let b = 2.0 * f.dot(d);
    let c = f.length_squared() - radius * radius;
    if c <= 0.0 {
        return None;
    }
    let discriminant = b * b - 4.0 * a * c;
    if discriminant < 0.0 {
        return None;
    }
    let t = (-b - discriminant.sqrt()) / (2.0 * a);
    (0.0..=1.0).contains(&t).then(|| from + d * t)
}

/// The destination and velocity after the camera is avoided, or `None` when the path misses the sphere. `work` is the
/// lead position (`pos + V * 0.45`).
pub fn avoid(position: Vec3, velocity: Vec3, work: Vec3, camera: Camera) -> Option<Avoided> {
    let centre = camera.position + camera.forward.normalize_or_zero() * AHEAD;
    let distance = position.distance(centre);
    let radius = match distance < RADIUS && distance > CENTRE_ZONE {
        true => distance - 0.5,
        false => RADIUS,
    };
    let relative = velocity - camera.velocity;
    let end = position + relative * LEAD;
    let hit = enter(position, end, centre, radius)?;
    let normal = (hit - centre).normalize_or_zero();
    let dest = work + normal * (hit - work).dot(normal);
    let kick = normal * (relative.dot(normal).abs() * ((RADIUS - radius) * 0.5 + KICK));
    Some(Avoided { dest, velocity: velocity + kick })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn camera() -> Camera {
        Camera { position: Vec3::ZERO, forward: Vec3::X, velocity: Vec3::ZERO }
    }

    #[test]
    fn a_path_that_misses_the_sphere_is_untouched() {
        let got = avoid(Vec3::new(100.0, 0.0, 50.0), Vec3::new(-20.0, 0.0, 0.0), Vec3::ZERO, camera());
        assert!(got.is_none());
    }

    #[test]
    fn a_helicopter_flying_into_the_sphere_is_pushed_away_from_it() {
        // The sphere stands at x = 4 with radius 7; the helicopter is at x = 14 flying at it at 40 m/s: 0.2 s reaches x = 6.
        let position = Vec3::new(14.0, 0.0, 0.0);
        let velocity = Vec3::new(-40.0, 0.0, 0.0);
        let got = avoid(position, velocity, position + velocity * 0.45, camera()).unwrap();
        // The hit is at x = 11 with the normal along +x: the destination lies on the plane x = 11.
        assert!((got.dest.x - 11.0).abs() < 1e-3, "{:?}", got.dest);
        // Kick = n * |v.n| * 1.2 with the full radius.
        assert!((got.velocity.x - (-40.0 + 40.0 * 1.2)).abs() < 1e-3, "{:?}", got.velocity);
    }

    #[test]
    fn inside_the_sphere_the_radius_shrinks_to_just_inside_the_helicopter() {
        // 4 m from the centre: the radius becomes 3.5 and the kick gains (7 - 3.5) * 0.5.
        let position = Vec3::new(8.0, 0.0, 0.0);
        let velocity = Vec3::new(-30.0, 0.0, 0.0);
        let got = avoid(position, velocity, position, camera()).unwrap();
        let expected = -30.0 + 30.0 * ((7.0 - 3.5) * 0.5 + 1.2);
        assert!((got.velocity.x - expected).abs() < 1e-2, "{:?}", got.velocity);
    }

    #[test]
    fn a_helicopter_at_the_centre_is_left_alone() {
        let centre = Vec3::new(4.0, 0.0, 0.0);
        assert!(avoid(centre, Vec3::X * 10.0, centre, camera()).is_none(), "no entry from inside");
    }

    #[test]
    fn a_standing_helicopter_has_no_segment() {
        assert!(avoid(Vec3::new(14.0, 0.0, 0.0), Vec3::ZERO, Vec3::ZERO, camera()).is_none());
    }

    #[test]
    fn a_camera_moving_with_the_helicopter_takes_only_the_closing_speed() {
        // The camera and its sphere fly along +x at 80 m/s; the helicopter, 10 m behind the centre, at 100 m/s.
        let moving = Camera { velocity: Vec3::new(80.0, 0.0, 0.0), ..camera() };
        let position = Vec3::new(-6.0, 0.0, 0.0);
        let velocity = Vec3::new(100.0, 0.0, 0.0);
        let got = avoid(position, velocity, position, moving).unwrap();
        // Closing at 20 m/s: the kick is 20 * 1.2 back, so the helicopter ends 4 m/s slower than the camera.
        assert!((got.velocity.x - (100.0 - 24.0)).abs() < 1e-3, "{:?}", got.velocity);
    }

    #[test]
    fn repeated_kicks_never_speed_the_helicopter_up() {
        // The camera keeps catching the helicopter: step after step the result stays below the speed it came in with.
        let moving = Camera { velocity: Vec3::new(80.0, 0.0, 0.0), ..camera() };
        let mut position = Vec3::new(-6.0, 3.0, 0.0);
        let mut velocity = Vec3::new(100.0, 0.0, 0.0);
        let mut cam = moving;
        for _ in 0..120 {
            if let Some(got) = avoid(position, velocity, position, cam) {
                velocity = got.velocity;
            }
            velocity += (Vec3::new(100.0, 0.0, 0.0) - velocity).clamp_length_max(80.0 / 60.0);
            position += velocity / 60.0;
            cam.position += cam.velocity / 60.0;
            assert!(velocity.length() <= 100.5, "{velocity:?}");
        }
    }
}

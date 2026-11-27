//! An exact camera pose as one line of text, `cam <x> <y> <z> <heading> <pitch> <fov>`, which the world's `cam` console command
//! takes back: a picture's view can be told, logged and set again for graphics comparisons.

use glam::Vec3;

use super::Lens;

/// The free camera's pitch limit in degrees (`FlyCamera::update` clamps to 1.55 radians).
const PITCH_LIMIT_DEGREES: f32 = 88.8;
/// The vertical field of view a pose may ask for, in degrees.
const FOV_RANGE: std::ops::RangeInclusive<f32> = 5.0..=150.0;
/// A coordinate further out than this is a typing error, not a place in the world.
const COORDINATE_LIMIT: f32 = 1_000_000.0;
pub const USAGE: &str = "cam <x> <y> <z> <heading> <pitch> [fov]";

/// Where a camera is and where it looks: the eye in world space (z is absolute, not above the ground), the heading in degrees
/// counter-clockwise from +X and the pitch in degrees up from level (the `goto` convention), and the vertical field of view.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Pose {
    pub eye: Vec3,
    pub heading: f32,
    pub pitch: f32,
    /// Vertical field of view, degrees. A parsed line without one leaves it to the camera.
    pub fov: Option<f32>,
}

impl Pose {
    /// The pose of a drawn lens. The roll of a lens (a bike's lean, a shake) is not part of it.
    pub fn from_lens(lens: &Lens) -> Self {
        let forward = lens.forward.normalize_or_zero();
        let heading = forward.y.atan2(forward.x).to_degrees().rem_euclid(360.0);
        let pitch = forward.z.clamp(-1.0, 1.0).asin().to_degrees();
        Self { eye: lens.position, heading, pitch, fov: Some(lens.fov_y_degrees) }
    }

    /// The pose as the console command that sets it again, with one decimal each.
    pub fn line(&self) -> String {
        let Self { eye, heading, pitch, fov } = *self;
        let fov = fov.map(|fov| format!(" {fov:.1}")).unwrap_or_default();
        // `-0.0` prints as `-0.0`: drop the sign of a value that rounds to zero.
        let tidy = |value: f32| match (value * 10.0).round() == 0.0 {
            true => 0.0,
            false => value,
        };
        format!(
            "cam {:.1} {:.1} {:.1} {:.1} {:.1}{fov}",
            tidy(eye.x),
            tidy(eye.y),
            tidy(eye.z),
            tidy(heading.rem_euclid(360.0) % 360.0),
            tidy(pitch)
        )
    }

    /// The arguments of `cam`: x y z heading pitch, and the field of view if given. Commas between the numbers are allowed,
    /// and a leading `cam` (a whole pasted line) is skipped.
    pub fn parse(args: &[&str]) -> Result<Self, String> {
        let args = match args.first() {
            Some(&"cam") => &args[1..],
            _ => args,
        };
        if !(5..=6).contains(&args.len()) {
            return Err(format!("usage: {USAGE}"));
        }
        let mut values = [0.0f32; 6];
        for (slot, arg) in values.iter_mut().zip(args) {
            let text = arg.trim_end_matches(',');
            *slot = text.parse().map_err(|_| format!("{arg:?} is not a number (usage: {USAGE})"))?;
            if !slot.is_finite() {
                return Err(format!("{arg:?} is not a finite number (usage: {USAGE})"));
            }
        }
        let [x, y, z, heading, pitch, fov] = values;
        if [x, y, z].iter().any(|v| v.abs() > COORDINATE_LIMIT) {
            return Err(format!("the position is out of the world (usage: {USAGE})"));
        }
        if pitch.abs() > PITCH_LIMIT_DEGREES {
            return Err(format!("the pitch must be within ±{PITCH_LIMIT_DEGREES} degrees"));
        }
        let fov = (args.len() == 6).then_some(fov);
        if let Some(fov) = fov.filter(|fov| !FOV_RANGE.contains(fov)) {
            return Err(format!(
                "a field of view of {fov} degrees is outside {}..{}",
                FOV_RANGE.start(),
                FOV_RANGE.end()
            ));
        }
        Ok(Self { eye: Vec3::new(x, y, z), heading, pitch, fov })
    }

    /// The heading and the pitch in radians, as the free camera keeps them.
    pub fn yaw_pitch_radians(&self) -> (f32, f32) {
        (self.heading.to_radians(), self.pitch.to_radians())
    }
}

#[cfg(test)]
#[path = "pose_tests.rs"]
mod tests;

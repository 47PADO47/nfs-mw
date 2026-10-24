//! Dynamic FOV: the view widens as the car speeds up, so speed reads at a glance. The curve is a
//! normalised speed, eased with a cubic, mapped between the chase camera's rest FOV and a wider top
//! FOV. Written from the behaviour of MWDynamicFOV (no licence, so facts only; see NOTICE).

use super::{Camera, Lens};

/// Field of view at rest, degrees: a little narrower than the chase camera, so a stopped car looks close.
pub const REST_FOV_DEGREES: f32 = 45.0;
/// Field of view at and above [`TOP_SPEED`], degrees: wider than before, so speed reads.
pub const TOP_FOV_DEGREES: f32 = 110.0;
/// The speed (m/s) at which the view is fully widened (about 288 km/h); faster does not go further.
pub const TOP_SPEED: f32 = 80.0;

/// The field of view for `speed` (m/s, either direction): cubic ease-in between the rest and top FOVs.
pub fn fov_for_speed(speed: f32) -> f32 {
    let t = (speed.abs() / TOP_SPEED).clamp(0.0, 1.0);
    REST_FOV_DEGREES + (TOP_FOV_DEGREES - REST_FOV_DEGREES) * t * t * t
}

/// [`fov_for_speed`] with its widening over the rest FOV scaled by `intensity` (`[hud] fov_intensity`, 0 to 1): 1 is the
/// full curve, 0 keeps the rest FOV at every speed. A value that is not a number is the full curve.
pub fn scaled_fov_for_speed(speed: f32, intensity: f32) -> f32 {
    let full = fov_for_speed(speed);
    // The full curve exactly, not through the subtraction below.
    if !intensity.is_finite() || intensity >= 1.0 {
        return full;
    }
    REST_FOV_DEGREES + (full - REST_FOV_DEGREES) * intensity.max(0.0)
}

/// The chase camera with its field of view set by the car's speed. It borrows the chase camera
/// (which does the following) and only changes how wide it sees.
pub struct DynamicFov<'a> {
    pub chase: &'a super::chase::ChaseView,
    /// The car's speed, m/s.
    pub speed: f32,
    /// Degrees added on top of the speed's field of view (the high-speed effects' nitrous kick).
    pub kick: f32,
    /// How much of the speed widening applies, 0 to 1 (`[hud] fov_intensity`); the kick is not scaled.
    pub intensity: f32,
}

impl Camera for DynamicFov<'_> {
    fn lens(&self, aspect: f32) -> Lens {
        self.chase.lens_with_fov(aspect, scaled_fov_for_speed(self.speed, self.intensity) + self.kick)
    }

    fn selectable(&self) -> bool {
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rests_at_the_chase_fov_and_tops_out_at_top_speed() {
        assert!((fov_for_speed(0.0) - REST_FOV_DEGREES).abs() < 1e-4);
        assert!((fov_for_speed(TOP_SPEED) - TOP_FOV_DEGREES).abs() < 1e-4);
        assert!((fov_for_speed(TOP_SPEED * 3.0) - TOP_FOV_DEGREES).abs() < 1e-4, "faster does not widen more");
    }

    #[test]
    fn eases_in_and_never_narrows() {
        // Half speed is an eighth of the way: the cubic keeps the early speeds close to the rest FOV.
        let half = fov_for_speed(TOP_SPEED / 2.0);
        assert!((half - (REST_FOV_DEGREES + (TOP_FOV_DEGREES - REST_FOV_DEGREES) / 8.0)).abs() < 1e-4);
        let mut last = fov_for_speed(0.0);
        for step in 1..=80 {
            let fov = fov_for_speed(step as f32);
            assert!(fov >= last, "widens with speed");
            last = fov;
        }
        assert!((fov_for_speed(-TOP_SPEED) - TOP_FOV_DEGREES).abs() < 1e-4, "reverse counts as speed");
    }

    #[test]
    fn the_intensity_scales_only_the_widening_over_the_rest_fov() {
        for speed in [0.0, TOP_SPEED / 2.0, TOP_SPEED, TOP_SPEED * 2.0] {
            assert_eq!(scaled_fov_for_speed(speed, 1.0), fov_for_speed(speed), "1 is the full curve");
            assert_eq!(scaled_fov_for_speed(speed, 0.0), REST_FOV_DEGREES, "0 is the rest FOV at {speed}");
            assert_eq!(scaled_fov_for_speed(speed, f32::NAN), fov_for_speed(speed));
        }
        let half = scaled_fov_for_speed(TOP_SPEED, 0.5);
        assert!((half - (REST_FOV_DEGREES + TOP_FOV_DEGREES) / 2.0).abs() < 1e-4);
        assert_eq!(scaled_fov_for_speed(TOP_SPEED, 2.0), TOP_FOV_DEGREES, "clamped to 1");
    }

    #[test]
    fn a_zero_intensity_keeps_the_rest_fov_and_the_kick() {
        let chase = crate::viewer::camera::ChaseCamera::default().view();
        let lens = DynamicFov { chase: &chase, speed: TOP_SPEED, kick: 5.0, intensity: 0.0 }.lens(16.0 / 9.0);
        assert!((lens.fov_y_degrees - (REST_FOV_DEGREES + 5.0)).abs() < 1e-4);
    }

    #[test]
    fn keeps_the_chase_position_and_widens_the_lens() {
        let chase = crate::viewer::camera::ChaseCamera::default().view();
        let lens = DynamicFov { chase: &chase, speed: TOP_SPEED, kick: 0.0, intensity: 1.0 }.lens(16.0 / 9.0);
        assert_eq!(lens.position, chase.lens(16.0 / 9.0).position);
        assert_eq!(lens.fov_y_degrees, TOP_FOV_DEGREES);
        assert!(DynamicFov { chase: &chase, speed: 0.0, kick: 0.0, intensity: 1.0 }.selectable());
    }
}

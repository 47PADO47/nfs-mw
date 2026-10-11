//! Dynamic FOV: the view widens as the car speeds up, so speed reads at a glance. The curve is a
//! normalised speed, eased with a cubic, mapped between the chase camera's rest FOV and a wider top
//! FOV. Written from the behaviour of MWDynamicFOV (no licence, so facts only; see NOTICE).

use super::{Camera, ChaseCamera, Lens};

/// Field of view at rest, degrees: the chase camera's own rest FOV.
pub const REST_FOV_DEGREES: f32 = 60.0;
/// Field of view at and above [`TOP_SPEED`], degrees.
pub const TOP_FOV_DEGREES: f32 = 90.0;
/// The speed (m/s) at which the view is fully widened (about 288 km/h); faster does not go further.
pub const TOP_SPEED: f32 = 80.0;

/// The field of view for `speed` (m/s, either direction): cubic ease-in between the rest and top FOVs.
pub fn fov_for_speed(speed: f32) -> f32 {
    let t = (speed.abs() / TOP_SPEED).clamp(0.0, 1.0);
    REST_FOV_DEGREES + (TOP_FOV_DEGREES - REST_FOV_DEGREES) * t * t * t
}

/// The chase camera with its field of view set by the car's speed. It borrows the chase camera
/// (which does the following) and only changes how wide it sees.
pub struct DynamicFov<'a> {
    pub chase: &'a ChaseCamera,
    /// The car's speed, m/s.
    pub speed: f32,
}

impl Camera for DynamicFov<'_> {
    fn lens(&self, aspect: f32) -> Lens {
        self.chase.lens_with_fov(aspect, fov_for_speed(self.speed))
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
    fn keeps_the_chase_position_and_widens_the_lens() {
        let chase = ChaseCamera::default();
        let lens = DynamicFov { chase: &chase, speed: TOP_SPEED }.lens(16.0 / 9.0);
        assert_eq!(lens.position, chase.lens(16.0 / 9.0).position);
        assert_eq!(lens.fov_y_degrees, TOP_FOV_DEGREES);
        assert!(DynamicFov { chase: &chase, speed: 0.0 }.selectable());
    }
}

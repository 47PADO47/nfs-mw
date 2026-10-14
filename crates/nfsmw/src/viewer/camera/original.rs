//! The original game's gameplay views (bumper, hood, close, far, super far, drift, pursuit): each is placed from the
//! game's own `camerainfo` values for field of view, height, offset along the car and pitch. The pair selects
//! fullscreen or horizontal split-screen, not a vehicle speed curve. The sign
//! of the offset and the meaning of the stiffness are not (see docs/specs/camera-views.md).

use glam::Vec3;
use nfsmw_data::camera::View;

use super::{Camera, Lens, direction};
use crate::settings::CameraMode;

/// Distance of the near plane, metres (the same as the chase camera).
const NEAR_PLANE: f32 = 0.3;
/// Distance the view looks along its direction, metres: only sets the target point, not the picture.
const LOOK_DISTANCE: f32 = 10.0;
/// The index of a view in `nfsmw_data::camera::VIEWS`, for a camera setting. `None` for the two chase cameras.
pub fn index_of(mode: CameraMode) -> Option<usize> {
    Some(match mode {
        CameraMode::Bumper => 0,
        CameraMode::Hood => 1,
        CameraMode::Close => 2,
        CameraMode::Far => 3,
        CameraMode::SuperFar => 4,
        CameraMode::Drift => 5,
        CameraMode::Pursuit => 6,
        CameraMode::Chase | CameraMode::DynamicFov => return None,
    })
}

/// Where a positive offset starts: the front bumper, so a forward view is outside the body. A negative offset
/// starts at the car's origin. Inferred from the screenshots (the bumper and hood views sat inside the body when the
/// offset started at the origin); see docs/specs/camera-views.md.
fn forward_from_centre(lag: f32, half_length: f32) -> f32 {
    if lag > 0.0 { half_length } else { 0.0 }
}

/// One original view for one frame: the car's origin and heading, the view's values at the car's speed.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct OriginalView {
    eye: Vec3,
    forward: Vec3,
    fov_y_degrees: f32,
}

impl OriginalView {
    /// The view `view` on a car at `position`, facing `heading` (radians from +X), moving at `speed` m/s.
    pub fn new(view: &View, position: Vec3, heading: f32, _speed: f32, half_length: f32) -> Self {
        // Single-player uses the fullscreen slot. Native mode TWOH alone uses
        // the second slot; split-screen is not available in this host.
        let lag = view.lag[0];
        let height = view.height[0];
        let pitch = view.angle[0].to_radians();
        Self {
            eye: position + direction(heading, 0.0) * (lag + forward_from_centre(lag, half_length)) + Vec3::Z * height,
            forward: direction(heading, -pitch),
            fov_y_degrees: view.fov[0],
        }
    }

    /// Native POV target: zoom divides spatial lag and multiplies FOV.
    pub fn with_zoom(view: &View, position: Vec3, heading: f32, speed: f32, half_length: f32, zoom: f32) -> Self {
        let mut target = *view;
        target.lag = target.lag.map(|lag| lag / zoom);
        target.fov = target.fov.map(|fov| fov * zoom);
        Self::new(&target, position, heading, speed, half_length)
    }
}

impl Camera for OriginalView {
    fn lens(&self, aspect: f32) -> Lens {
        Lens::look_at(self.eye, self.eye + self.forward * LOOK_DISTANCE, self.fov_y_degrees, NEAR_PLANE, aspect)
    }

    fn selectable(&self) -> bool {
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const CAR: Vec3 = Vec3::new(100.0, 50.0, 10.0);

    fn view() -> View {
        View { fov: [78.0, 90.0], height: [1.0, 2.0], lag: [-4.0, -8.0], angle: [0.0, 10.0], stiffness: [0.5, 0.5] }
    }

    #[test]
    fn the_setting_picks_the_seven_views_in_the_game_order() {
        let order = [
            CameraMode::Bumper,
            CameraMode::Hood,
            CameraMode::Close,
            CameraMode::Far,
            CameraMode::SuperFar,
            CameraMode::Drift,
            CameraMode::Pursuit,
        ];
        for (i, mode) in order.into_iter().enumerate() {
            assert_eq!(index_of(mode), Some(i));
        }
        assert_eq!(index_of(CameraMode::Chase), None);
        assert_eq!(index_of(CameraMode::DynamicFov), None);
    }

    #[test]
    fn the_offset_is_along_the_heading_and_the_height_is_up() {
        // Heading a quarter turn left: forward is +Y, so a negative offset puts the camera at -Y (behind).
        let cam = OriginalView::new(&view(), CAR, std::f32::consts::FRAC_PI_2, 0.0, 2.2);
        let lens = cam.lens(1.0);
        assert!((lens.position - (CAR + Vec3::new(0.0, -4.0, 1.0))).length() < 1e-3, "{:?}", lens.position);
    }

    #[test]
    fn fullscreen_values_do_not_interpolate_toward_split_screen_values_at_speed() {
        let at_rest = OriginalView::new(&view(), CAR, 0.0, 0.0, 2.2).lens(1.0);
        let fast = OriginalView::new(&view(), CAR, 0.0, 60.0, 2.2).lens(1.0);
        let faster = OriginalView::new(&view(), CAR, 0.0, 180.0, 2.2).lens(1.0);
        assert!((at_rest.fov_y_degrees - 78.0).abs() < 1e-4);
        assert_eq!(fast.fov_y_degrees, at_rest.fov_y_degrees);
        assert_eq!(fast.fov_y_degrees, faster.fov_y_degrees);
        assert_eq!(fast.position, at_rest.position);
    }

    #[test]
    fn a_positive_pitch_looks_down() {
        let level = OriginalView::new(&view(), CAR, 0.0, 0.0, 2.2).lens(1.0);
        let mut down = view();
        down.angle = [10.0, 0.0];
        let pitched = OriginalView::new(&down, CAR, 0.0, 60.0, 2.2).lens(1.0);
        assert!(level.forward.z.abs() < 1e-6);
        assert!(pitched.forward.z < -0.1, "{:?}", pitched.forward);
    }

    #[test]
    fn breaker_pov_divides_lag_and_multiplies_fov_without_changing_height_or_angle() {
        for speed in [0.0, 30.0, 60.0] {
            let normal = OriginalView::new(&view(), CAR, 0.0, speed, 2.2).lens(1.0);
            for zoom in [1.0, 0.8, 0.6] {
                let held = OriginalView::with_zoom(&view(), CAR, 0.0, speed, 2.2, zoom).lens(1.0);
                assert!((held.position.x - CAR.x - (normal.position.x - CAR.x) / zoom).abs() < 1e-4);
                assert_eq!(held.position.z, normal.position.z);
                assert!((held.fov_y_degrees - normal.fov_y_degrees * zoom).abs() < 1e-4);
                assert_eq!(held.forward, normal.forward);
            }
        }
    }

    #[test]
    fn a_forward_offset_starts_at_the_front_bumper() {
        // A positive offset of 0.5 m is 0.5 m past the front bumper (2.2 m from the origin), not 0.5 m from the origin.
        let mut forward = view();
        forward.lag = [0.5, 0.5];
        let lens = OriginalView::new(&forward, CAR, 0.0, 0.0, 2.2).lens(1.0);
        assert!((lens.position.x - (CAR.x + 2.7)).abs() < 1e-4, "{:?}", lens.position);
    }

    #[test]
    fn every_original_view_is_selectable() {
        assert!(OriginalView::new(&view(), CAR, 0.0, 0.0, 2.2).selectable());
    }
}

//! Ordinary hotkey selection uses native SELECTABLE, while direct choices remain available.

use nfsmw_data::camera::Views;

use crate::settings::CameraMode;
use crate::viewer::camera::original;

pub(super) fn next_mode(current: CameraMode, views: Option<&Views>) -> CameraMode {
    let Some(views) = views else {
        return current.next();
    };
    let mut candidate = current;
    for _ in CameraMode::ALL {
        candidate = candidate.next();
        let Some(index) = original::index_of(candidate) else {
            return candidate;
        };
        if views.views[index].selectable[0] {
            return candidate;
        }
    }
    current
}

#[cfg(test)]
mod tests {
    use nfsmw_data::camera::View;

    use super::*;

    fn views(selectable: [bool; 7]) -> Views {
        Views {
            views: std::array::from_fn(|index| {
                let selected = selectable[index];
                View {
                    fov: [90.0; 2],
                    height: [1.7; 2],
                    lag: [-5.0; 2],
                    angle: [0.0; 2],
                    stiffness: [0.5; 2],
                    lat_offset: [0.0; 2],
                    tilting: [true; 2],
                    selectable: [selected, !selected],
                }
            }),
        }
    }

    #[test]
    fn native_fullscreen_flags_skip_the_three_extra_views_and_retain_custom_cameras() {
        let views = views([true, true, true, true, false, false, false]);
        let mut current = CameraMode::Chase;
        for expected in [
            CameraMode::DynamicFov,
            CameraMode::Bumper,
            CameraMode::Hood,
            CameraMode::Close,
            CameraMode::Far,
            CameraMode::Chase,
        ] {
            current = next_mode(current, Some(&views));
            assert_eq!(current, expected);
        }
        // Direct selection still reaches an extra native view and cycles out normally.
        assert_eq!(next_mode(CameraMode::Drift, Some(&views)), CameraMode::Chase);
    }

    #[test]
    fn car_specific_flags_use_fullscreen_slot_and_never_remove_custom_modes() {
        let profiles = views([false, true, false, true, false, false, false]);
        assert_eq!(next_mode(CameraMode::DynamicFov, Some(&profiles)), CameraMode::Hood);
        assert_eq!(next_mode(CameraMode::Hood, Some(&profiles)), CameraMode::Far);
        let none = views([false; 7]);
        assert_eq!(next_mode(CameraMode::DynamicFov, Some(&none)), CameraMode::Chase);
        assert_eq!(next_mode(CameraMode::Chase, Some(&none)), CameraMode::DynamicFov);
    }

    #[test]
    fn missing_profiles_keep_the_existing_cycle() {
        for mode in CameraMode::ALL {
            assert_eq!(next_mode(mode, None), mode.next());
        }
    }
}

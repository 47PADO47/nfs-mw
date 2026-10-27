//! Ordinary hotkey selection uses native SELECTABLE, while direct choices remain available.

use nfsmw_data::camera::Views;

use crate::settings::{CameraMode, ViewKey};
use crate::viewer::camera::original;

/// The camera after `current` when the hotkey is pressed: the built-in cameras the car's profile allows, then the extra views
/// that fit the car (`extras`, in registration order), then the first built-in camera again.
pub(super) fn next_mode(current: CameraMode, views: Option<&Views>, extras: &[ViewKey]) -> CameraMode {
    if let CameraMode::Extra(key) = current {
        let after = extras.iter().position(|known| *known == key).and_then(|at| extras.get(at + 1));
        return match after {
            Some(next) => CameraMode::Extra(*next),
            None => next_built_in(CameraMode::ALL[CameraMode::ALL.len() - 1], views),
        };
    }
    let next = next_built_in(current, views);
    let position = |mode: CameraMode| CameraMode::ALL.iter().position(|known| *known == mode);
    let wrapped = position(next) <= position(current);
    match (wrapped, extras.first()) {
        (true, Some(first)) => CameraMode::Extra(*first),
        _ => next,
    }
}

fn next_built_in(current: CameraMode, views: Option<&Views>) -> CameraMode {
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
            current = next_mode(current, Some(&views), &[]);
            assert_eq!(current, expected);
        }
        // Direct selection still reaches an extra native view and cycles out normally.
        assert_eq!(next_mode(CameraMode::Drift, Some(&views), &[]), CameraMode::Chase);
    }

    #[test]
    fn car_specific_flags_use_fullscreen_slot_and_never_remove_custom_modes() {
        let profiles = views([false, true, false, true, false, false, false]);
        assert_eq!(next_mode(CameraMode::DynamicFov, Some(&profiles), &[]), CameraMode::Hood);
        assert_eq!(next_mode(CameraMode::Hood, Some(&profiles), &[]), CameraMode::Far);
        let none = views([false; 7]);
        assert_eq!(next_mode(CameraMode::DynamicFov, Some(&none), &[]), CameraMode::Chase);
        assert_eq!(next_mode(CameraMode::Chase, Some(&none), &[]), CameraMode::DynamicFov);
    }

    #[test]
    fn extra_views_follow_the_last_built_in_camera_and_lead_back_to_the_first() {
        let (a, b) = (ViewKey::of("tests", "a"), ViewKey::of("tests", "b"));
        let extras = [a, b];
        assert_eq!(next_mode(CameraMode::Pursuit, None, &extras), CameraMode::Extra(a));
        assert_eq!(next_mode(CameraMode::Extra(a), None, &extras), CameraMode::Extra(b));
        assert_eq!(next_mode(CameraMode::Extra(b), None, &extras), CameraMode::Chase);
        assert_eq!(next_mode(CameraMode::Chase, None, &extras), CameraMode::DynamicFov, "extras only after the wrap");
    }

    #[test]
    fn an_unavailable_extra_view_goes_back_to_the_first_camera_and_profiles_still_apply() {
        let (a, gone) = (ViewKey::of("tests", "a"), ViewKey::of("tests", "gone"));
        assert_eq!(next_mode(CameraMode::Extra(gone), None, &[a]), CameraMode::Chase);
        let none = views([false; 7]);
        // With every original view switched off by the car's profile, the custom cameras and the extras remain.
        assert_eq!(next_mode(CameraMode::DynamicFov, Some(&none), &[a]), CameraMode::Extra(a));
        assert_eq!(next_mode(CameraMode::Extra(a), Some(&none), &[a]), CameraMode::Chase);
    }

    #[test]
    fn missing_profiles_keep_the_existing_cycle() {
        for mode in CameraMode::ALL {
            assert_eq!(next_mode(mode, None, &[]), mode.next());
        }
    }
}

//! The city map of the milestones and bounty sheets (`docs/specs/career-rep-sheets.md` section 1.1). The picture is the
//! whole city, `TRACK_MAP`, whatever the career's stage. The view is a square 0.27 of the picture's side centred on the
//! selected entry's marker, and the package's pin (the car arrow at the picture's centre) is turned to the marker's
//! heading. The race sheet shows its race's own picture instead (`race.rs`).

use std::sync::OnceLock;

use blackbox_feng::fe_hash_upper;
use blackbox_minimap::{Calibration, bearing_degrees};

use crate::career::JumpPoint;
use crate::frontend::challenge_series::{Crop, board};

/// The side of the picture the view shows about the marker, in picture units.
const SIDE: f32 = 0.27;

/// The picture of the two sheets: the whole city (`TRACK_MAP` of `TrackMaps.bin`, read with the UI assets).
pub fn texture() -> u32 {
    fe_hash_upper("TRACK_MAP")
}

/// The picture and the view of a marker: the square about its place on the picture. It is not moved inside the
/// picture, so the pin at the picture's centre stays on the marker.
pub fn view(city: &Calibration, point: &JumpPoint) -> (u32, Crop) {
    (texture(), Crop { centre: city.to_map(point.xy()), half: SIDE * 0.5 })
}

/// The pin's bearing, degrees clockwise from up the picture: the way the marker faces (`Rotation` 0 when it has none).
pub fn bearing(point: &JumpPoint) -> f32 {
    let heading = point.heading.unwrap_or(0.0);
    bearing_degrees([heading.cos(), heading.sin()])
}

/// The open city's calibration, read from the install the first time a sheet opens. `None` while the install is not
/// known or has none; it is asked again on the next open.
pub fn city() -> Option<Calibration> {
    static CITY: OnceLock<Calibration> = OnceLock::new();
    if let Some(city) = CITY.get() {
        return Some(*city);
    }
    let dir = board::install()?;
    let city = match nfsmw_data::minimap::open_city_calibration(dir) {
        Ok(city) => city,
        Err(e) => {
            log::warn!("rep sheet map: no city calibration: {e:#}");
            return None;
        }
    };
    Some(*CITY.get_or_init(|| city))
}

#[cfg(test)]
mod tests {
    use glam::Vec3;

    use super::*;

    const CITY: Calibration = Calibration { origin: [-1224.8894, -1591.1449], width: 6659.332 };

    #[test]
    fn the_view_is_the_whole_city_0_27_wide_about_the_marker() {
        // Riverfront Stadium: map (939.84, -595.59), so physics (595.59, y, 939.84).
        let point = JumpPoint::from_marker(Vec3::new(595.59, 12.0, 939.84), Some(-102.7));
        let (texture, crop) = view(&CITY, &point);
        assert_eq!(texture, fe_hash_upper("TRACK_MAP"));
        assert_eq!(crop.centre, CITY.to_map([939.84, -595.59]));
        let [u0, v0, u1, v1] = crop.uv();
        assert!((u1 - u0 - SIDE).abs() < 1e-6 && (v1 - v0 - SIDE).abs() < 1e-6, "{crop:?}");
    }

    #[test]
    fn a_view_by_the_edge_stays_centred_on_the_marker() {
        let point = JumpPoint::from_marker(Vec3::new(1591.0, 0.0, -1224.0), None);
        let (_, crop) = view(&CITY, &point);
        assert_eq!(crop.centre, CITY.to_map(point.xy()));
        assert!(crop.uv()[0] < 0.0 && crop.uv()[3] > 1.0, "runs off the picture, as the original's does: {crop:?}");
    }

    #[test]
    fn the_pin_points_the_way_the_marker_faces() {
        let at = |rotation| bearing(&JumpPoint::from_marker(Vec3::ZERO, rotation));
        // `Rotation` R faces physics (-sin R, 0, cos R): 0 is right on the picture, 180 left, -90 down.
        assert!((at(Some(0.0)) - 90.0).abs() < 1e-3);
        assert!((at(None) - 90.0).abs() < 1e-3, "no `Rotation` is 0");
        assert!((at(Some(180.0)) - 270.0).abs() < 1e-3);
        assert!((at(Some(-90.0)) - 180.0).abs() < 1e-3);
    }
}

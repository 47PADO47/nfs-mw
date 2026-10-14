//! Where the map's picture sits on the FEng screen, in FEng units: the origin is the middle of the screen, y points
//! down and the screen is 480 units tall. The picture is the package's map object (`WorldMapMain.fng`, guid 0xF9984),
//! centred on [`PICTURE_CENTRE`] and [`PICTURE_SIDE`] wide.

use glam::Vec2;

/// The centre of the picture, in FEng units (`docs/provenance/world-map-screen.md`).
pub const PICTURE_CENTRE: [f32; 2] = [-111.0, 11.0];
/// The picture's side, in FEng units.
pub const PICTURE_SIDE: f32 = 340.0;

/// The top left corner of the picture, in FEng units.
pub fn picture_origin() -> [f32; 2] {
    [PICTURE_CENTRE[0] - PICTURE_SIDE * 0.5, PICTURE_CENTRE[1] - PICTURE_SIDE * 0.5]
}

/// A screen point (FEng units) as a view position of the picture: `(0, 0)` its top left, `(1, 1)` its bottom right.
/// Outside the picture the values are below 0 or above 1.
pub fn view_of(point: Vec2) -> [f32; 2] {
    let origin = picture_origin();
    [(point.x - origin[0]) / PICTURE_SIDE, (point.y - origin[1]) / PICTURE_SIDE]
}

/// A view position of the picture as a screen point (FEng units).
pub fn point_of(view: [f32; 2]) -> [f32; 2] {
    let origin = picture_origin();
    [origin[0] + view[0] * PICTURE_SIDE, origin[1] + view[1] * PICTURE_SIDE]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_picture_is_centred_on_its_object() {
        assert_eq!(point_of([0.5, 0.5]), PICTURE_CENTRE);
        assert_eq!(point_of([0.0, 0.0]), picture_origin());
        assert_eq!(point_of([1.0, 1.0]), [PICTURE_CENTRE[0] + 170.0, PICTURE_CENTRE[1] + 170.0]);
    }

    #[test]
    fn a_screen_point_maps_to_the_view_and_back() {
        for view in [[0.0, 0.0], [0.25, 0.75], [1.0, 1.0]] {
            let back = view_of(Vec2::from(point_of(view)));
            assert!((back[0] - view[0]).abs() < 1e-5 && (back[1] - view[1]).abs() < 1e-5, "{view:?}");
        }
        assert!(view_of(Vec2::new(0.0, 300.0))[1] > 1.0, "below the picture");
    }
}

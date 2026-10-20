//! Where the map's picture sits on the FEng screen, in FEng units: the origin is the middle of the screen, y points
//! down and the screen is 480 units tall. The picture is the package's map object (`WorldMapMain.fng`, guid 0xF9984),
//! centred on (-111, 11) and 340 wide ([`Picture::CLASSIC`]).

use glam::Vec2;

/// A square picture on the screen: its top left corner and its side, in FEng units.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Picture {
    pub origin: [f32; 2],
    pub side: f32,
}

impl Picture {
    /// The package's own map object (`docs/provenance/world-map-screen.md`): 340 units wide, centred on (-111, 11).
    pub const CLASSIC: Picture = Picture { origin: [-111.0 - 170.0, 11.0 - 170.0], side: 340.0 };
    /// The themed full-screen map: the picture is the screen's height, centred. The view square is that picture, and the
    /// rest of the window shows the map beyond it.
    pub const FULL: Picture = Picture { origin: [-240.0, -240.0], side: 480.0 };

    /// A screen point (FEng units) as a view position of the picture: `(0, 0)` its top left, `(1, 1)` its bottom
    /// right. Outside the picture the values are below 0 or above 1.
    pub fn view_of(self, point: Vec2) -> [f32; 2] {
        [(point.x - self.origin[0]) / self.side, (point.y - self.origin[1]) / self.side]
    }

    /// A view position of the picture as a screen point (FEng units).
    pub fn point_of(self, view: [f32; 2]) -> [f32; 2] {
        [self.origin[0] + view[0] * self.side, self.origin[1] + view[1] * self.side]
    }
}

/// A screen point as a view position of the package's picture.
pub fn view_of(point: Vec2) -> [f32; 2] {
    Picture::CLASSIC.view_of(point)
}

/// A view position of the package's picture as a screen point.
pub fn point_of(view: [f32; 2]) -> [f32; 2] {
    Picture::CLASSIC.point_of(view)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_picture_is_centred_on_its_object() {
        assert_eq!(point_of([0.5, 0.5]), [-111.0, 11.0]);
        assert_eq!(point_of([0.0, 0.0]), Picture::CLASSIC.origin);
        assert_eq!(point_of([1.0, 1.0]), [-111.0 + 170.0, 11.0 + 170.0]);
    }

    #[test]
    fn the_full_screen_picture_is_the_screens_height_and_centred() {
        assert_eq!(Picture::FULL.point_of([0.5, 0.5]), [0.0, 0.0]);
        assert_eq!(Picture::FULL.view_of(Vec2::new(240.0, -240.0)), [1.0, 0.0]);
        assert!(Picture::FULL.view_of(Vec2::new(400.0, 0.0))[0] > 1.0, "the window runs beyond the picture");
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

//! Where the world lands on the full-screen map. The frontend's map works on the city's picture (`Calibration::to_map`,
//! 0..1 on both axes): its camera zooms and pans over it, and the cursor picks blips by their place on it. The picture
//! is the screen's height, centred (`Picture::FULL`), and the window shows the map beyond it on a wider screen.
//!
//! So the route's strokes project every world point the same way: through the calibration onto the picture, through
//! the camera onto the view, and onto the screen. All three are scales and shifts, so the whole is one north-up
//! [`MapView`], the projection the centred map uses; a route point lands where the icons (`art.rs`, placed from the
//! blips' places on the picture through the camera alone) and the cursor find the same world place.
//!
//! A map opened without the city's calibration has no blips and no route, but it still has the picture, which needs no
//! calibration: it is placed by the camera alone.

use blackbox_minimap::Calibration;
use glam::Vec2;

use crate::frontend::world_map::Camera;
use crate::frontend::world_map::layout::Picture;
use crate::ui::theme::custom::map::view::MapView;

/// The projection of the full-screen map for `calibration` and `camera`: a north-up view whose centre is the screen's
/// centre, showing the picture's point under the camera's centre there, at the picture's size times the zoom.
pub fn view(calibration: Calibration, camera: Camera) -> MapView {
    let [x, y] = camera.centre;
    // The world point under the camera's centre: the inverse of `Calibration::to_map`.
    let middle = [calibration.origin[0] + x * calibration.width, calibration.origin[1] + (1.0 - y) * calibration.width];
    let scale = Picture::FULL.side * camera.zoom() / calibration.width;
    let centre = Picture::FULL.point_of([0.5, 0.5]);
    MapView::new(middle, [0.0, 1.0], false, scale, Vec2::from(centre))
}

/// The screen point of a place on the picture (`PlacedBlip::map`, `PlayerMark::map`) under `camera`.
pub fn on_screen(camera: Camera, map: [f32; 2]) -> Vec2 {
    Vec2::from(Picture::FULL.point_of(camera.to_view(map)))
}

/// The screen point of the cursor, given in view units (`WorldMap::cursor`).
pub fn cursor_on_screen(view: [f32; 2]) -> Vec2 {
    Vec2::from(Picture::FULL.point_of(view))
}

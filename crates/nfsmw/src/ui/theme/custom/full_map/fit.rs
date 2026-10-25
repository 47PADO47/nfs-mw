//! Where the world lands on the full-screen map. The frontend's map works on the city's picture (`Calibration::to_map`,
//! 0..1 on both axes): its camera zooms and pans over it, and the cursor picks blips by their place on it. The picture
//! is the screen's height, centred (`Picture::FULL`), and the window shows the map beyond it on a wider screen.
//!
//! So the full-screen map projects every world point the same way: through the calibration onto the picture, through
//! the camera onto the view, and onto the screen. All three are scales and shifts, so the whole is one north-up
//! [`MapView`], the projection the centred map uses; a blip drawn from its world place lands where the cursor finds it.
//!
//! A map opened without the city's calibration has no blips, but it still has the roads: [`fitted`] makes a
//! calibration that fits the network's bounds into the picture, so the whole network fits the screen's height.

use blackbox_minimap::Calibration;
use glam::Vec2;

use crate::frontend::world_map::Camera;
use crate::frontend::world_map::layout::Picture;
use crate::hud::MapRoads;
use crate::ui::theme::custom::map::view::MapView;

/// The room left round the network when it is fitted into the picture, as a share of its larger side, on each side.
pub const FIT_MARGIN: f32 = 0.05;

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

/// A calibration that puts the network's bounds in the middle of the picture, its larger side filling the picture
/// less [`FIT_MARGIN`] on each side. `None` for a network with no extent.
pub fn fitted(roads: &MapRoads) -> Option<Calibration> {
    let mut points = roads.roads().iter().flat_map(|road| road.points.iter().copied());
    let first = points.next()?;
    let (mut low, mut high) = (first, first);
    for [x, y] in points {
        low = [low[0].min(x), low[1].min(y)];
        high = [high[0].max(x), high[1].max(y)];
    }
    let side = (high[0] - low[0]).max(high[1] - low[1]);
    if !side.is_finite() || side <= 0.0 {
        return None;
    }
    let width = side * (1.0 + 2.0 * FIT_MARGIN);
    let middle = [(low[0] + high[0]) / 2.0, (low[1] + high[1]) / 2.0];
    Some(Calibration { origin: [middle[0] - width / 2.0, middle[1] - width / 2.0], width })
}

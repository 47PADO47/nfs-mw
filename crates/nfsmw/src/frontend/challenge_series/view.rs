//! The map picture of an event: which part of the 1024 x 1024 picture the screen shows and how the view moves when the
//! selection changes. The original streams `TRACK_MAP_<event>` and eases a zoom and a pan towards the race's bounding
//! box over a second (`UITrackMapStreamer::ZoomToTrack`, `PanToTrack`, `UpdateMap`).

/// How long the view takes to move to a new event (`ZoomCubic`, `PanCubic`: `SetDuration(1.0)`).
pub const EASE_SECS: f32 = 1.0;
/// Margin added on every side of a race's bounding box, in picture units (`CalcBoundsForRace`).
const MARGIN: f32 = 0.125;

/// A square crop of the picture: its centre and half side, in picture units (0 to 1, the top left is the origin).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Crop {
    pub centre: [f32; 2],
    pub half: f32,
}

impl Crop {
    /// The whole picture.
    pub const WHOLE: Crop = Crop { centre: [0.5, 0.5], half: 0.5 };

    /// The crop that shows `points` with the margin: the square about the box's centre that holds its longer side
    /// (`UpdateMap`), moved back inside the picture when it would run off an edge **[ours]** (the original lets the
    /// view run off the picture and draws the edge's colour there).
    pub fn around(points: &[[f32; 2]]) -> Crop {
        let Some(first) = points.first() else { return Crop::WHOLE };
        let (mut min, mut max) = (*first, *first);
        for p in points {
            for axis in 0..2 {
                min[axis] = min[axis].min(p[axis]);
                max[axis] = max[axis].max(p[axis]);
            }
        }
        let half = ((max[0] - min[0]).max(max[1] - min[1]) * 0.5 + MARGIN).min(0.5);
        let inside = |axis: usize| ((min[axis] + max[axis]) * 0.5).clamp(half, 1.0 - half);
        Crop { centre: [inside(0), inside(1)], half }
    }

    /// The texture rectangle `[u0, v0, u1, v1]` of the crop.
    pub fn uv(self) -> [f32; 4] {
        let [x, y] = self.centre;
        [x - self.half, y - self.half, x + self.half, y + self.half]
    }

    fn lerp(self, to: Crop, t: f32) -> Crop {
        let mix = |a: f32, b: f32| a + (b - a) * t;
        Crop {
            centre: [mix(self.centre[0], to.centre[0]), mix(self.centre[1], to.centre[1])],
            half: mix(self.half, to.half),
        }
    }
}

/// The view while it moves: from the crop it was at to the crop it is going to.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Zoom {
    from: Crop,
    to: Crop,
    elapsed: f32,
}

impl Zoom {
    /// A view at rest on `crop`.
    pub fn at(crop: Crop) -> Self {
        Self { from: crop, to: crop, elapsed: EASE_SECS }
    }

    /// Starts moving from where the view is now to `crop`.
    pub fn go_to(&mut self, crop: Crop) {
        if crop == self.to {
            return;
        }
        self.from = self.now();
        self.to = crop;
        self.elapsed = 0.0;
    }

    pub fn tick(&mut self, dt: f32) {
        self.elapsed = (self.elapsed + dt).min(EASE_SECS);
    }

    /// Where the view is now: a cubic ease between the two crops.
    pub fn now(&self) -> Crop {
        let t = (self.elapsed / EASE_SECS).clamp(0.0, 1.0);
        self.from.lerp(self.to, t * t * (3.0 - 2.0 * t))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_single_point_zooms_to_a_quarter_of_the_picture() {
        let crop = Crop::around(&[[0.5, 0.5]]);
        assert_eq!(crop.half, 0.125);
        assert_eq!(crop.uv(), [0.375, 0.375, 0.625, 0.625]);
    }

    #[test]
    fn the_longer_side_sets_the_square() {
        let crop = Crop::around(&[[0.3, 0.4], [0.5, 0.45]]);
        assert!((crop.half - 0.225).abs() < 1e-6, "{crop:?}");
        assert!((crop.centre[0] - 0.4).abs() < 1e-6 && (crop.centre[1] - 0.425).abs() < 1e-6);
    }

    #[test]
    fn a_route_near_an_edge_is_moved_inside_and_a_huge_one_shows_the_whole_picture() {
        let edge = Crop::around(&[[0.02, 0.5]]);
        assert_eq!(edge.uv()[0], 0.0);
        assert_eq!(Crop::around(&[[0.0, 0.0], [1.0, 1.0]]), Crop::WHOLE);
        assert_eq!(Crop::around(&[]), Crop::WHOLE);
    }

    #[test]
    fn the_view_eases_from_the_old_crop_to_the_new_one_in_a_second() {
        let target = Crop { centre: [0.25, 0.25], half: 0.125 };
        let mut zoom = Zoom::at(Crop::WHOLE);
        zoom.go_to(target);
        assert_eq!(zoom.now(), Crop::WHOLE);
        zoom.tick(0.5);
        let mid = zoom.now();
        assert!((mid.half - 0.3125).abs() < 1e-6, "half way is the midpoint of a symmetric ease: {mid:?}");
        zoom.tick(0.6);
        assert_eq!(zoom.now(), target);
    }

    #[test]
    fn a_new_target_mid_move_starts_from_where_the_view_is() {
        let mut zoom = Zoom::at(Crop::WHOLE);
        zoom.go_to(Crop { centre: [0.2, 0.2], half: 0.1 });
        zoom.tick(0.25);
        let here = zoom.now();
        zoom.go_to(Crop { centre: [0.8, 0.8], half: 0.2 });
        assert_eq!(zoom.now(), here);
    }
}

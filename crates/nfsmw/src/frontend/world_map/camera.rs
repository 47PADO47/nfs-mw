//! The map's view: zoom and pan over the picture. The picture is square and runs 0..1 on both axes
//! (`blackbox_minimap::Calibration::to_map`). The viewport shows the square `centre ± 0.5 / zoom` of it, so at zoom 1
//! ("Zoom All") the whole picture fits and the view cannot leave it. The zoom has four levels, as the executable's
//! (`docs/provenance/world-map-ghidra.md`).

/// The zoom of each level, from Zoom All.
pub const ZOOMS: [f32; 4] = [1.0, 2.0, 3.3, 5.0];
#[cfg(test)]
pub(super) const MAX_ZOOM: f32 = ZOOMS[ZOOMS.len() - 1];

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Camera {
    /// An index into [`ZOOMS`]; 0 is Zoom All.
    level: usize,
    /// The part of a scroll that is not a whole level yet, so a fine wheel still steps the zoom.
    carry: f32,
    /// The picture point at the middle of the view, 0..1 on both axes.
    pub centre: [f32; 2],
}

impl Default for Camera {
    fn default() -> Self {
        Self::zoom_all()
    }
}

impl Camera {
    /// "Zoom All": the whole picture.
    pub fn zoom_all() -> Self {
        Self { level: 0, carry: 0.0, centre: [0.5, 0.5] }
    }

    /// A view at a zoom level, on a centre (clamped to the picture).
    #[cfg(test)]
    pub(crate) fn at_level(level: usize, centre: [f32; 2]) -> Self {
        let mut camera = Self { level: level.min(ZOOMS.len() - 1), carry: 0.0, centre };
        camera.clamp_centre();
        camera
    }

    /// How far in the view is: 1 shows the whole picture; larger is closer.
    pub fn zoom(&self) -> f32 {
        ZOOMS[self.level]
    }

    /// Zooms by `lines` scroll lines (positive zooms in), one level per whole line, keeping the view inside the
    /// picture.
    pub fn step(&mut self, lines: f32) {
        if lines == 0.0 || !lines.is_finite() {
            return;
        }
        self.carry += lines;
        let whole = self.carry.trunc();
        self.carry -= whole;
        let level = (self.level as i64 + whole as i64).clamp(0, ZOOMS.len() as i64 - 1);
        self.level = level as usize;
        self.clamp_centre();
    }

    /// Pans by `delta`, in view units (a full view width is 1.0), so the picture moves with the drag.
    pub fn pan(&mut self, delta: [f32; 2]) {
        self.centre[0] += delta[0] / self.zoom();
        self.centre[1] += delta[1] / self.zoom();
        self.clamp_centre();
    }

    /// Picture units to view units: `(0, 0)` is the view's top left, `(1, 1)` its bottom right.
    pub fn to_view(self, map: [f32; 2]) -> [f32; 2] {
        let origin = self.origin();
        [(map[0] - origin[0]) * self.zoom(), (map[1] - origin[1]) * self.zoom()]
    }

    /// The texture crop of the picture the view shows, `[u0, v0, u1, v1]`.
    pub fn uv(&self) -> [f32; 4] {
        let half = 0.5 / self.zoom();
        let [x, y] = self.centre;
        [x - half, y - half, x + half, y + half]
    }

    fn origin(&self) -> [f32; 2] {
        let half = 0.5 / self.zoom();
        [self.centre[0] - half, self.centre[1] - half]
    }

    fn clamp_centre(&mut self) {
        let half = 0.5 / self.zoom();
        for c in &mut self.centre {
            *c = c.clamp(half, 1.0 - half);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_zoom_steps_through_four_levels() {
        let mut camera = Camera::zoom_all();
        let mut seen = vec![camera.zoom()];
        for _ in 0..3 {
            camera.step(1.0);
            seen.push(camera.zoom());
        }
        assert_eq!(seen, ZOOMS.to_vec());
        camera.step(1.0);
        assert_eq!(camera.zoom(), MAX_ZOOM, "the top level holds");
    }

    #[test]
    fn a_fine_scroll_adds_up_to_a_level() {
        let mut camera = Camera::zoom_all();
        for _ in 0..4 {
            camera.step(0.25);
        }
        assert_eq!(camera.zoom(), ZOOMS[1]);
    }

    #[test]
    fn the_crop_is_the_view_square() {
        assert_eq!(Camera::zoom_all().uv(), [0.0, 0.0, 1.0, 1.0]);
        assert_eq!(Camera::at_level(1, [0.5, 0.5]).uv(), [0.25, 0.25, 0.75, 0.75]);
    }

    #[test]
    fn zooming_out_stops_at_zoom_all_and_a_level_keeps_the_view_inside() {
        let mut camera = Camera::zoom_all();
        camera.step(-100.0);
        assert_eq!(camera.zoom(), 1.0);
        camera.step(1000.0);
        let half = 0.5 / camera.zoom();
        assert!(camera.centre.iter().all(|c| *c >= half - 1e-6 && *c <= 1.0 - half + 1e-6));
    }
}

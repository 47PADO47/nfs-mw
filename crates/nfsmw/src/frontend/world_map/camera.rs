//! The map's view: zoom and pan over the picture. The picture is square and runs 0..1 on both axes
//! (`blackbox_minimap::Calibration::to_map`). The viewport shows the square `centre ± 0.5 / zoom` of it, so at zoom 1
//! ("Zoom All") the whole picture fits and the view cannot leave it.

/// The most the map zooms in.
pub const MAX_ZOOM: f32 = 8.0;
/// The zoom factor per scroll line.
const STEP: f32 = 1.25;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Camera {
    /// 1 shows the whole picture; larger is closer.
    pub zoom: f32,
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
        Self { zoom: 1.0, centre: [0.5, 0.5] }
    }

    /// Zooms by `lines` scroll lines (positive zooms in), keeping the view inside the picture.
    pub fn step(&mut self, lines: f32) {
        if lines == 0.0 || !lines.is_finite() {
            return;
        }
        self.zoom = (self.zoom * STEP.powf(lines)).clamp(1.0, MAX_ZOOM);
        self.clamp_centre();
    }

    /// Pans by `delta`, in view units (a full view width is 1.0), so the picture moves with the drag.
    pub fn pan(&mut self, delta: [f32; 2]) {
        self.centre[0] += delta[0] / self.zoom;
        self.centre[1] += delta[1] / self.zoom;
        self.clamp_centre();
    }

    /// Picture units to view units: `(0, 0)` is the view's top left, `(1, 1)` its bottom right.
    pub fn to_view(&self, map: [f32; 2]) -> [f32; 2] {
        let origin = self.origin();
        [(map[0] - origin[0]) * self.zoom, (map[1] - origin[1]) * self.zoom]
    }

    fn origin(&self) -> [f32; 2] {
        let half = 0.5 / self.zoom;
        [self.centre[0] - half, self.centre[1] - half]
    }

    fn clamp_centre(&mut self) {
        let half = 0.5 / self.zoom;
        for c in &mut self.centre {
            *c = c.clamp(half, 1.0 - half);
        }
    }
}

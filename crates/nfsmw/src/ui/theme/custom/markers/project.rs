//! From the world to the canvas (`docs/specs/custom-hud/world-markers.md` sections 1, 3 and 4): the projection through
//! the frame's gameplay camera, the edge arrow's place for a target outside the view, and the steadying of label
//! positions against the camera's shake.
//!
//! Canvas units are the presenter's: the window is [`window`](super::super::layout::window) wide, 480 high, with the
//! origin at its centre and y down. The camera's viewport is the same window in logical pixels, so a pixel maps onto the
//! canvas by the share of the viewport it is at.

use blackbox_ui::Rect;
use glam::{Mat4, Vec2, Vec4};

use crate::hud::HudCamera;
use crate::ui::present::{FENG_HEIGHT, Screen};

use super::super::layout::SAFE_INSET;

/// A label's position lags its target by this time constant, seconds: a one-pole low-pass of about 50 ms.
pub const LABEL_LAG: f32 = 0.05;
/// A label whose target moved farther than this in one frame goes straight there, canvas units: the marker changed
/// place (it left the edge, or a new one took the id), which is not shake to smooth.
pub const LABEL_JUMP: f32 = 60.0;

/// Where a world point lands this frame.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Placement {
    /// In front of the camera and inside the window: the canvas point.
    OnScreen(Vec2),
    /// Outside the window, or behind the camera: the unit direction on the canvas (y down) from the window's centre
    /// toward the target. Behind the camera it is not mirrored: a target behind and to the left points left.
    OffScreen(Vec2),
}

/// Where `point` (render space) lands through `camera` on the canvas of `window`. A point closer than `margin` canvas
/// units to the window's edge counts as off screen, so a mark is never cut by the edge. `None` before the camera has a
/// viewport.
pub fn place(camera: &HudCamera, window: Rect, point: [f32; 3], margin: f32) -> Option<Placement> {
    let [width, height] = camera.viewport;
    if !width.is_finite() || !height.is_finite() || width <= 0.0 || height <= 0.0 {
        return None;
    }
    let inside = window.inset(margin, margin);
    let on_screen = camera
        .project(point)
        .map(|(pixel, _)| to_canvas(pixel, camera.viewport, window))
        .filter(|at| inside.contains(at.x, at.y));
    if let Some(at) = on_screen {
        return Some(Placement::OnScreen(at));
    }
    let clip = Mat4::from_cols_array_2d(&camera.view_proj) * Vec4::new(point[0], point[1], point[2], 1.0);
    // The clip x and y before the divide by w: in front of the camera they point where the projected point is, and
    // behind it (w negative) they keep the side the target is on, where the divided ones would flip it.
    let toward = Vec2::new(clip.x * window.w / 2.0, -clip.y * window.h / 2.0);
    // Straight behind, with no side to take: point down, toward the back of the car.
    let direction = toward.try_normalize().unwrap_or(Vec2::Y);
    Some(Placement::OffScreen(direction))
}

/// A viewport pixel (from the top left) as a canvas point of `window`.
pub fn to_canvas(pixel: [f32; 2], viewport: [f32; 2], window: Rect) -> Vec2 {
    let x = (pixel[0] / viewport[0] - 0.5) * window.w + window.x;
    let y = (pixel[1] / viewport[1] - 0.5) * window.h + window.y;
    Vec2::new(x, y)
}

/// The rectangle an edge arrow's centre stays in: the window less the safe inset and `half` (the arrow's half size).
pub fn edge_rect(window: Rect, half: f32) -> Rect {
    let inset = SAFE_INSET * FENG_HEIGHT + half;
    window.inset(inset, inset)
}

/// Where an edge arrow pointing along `direction` stands: on the ellipse inscribed in `edge`, along the direction from
/// its centre, so the arrows ring the view at an even inset on any window shape.
pub fn edge_point(edge: Rect, direction: Vec2) -> Vec2 {
    let (a, b) = (edge.w / 2.0, edge.h / 2.0);
    let centre = Vec2::new(edge.x, edge.y);
    if a <= 0.0 || b <= 0.0 {
        return centre;
    }
    let reach = ((direction.x / a).powi(2) + (direction.y / b).powi(2)).sqrt();
    if reach.is_nan() || reach <= f32::EPSILON {
        return centre;
    }
    let along = direction / reach;
    centre + Vec2::new(along.x.clamp(-a, a), along.y.clamp(-b, b))
}

/// A label's position one step after `previous` toward `target`, `dt` seconds later: the one-pole low-pass of
/// [`LABEL_LAG`]. A jump past [`LABEL_JUMP`] is taken at once, and no time passing holds the label still.
pub fn follow(previous: Vec2, target: Vec2, dt: f32) -> Vec2 {
    if previous.distance(target) > LABEL_JUMP || !previous.is_finite() {
        return target;
    }
    if dt.is_nan() || dt <= 0.0 {
        return previous;
    }
    let k = 1.0 - (-dt / LABEL_LAG).exp();
    previous + (target - previous) * k
}

/// How a window's canvas maps onto its physical pixels: the pixels per canvas unit and the window's size in pixels.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PixelGrid {
    pub per_unit: f32,
    pub size: Vec2,
}

impl PixelGrid {
    /// The grid of `screen`: 480 canvas units fill its height of `height * pixels_per_point` pixels.
    pub fn of(screen: Screen) -> Self {
        let ppp = match screen.pixels_per_point.is_finite() && screen.pixels_per_point > 0.0 {
            true => screen.pixels_per_point,
            false => 1.0,
        };
        Self { per_unit: screen.scale() * ppp, size: Vec2::new(screen.width, screen.height) * ppp }
    }

    /// `at` (canvas units from the window's centre) moved onto the nearest whole pixel. The grid starts at the window's
    /// edge, so on a window an odd number of pixels across the centre is half a pixel off the grid.
    pub fn snap(self, at: Vec2) -> Vec2 {
        if self.per_unit.is_nan() || self.per_unit <= 0.0 {
            return at;
        }
        let half = self.size / 2.0;
        let pixel = (at * self.per_unit + half).round();
        (pixel - half) / self.per_unit
    }
}

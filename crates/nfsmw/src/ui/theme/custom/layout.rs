//! Where the Custom HUD's elements sit on any window shape (ADR 0013; `docs/specs/custom-hud/layout.md`).
//!
//! Units are the presenter's canvas units: the FEng screen is 480 units high and the window's height fills it
//! (`Screen::scale`), x is centred on the window and y points down, so the canvas centre is the origin.
//!
//! - **The content box.** The HUD is laid out in a box as tall as the window and at most 16:9 wide, centred. On 16:9
//!   and narrower windows the box is the whole window; on 21:9 and 32:9 it is the central 16:9 part, so the HUD does
//!   not stretch across an ultrawide window and the cluster anchors to the box's edge, not the window's.
//! - **The safe area.** Every anchor is inset from the box's edge by [`SAFE_INSET`] of its height, at every scale.
//! - **The scale.** `[custom_hud] scale`, times a narrow-window factor: below 16:9 the HUD shrinks with the aspect,
//!   down to [`NARROW_SCALE_FLOOR`] (a 4:3 window draws at 0.8). An element is scaled about its anchor, so a larger HUD
//!   grows inwards and never leaves the safe area.
//!
//! - **The top-centre stack.** The map sits at the top centre (`map/place.rs`). A frame made with
//!   [`HudBox::with_top_clearance`] pushes every top-centre element down by the map's height, so the race strip, its
//!   lines and the turn chip stack under the map, and [`HudBox::place_clear`] pushes a card only as far as it needs to
//!   clear the map.
//!
//! World markers are the exception: they point at the world, so they use the whole window ([`window`]).

use blackbox_ui::Rect;

use crate::ui::present::{FENG_HEIGHT, Screen};

/// The FEng canvas as a window: 640 by 480 points. The layout of a window with no size, and the feed's screen before
/// the first frame.
pub const CANVAS: Screen = Screen { width: 640.0, height: 480.0, pixels_per_point: 1.0 };
/// The widest the content box gets, as width over height.
pub const MAX_ASPECT: f32 = 16.0 / 9.0;
/// The safe inset, as a fraction of the box height: about 17 units on every side.
pub const SAFE_INSET: f32 = 0.035;
/// The least the narrow-window factor shrinks the HUD to.
pub const NARROW_SCALE_FLOOR: f32 = 0.8;

/// A point of the safe area an element is placed from.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Anchor {
    TopLeft,
    TopCentre,
    TopRight,
    BottomLeft,
    BottomRight,
}

impl Anchor {
    /// Every anchor.
    #[cfg(test)]
    pub const ALL: [Anchor; 5] =
        [Anchor::TopLeft, Anchor::TopCentre, Anchor::TopRight, Anchor::BottomLeft, Anchor::BottomRight];

    /// The side of the box the anchor is on, per axis: -1 the left (top), 0 the middle, 1 the right (bottom).
    fn sides(self) -> (f32, f32) {
        match self {
            Anchor::TopLeft => (-1.0, -1.0),
            Anchor::TopCentre => (0.0, -1.0),
            Anchor::TopRight => (1.0, -1.0),
            Anchor::BottomLeft => (-1.0, 1.0),
            Anchor::BottomRight => (1.0, 1.0),
        }
    }
}

/// The window as canvas units from the canvas centre. A window with no size is the FEng canvas.
pub fn window(screen: Screen) -> Rect {
    let aspect = aspect(screen);
    Rect::new(0.0, 0.0, FENG_HEIGHT * aspect, FENG_HEIGHT)
}

/// The width over the height of `screen`, or the canvas's 4:3 for a window with no size.
fn aspect(screen: Screen) -> f32 {
    let aspect = screen.width / screen.height;
    if !aspect.is_finite() || aspect <= 0.0 {
        return CANVAS.width / CANVAS.height;
    }
    aspect
}

/// The HUD's frame for one window and one HUD scale: the content box, its safe area and the effective scale.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct HudBox {
    content: Rect,
    safe: Rect,
    scale: f32,
    /// How far down the map takes the top centre, in canvas units (not scaled by the HUD scale: the map is not).
    top_clearance: f32,
}

impl HudBox {
    /// The frame of `screen` at `[custom_hud] scale` `hud_scale`. A scale that is not a positive number counts as 1.
    pub fn new(screen: Screen, hud_scale: f32) -> Self {
        let aspect = aspect(screen);
        let content = Rect::new(0.0, 0.0, FENG_HEIGHT * aspect.min(MAX_ASPECT), FENG_HEIGHT);
        let inset = SAFE_INSET * FENG_HEIGHT;
        let safe = content.inset(inset, inset);
        let hud_scale = match hud_scale.is_finite() && hud_scale > 0.0 {
            true => hud_scale,
            false => 1.0,
        };
        let narrow = (aspect / MAX_ASPECT).clamp(NARROW_SCALE_FLOOR, 1.0);
        Self { content, safe, scale: hud_scale * narrow, top_clearance: 0.0 }
    }

    /// This frame with the top centre cleared down by `units` canvas units (the map's height and the gap under it):
    /// [`HudBox::place`] moves a top-centre element down by it. A negative or non-finite value counts as none.
    pub fn with_top_clearance(self, units: f32) -> Self {
        let top_clearance = match units.is_finite() && units > 0.0 {
            true => units,
            false => 0.0,
        };
        Self { top_clearance, ..self }
    }

    /// How far down the top centre is cleared, canvas units.
    pub fn top_clearance(&self) -> f32 {
        self.top_clearance
    }

    /// The content box: the window, or its central 16:9 part on a wider window.
    pub fn content(&self) -> Rect {
        self.content
    }

    /// The content box less the safe inset: no element is placed outside it.
    pub fn safe(&self) -> Rect {
        self.safe
    }

    /// The safe inset, in canvas units. The same at every HUD scale.
    #[cfg(test)]
    pub fn inset(&self) -> f32 {
        SAFE_INSET * FENG_HEIGHT
    }

    /// The effective scale: `[custom_hud] scale` times the narrow-window factor. Every size of a Custom element is
    /// multiplied by it.
    pub fn scale(&self) -> f32 {
        self.scale
    }

    /// The point `anchor` names on the safe area's edge (or its centre).
    pub fn point(&self, anchor: Anchor) -> (f32, f32) {
        let (sx, sy) = anchor.sides();
        (self.safe.x + sx * self.safe.w / 2.0, self.safe.y + sy * self.safe.h / 2.0)
    }

    /// The rectangle of an element `size` units across at scale 1 (width, height), placed at `anchor` and moved by
    /// `offset` units at scale 1 (x right, y down). The side of the element that faces the anchor's side lands on the
    /// anchor (its right edge on a right anchor, its centre on a centre one), and both the size and the offset are
    /// multiplied by the scale, so the element grows about its anchor. A top-centre element is moved down by the frame's
    /// top clearance (the map above it).
    pub fn place(&self, anchor: Anchor, offset: (f32, f32), size: (f32, f32)) -> Rect {
        let rect = self.place_plain(anchor, offset, size);
        match anchor {
            Anchor::TopCentre => Rect::new(rect.x, rect.y + self.top_clearance, rect.w, rect.h),
            _ => rect,
        }
    }

    /// As [`HudBox::place`], but a top-centre element is pushed down only as far as it needs to clear the map: it stays
    /// where `offset` puts it unless its top would be inside the cleared band. For the cards that belong in the upper
    /// third (the countdown, the end of a chase).
    pub fn place_clear(&self, anchor: Anchor, offset: (f32, f32), size: (f32, f32)) -> Rect {
        let rect = self.place_plain(anchor, offset, size);
        if anchor != Anchor::TopCentre {
            return rect;
        }
        let shift = (self.safe.min_y() + self.top_clearance - rect.min_y()).max(0.0);
        Rect::new(rect.x, rect.y + shift, rect.w, rect.h)
    }

    /// [`HudBox::place`] without the top clearance.
    fn place_plain(&self, anchor: Anchor, offset: (f32, f32), size: (f32, f32)) -> Rect {
        let (sx, sy) = anchor.sides();
        let (px, py) = self.point(anchor);
        let (w, h) = (size.0 * self.scale, size.1 * self.scale);
        // The element's centre: the anchor point, moved in by half the element on a side anchor, then by the offset.
        let x = px - sx * w / 2.0 + offset.0 * self.scale;
        let y = py - sy * h / 2.0 + offset.1 * self.scale;
        Rect::new(x, y, w, h)
    }
}

impl Default for HudBox {
    /// The frame of the FEng canvas at scale 1.
    fn default() -> Self {
        Self::new(CANVAS, 1.0)
    }
}

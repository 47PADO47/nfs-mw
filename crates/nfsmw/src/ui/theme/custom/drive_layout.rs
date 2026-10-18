//! Where the Custom drive cluster sits: one card in the bottom-right corner of the window. The arc-style tachometer
//! sweeps from the lower-left to the upper-right of the card, with the speed readout below the arc centre, the gear
//! number at the right, and the meter bars (nitrous, boost, speed breaker) along the bottom.
//!
//! The arc is a circular arc whose centre is below the card. Points on the arc are computed from the centre, radius
//! and angle range. Numbers 1–N (thousands of RPM) sit outside the arc; tick marks point outward from the arc line.

use blackbox_ui::Rect;

use crate::ui::present::Screen;

/// The card's size, and the inset from the window's edges, in FEng units.
const CARD_WIDTH: f32 = 220.0;
const CARD_HEIGHT: f32 = 115.0;
const PAD: f32 = 10.0;
/// Never closer to the window's edge than this many points.
const EDGE: f32 = 6.0;

/// The arc's geometry, in card-local coordinates (origin at top-left, x right, y down).
const ARC_CX: f32 = 115.0;
const ARC_CY: f32 = 145.0;
const ARC_R: f32 = 130.0;
/// The angle (standard math, CCW from +x, y-up) where the "1" tick sits (lower-left of the card).
const ARC_START_DEG: f32 = 155.0;
/// The angle where the last tick sits (upper-right of the card).
const ARC_END_DEG: f32 = 48.0;
/// The size of each dot forming the arc line, in FEng units.
pub(super) const ARC_DOT: f32 = 1.2;
/// How many dots form the full arc line.
pub(super) const ARC_DOTS: usize = 90;
/// The major tick mark's outward length and the minor tick's.
pub(super) const TICK_MAJOR: f32 = 6.0;
/// Tick mark thickness.
pub(super) const TICK_W: f32 = 1.0;
/// How far outside the arc the number labels sit.
const NUMBER_OFFSET: f32 = 12.0;

/// The meter rows: how the label and bar share the row's width, height of the bar, and the pitch between rows.
const METER_TOP: f32 = 96.0;
const METER_PITCH: f32 = 9.0;
const LABEL_WIDTH: f32 = 30.0;
const METER_GAP: f32 = 4.0;
const BAR_HEIGHT: f32 = 4.0;

/// The window's bottom-right corner in FEng units from the canvas centre.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(in crate::ui::theme) struct Corner {
    pub right: f32,
    pub bottom: f32,
}

impl Corner {
    pub(super) fn of(screen: Screen) -> Self {
        let points = screen.scale();
        if points.is_nan() || points <= 0.0 {
            return Self { right: 320.0 - EDGE, bottom: 240.0 - EDGE };
        }
        Self { right: (screen.width / 2.0 - EDGE) / points, bottom: (screen.height / 2.0 - EDGE) / points }
    }

    fn left(self) -> f32 {
        self.right - CARD_WIDTH
    }

    fn top(self) -> f32 {
        self.bottom - CARD_HEIGHT
    }
}

/// The card's background.
pub(super) fn card(c: Corner) -> Rect {
    at(c.left(), c.top(), CARD_WIDTH, CARD_HEIGHT)
}

/// A point on the arc at fraction `t` (0 = start/"1", 1 = end/last tick). Returns `(x, y)` in FEng coordinates.
pub(super) fn arc_point(c: Corner, t: f32) -> (f32, f32) {
    let angle_deg = ARC_START_DEG + t * (ARC_END_DEG - ARC_START_DEG);
    let angle = angle_deg.to_radians();
    let x = c.left() + ARC_CX + ARC_R * angle.cos();
    let y = c.top() + ARC_CY - ARC_R * angle.sin();
    (x, y)
}

/// The outward direction (pointing away from the arc centre) at fraction `t`, as a unit vector `(dx, dy)`.
pub(super) fn arc_outward(t: f32) -> (f32, f32) {
    let angle_deg = ARC_START_DEG + t * (ARC_END_DEG - ARC_START_DEG);
    let angle = angle_deg.to_radians();
    (angle.cos(), -angle.sin())
}

/// The rectangle for a dot at position `(x, y)` on the arc line.
pub(super) fn arc_dot_rect(x: f32, y: f32) -> Rect {
    Rect::new(x, y, ARC_DOT, ARC_DOT)
}

/// The rectangle for a tick mark at arc fraction `t`, extending `length` outward from the arc line.
pub(super) fn tick_rect(c: Corner, t: f32, length: f32) -> Rect {
    let (px, py) = arc_point(c, t);
    let (dx, dy) = arc_outward(t);
    let cx = px + dx * (length / 2.0);
    let cy = py + dy * (length / 2.0);
    let angle_deg = ARC_START_DEG + t * (ARC_END_DEG - ARC_START_DEG);
    let angle = angle_deg.to_radians();
    let abs_cos = angle.cos().abs();
    let abs_sin = angle.sin().abs();
    let w = TICK_W * abs_sin + length * abs_cos;
    let h = TICK_W * abs_cos + length * abs_sin;
    Rect::new(cx, cy, w, h)
}

/// The centre of the number label at tick index `i` (0-based, so tick 0 = "1"). `ticks` is the total number of ticks.
pub(super) fn number_pos(c: Corner, i: usize, ticks: usize) -> (f32, f32) {
    let t = tick_fraction(i, ticks);
    let (px, py) = arc_point(c, t);
    let (dx, dy) = arc_outward(t);
    (px + dx * NUMBER_OFFSET, py + dy * NUMBER_OFFSET)
}

/// The fraction along the arc for tick index `i` of `ticks` total (evenly spaced).
pub(super) fn tick_fraction(i: usize, ticks: usize) -> f32 {
    if ticks <= 1 {
        return 0.0;
    }
    i as f32 / (ticks - 1) as f32
}

/// The speed text box, centred under the arc in the lower portion of the card.
pub(super) fn speed(c: Corner) -> Rect {
    at(c.left() + 70.0, c.top() + 52.0, 100.0, 38.0)
}

/// The unit label box, under the speed.
pub(super) fn unit(c: Corner) -> Rect {
    at(c.left() + 90.0, c.top() + 72.0, 50.0, 14.0)
}

/// The gear number, to the right of the speed readout.
pub(super) fn gear(c: Corner) -> Rect {
    at(c.left() + CARD_WIDTH - 40.0, c.top() + 50.0, 30.0, 40.0)
}

/// The label of meter row `row` (0 nitrous, 1 boost, 2 speed breaker).
pub(super) fn meter_label(c: Corner, row: usize) -> Rect {
    at(c.left() + PAD, meter_top(c, row) - 3.0, LABEL_WIDTH, 10.0)
}

/// The bar of meter row `row`.
pub(super) fn meter_bar(c: Corner, row: usize) -> Rect {
    let x = c.left() + PAD + LABEL_WIDTH + METER_GAP;
    let width = CARD_WIDTH - 2.0 * PAD - LABEL_WIDTH - METER_GAP;
    at(x, meter_top(c, row), width, BAR_HEIGHT)
}

fn meter_top(c: Corner, row: usize) -> f32 {
    c.top() + METER_TOP + row as f32 * METER_PITCH
}

/// The rectangle whose top-left corner is `(left, top)`, as a `Rect` (which is centred).
pub(super) fn at(left: f32, top: f32, width: f32, height: f32) -> Rect {
    Rect::new(left + width / 2.0, top + height / 2.0, width, height)
}

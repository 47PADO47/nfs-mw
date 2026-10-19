//! Where the Custom drive cluster sits: one card in the bottom-right corner of the window. The corner is in FEng units
//! from the canvas centre (y down), the same units the presenter lays out in, and it is taken from the window, so the
//! card hugs the screen's edge whatever the HUD layout is.
//!
//! From the top the card holds the rev gauge, the speed and the gear box beside it, the gear pips under the gear box,
//! and three slim meter rows (nitrous, boost, speed breaker). The tachometer draws the card, and the other slots draw
//! their parts.

use blackbox_ui::Rect;

use crate::ui::present::Screen;

/// The card's size and padding, in FEng units.
const CARD_WIDTH: f32 = 210.0;
const CARD_HEIGHT: f32 = 112.0;
const PAD: f32 = 16.0;
const INNER: f32 = CARD_WIDTH - 2.0 * PAD;
/// Never closer to the window's edge than this many points.
const EDGE: f32 = 6.0;

/// The rev gauge's segments: how many, their height, and the gap between them.
pub(super) const SEGMENTS: usize = 32;
const SEGMENT_HEIGHT: f32 = 4.0;
const SEGMENT_GAP: f32 = 1.0;
/// The gear pips: one for reverse, neutral and each of the six forward gears, with the gap between them.
pub(super) const PIPS: usize = 8;
const PIP_HEIGHT: f32 = 3.0;
const PIP_GAP: f32 = 2.0;
/// The gear box's width, which the pips share, and its height.
const GEAR_WIDTH: f32 = 72.0;
const GEAR_HEIGHT: f32 = 50.0;
/// The meter rows: their top in the card, their pitch, and how the label and the bar share the row's width.
const METER_TOP: f32 = 82.0;
const METER_PITCH: f32 = 9.0;
const LABEL_WIDTH: f32 = 36.0;
const METER_GAP: f32 = 6.0;
const METER_BAR_HEIGHT: f32 = 4.0;

/// The window's bottom-right corner in FEng units from the canvas centre. The card is laid out from it.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(in crate::ui::theme) struct Corner {
    pub right: f32,
    pub bottom: f32,
}

impl Corner {
    /// The corner of `screen`: the window's edges, inset by [`EDGE`] points, in FEng units. A window with no scale falls
    /// back to the canvas's own corner.
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

    fn inner_left(self) -> f32 {
        self.left() + PAD
    }
}

/// The card's background.
pub(super) fn card(c: Corner) -> Rect {
    at(c.left(), c.top(), CARD_WIDTH, CARD_HEIGHT)
}

/// The rev gauge's segment `index`, counting from the left. The segments share the inner width.
pub(super) fn segment(c: Corner, index: usize) -> Rect {
    let width = (INNER - (SEGMENTS - 1) as f32 * SEGMENT_GAP) / SEGMENTS as f32;
    let x = c.inner_left() + index as f32 * (width + SEGMENT_GAP);
    at(x, c.top() + PAD, width, SEGMENT_HEIGHT)
}

/// The speed's box, under the rev gauge on the left.
pub(super) fn speed(c: Corner) -> Rect {
    at(c.inner_left(), c.top() + 22.0, 110.0, 38.0)
}

/// The unit's box, beside the speed's digits.
pub(super) fn unit(c: Corner) -> Rect {
    at(c.inner_left() + 52.0, c.top() + 40.0, 60.0, 10.0)
}

/// The gear box, on the right under the rev gauge. It has no box of its own beyond its fill: the letter sits in it.
pub(super) fn gear(c: Corner) -> Rect {
    at(c.inner_left() + INNER - GEAR_WIDTH, c.top() + 22.0, GEAR_WIDTH, GEAR_HEIGHT)
}

/// The gear strip's pip `index`, counting from the left, under the gear box (the strip is as wide as the box).
pub(super) fn pip(c: Corner, index: usize) -> Rect {
    let left = c.inner_left() + INNER - GEAR_WIDTH;
    let width = (GEAR_WIDTH - (PIPS - 1) as f32 * PIP_GAP) / PIPS as f32;
    let x = left + index as f32 * (width + PIP_GAP);
    at(x, c.top() + 77.0, width, PIP_HEIGHT)
}

/// The label of meter row `row` (0 nitrous, 1 boost, 2 speed breaker), at the left of the row.
pub(super) fn meter_label(c: Corner, row: usize) -> Rect {
    at(c.inner_left(), meter_top(c, row) - 3.0, LABEL_WIDTH, 10.0)
}

/// The bar of meter row `row`, filling the rest of the row's width.
pub(super) fn meter_bar(c: Corner, row: usize) -> Rect {
    let x = c.inner_left() + LABEL_WIDTH + METER_GAP;
    let width = INNER - LABEL_WIDTH - METER_GAP;
    at(x, meter_top(c, row), width, METER_BAR_HEIGHT)
}

fn meter_top(c: Corner, row: usize) -> f32 {
    c.top() + METER_TOP + row as f32 * METER_PITCH
}

/// The rectangle whose top-left corner is `(left, top)`, as a `Rect` (which is centred).
pub(super) fn at(left: f32, top: f32, width: f32, height: f32) -> Rect {
    Rect::new(left + width / 2.0, top + height / 2.0, width, height)
}

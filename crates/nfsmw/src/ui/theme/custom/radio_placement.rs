//! Where the radio cards sit, in canvas units from the canvas centre.
//!
//! - **The Custom card** ([`card_rect`]): at the top-left anchor of the content box (`layout.rs` `HudBox`), the zone
//!   the layout gives it, sized at the HUD scale and grown with `[custom_hud] text_size`. Its rows' places are
//!   [`row_middle`].
//! - **The Remastered card** ([`anchor`]): the authored spot of the 4:3 HUD, moved and scaled like the HUD's left side by
//!   the HUD layout setting (`hud/viewport.rs`), and kept a few points inside the window. The radio feed carries it
//!   (`radio_feed.rs`); the Custom card no longer reads it.

use blackbox_ui::Rect;

use super::layout::{self, HudBox};
use crate::hud::viewport;
use crate::settings::HudLayout;
use crate::ui::present::Screen;

/// The Custom card's width, the room inside its plate, and its rows' heights at text size 1, all at scale 1.
const CARD_WIDTH: f32 = 240.0;
pub(super) const PAD: f32 = 8.0;
const TITLE_LINE: f32 = 18.0;
const DETAIL_LINE: f32 = 15.0;

/// The card's top-left corner in the authored 4:3 HUD, in FEng units from the centre of the canvas: the left margin of
/// the HUD and the top of the screen.
const AUTHORED_LEFT: f32 = -300.0;
const AUTHORED_TOP: f32 = -228.0;
/// Never closer to the window's edge than this many points.
const EDGE: f32 = 4.0;

/// The card's top-left corner and the viewport scale it is drawn at. `left` and `top` already include the scale: they
/// are the FEng units the component lays out in, which the presenter maps to points.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(in crate::ui::theme) struct Anchor {
    pub left: f32,
    pub top: f32,
    pub scale: f32,
}

/// The card's anchor on `screen` under `layout`. The corner is clamped so the card stays [`EDGE`] points inside the
/// window, as the egui card did (`hud/radio.rs` before T9).
pub(super) fn anchor(screen: Screen, layout: HudLayout) -> Anchor {
    let (scale, offset) = viewport::parameters(screen, layout);
    let left = scale * (AUTHORED_LEFT - offset);
    let top = scale * AUTHORED_TOP;
    let points = screen.scale();
    if points.is_nan() || points <= 0.0 {
        return Anchor { left, top, scale };
    }
    let min_left = (EDGE - screen.width / 2.0) / points;
    let min_top = (EDGE - screen.height / 2.0) / points;
    Anchor { left: left.max(min_left), top: top.max(min_top), scale }
}

/// The heights of the Custom card's three rows at scale 1 and `text_size`: the title, then two detail rows.
fn row_heights(text_size: f32) -> [f32; 3] {
    [TITLE_LINE * text_size, DETAIL_LINE * text_size, DETAIL_LINE * text_size]
}

/// The middle of the Custom card's row `i` below the card's top, at scale 1 and `text_size`.
pub(super) fn row_middle(i: usize, text_size: f32) -> f32 {
    let heights = row_heights(text_size);
    let above: f32 = heights.iter().take(i).sum();
    PAD + above + heights.get(i).copied().unwrap_or(0.0) / 2.0
}

/// The Custom card's rectangle in `frame` at `text_size`, moved `slide` units right (at scale 1) from its place: at
/// the content box's top-left anchor.
pub(super) fn card_rect(frame: &HudBox, text_size: f32, slide: f32) -> Rect {
    let height = 2.0 * PAD + row_heights(text_size).iter().sum::<f32>();
    frame.place(layout::Anchor::TopLeft, (slide, 0.0), (CARD_WIDTH, height))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn screen(width: f32, height: f32) -> Screen {
        Screen { width, height, pixels_per_point: 1.0 }
    }

    /// The anchor's corner in window points, as the presenter places it.
    fn points(at: Anchor, screen: Screen) -> (f32, f32) {
        (screen.width / 2.0 + at.left * screen.scale(), screen.height / 2.0 + at.top * screen.scale())
    }

    #[test]
    fn the_card_follows_the_hud_layout_setting() {
        // 1080p, 16:9: 2.25 points per unit. The PC layout moves the left side out by 120 units.
        let wide = screen(1920.0, 1080.0);
        let pc = anchor(wide, HudLayout::Pc);
        let (left, top) = points(pc, wide);
        assert!((left - 15.0).abs() < 1e-3 && (top - 27.0).abs() < 1e-3 && pc.scale == 1.0);
        // Classic keeps the centred 4:3 HUD: the card sits further in, at the same size.
        let classic = anchor(wide, HudLayout::Classic);
        assert!((points(classic, wide).0 - 285.0).abs() < 1e-3 && classic.scale == pc.scale);
        // Xbox 360 scales the HUD by 0.92 about the centre: a smaller card and a margin that moves in.
        let xbox = anchor(wide, HudLayout::Xbox360);
        assert!((xbox.scale - 0.92).abs() < 1e-6);
        assert!(xbox.left > pc.left && xbox.top > pc.top);
        assert!((points(xbox, wide).0 - (960.0 - 420.0 * 0.92 * 2.25)).abs() < 1e-3);
    }

    #[test]
    fn the_card_stays_on_screen_in_narrow_and_odd_windows() {
        // 4:3 and narrower: the centred, unscaled HUD, and the card never leaves the window.
        let narrow = screen(300.0, 600.0);
        let at = anchor(narrow, HudLayout::Pc);
        assert!((points(at, narrow).0 - EDGE).abs() < 1e-3, "pushed in from the left edge");
        assert!((points(at, narrow).1 - 15.0).abs() < 1e-3, "228 units above the centre");
        let square = screen(480.0, 480.0);
        let (left, top) = points(anchor(square, HudLayout::Xbox360), square);
        assert!(left >= EDGE && top >= EDGE);
        let flat = anchor(screen(1024.0, 768.0), HudLayout::Pc);
        assert_eq!((flat.left, flat.scale), (AUTHORED_LEFT, 1.0));
        let ultrawide = screen(3440.0, 1440.0);
        let (left, _) = points(anchor(ultrawide, HudLayout::Pc), ultrawide);
        assert!((EDGE..400.0).contains(&left), "{left}");
    }

    #[test]
    fn a_window_with_no_height_is_not_clamped_or_divided_by() {
        let flat = anchor(screen(800.0, 0.0), HudLayout::Pc);
        assert!(flat.left.is_finite() && flat.top.is_finite());
    }

    #[test]
    fn the_custom_card_sits_in_the_top_left_corner_of_the_safe_area() {
        for screen in [screen(1920.0, 1080.0), screen(3440.0, 1440.0), screen(1024.0, 768.0)] {
            let frame = HudBox::new(screen, 1.0);
            let card = card_rect(&frame, 1.0, 0.0);
            let safe = frame.safe();
            assert!((card.min_x() - safe.min_x()).abs() < 1e-3 && (card.min_y() - safe.min_y()).abs() < 1e-3);
        }
        let wide = HudBox::new(screen(3440.0, 1440.0), 1.0);
        assert!(
            card_rect(&wide, 1.0, 0.0).min_x() > -240.0 * 3440.0 / 1440.0,
            "the box, not the ultrawide window's edge"
        );
    }

    #[test]
    fn the_custom_card_grows_with_the_scale_and_the_text_size_and_slides_from_the_left() {
        let frame = HudBox::new(screen(1920.0, 1080.0), 1.0);
        let base = card_rect(&frame, 1.0, 0.0);
        let larger = card_rect(&HudBox::new(screen(1920.0, 1080.0), 1.5), 1.0, 0.0);
        assert!((larger.w - base.w * 1.5).abs() < 1e-3 && larger.min_x() == base.min_x(), "grows about its anchor");
        assert!(card_rect(&frame, 1.4, 0.0).h > base.h, "larger text, a taller card");
        assert!((card_rect(&frame, 1.0, -12.0).min_x() - (base.min_x() - 12.0)).abs() < 1e-3);
    }

    #[test]
    fn the_custom_rows_stack_inside_the_padding() {
        let ts = 1.0;
        assert!((row_middle(0, ts) - (PAD + TITLE_LINE / 2.0)).abs() < 1e-4);
        assert!(row_middle(1, ts) > row_middle(0, ts) && row_middle(2, ts) > row_middle(1, ts));
        let height = card_rect(&HudBox::new(screen(1920.0, 1080.0), 1.0), ts, 0.0).h;
        assert!(
            (row_middle(2, ts) + DETAIL_LINE / 2.0 + PAD - height).abs() < 1e-3,
            "the last row ends a padding above the bottom"
        );
    }
}

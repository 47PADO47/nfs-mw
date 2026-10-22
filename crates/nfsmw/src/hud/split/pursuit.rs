//! The pursuit strip of a split-screen race: in a player's half, while a chase is on, the heat level as pips, a
//! busted meter in red and, while the cool-down runs, the evade meter in blue. The split packages have no pursuit board
//! (`docs/specs/split-screen-hud.md` section 4), so the strip is drawn here as plain egui shapes, tessellated after the
//! packages and drawn over them.

use egui::epaint::ClippedShape;
use egui::{Color32, Pos2, Rect, Shape, Vec2};

use super::Seat;
use super::half_rect;
use crate::hud::state::HudState;
use crate::ui::present::Screen;

/// The heat pips the strip has: a heat level above this shows as all of them.
const MAX_PIPS: u8 = 5;
/// The strip's width, as a share of its half's width.
const STRIP_WIDTH: f32 = 0.30;
/// The strip's height, as a share of its half's height.
const STRIP_HEIGHT: f32 = 0.08;
/// The gap under the strip, as a share of its half's height: the strip sits near the half's bottom edge.
const STRIP_BOTTOM: f32 = 0.02;

const BACKING: Color32 = Color32::from_black_alpha(150);
const TRACK: Color32 = Color32::from_gray(50);
const PIP_LIT: Color32 = Color32::from_rgb(230, 180, 40);
const PIP_OFF: Color32 = Color32::from_gray(60);
const BUSTED: Color32 = Color32::from_rgb(220, 40, 40);
const EVADE: Color32 = Color32::from_rgb(60, 130, 255);

/// The heat pips lit for a heat level: the level rounded down, capped at five. A NaN reads as no pips.
pub fn heat_pips(heat: f32) -> u8 {
    // `as` maps NaN to 0 after the clamp, so NaN needs no branch of its own.
    heat.floor().clamp(0.0, f32::from(MAX_PIPS)) as u8
}

/// The strip in a half, `[x, y, width, height]` in points like `half`: centred across the half, near its bottom edge.
pub fn strip_rect(half: [f32; 4]) -> [f32; 4] {
    let [x, y, w, h] = half;
    let height = h * STRIP_HEIGHT;
    [x + w * (1.0 - STRIP_WIDTH) / 2.0, y + h * (1.0 - STRIP_BOTTOM) - height, w * STRIP_WIDTH, height]
}

/// The strips of the halves with a chase on, as egui shapes over the window (`screen` in points). Empty outside a
/// chase.
pub(super) fn shapes(states: &[HudState; 2], screen: Screen) -> Vec<ClippedShape> {
    let clip = Rect::from_min_size(Pos2::ZERO, Vec2::new(screen.width, screen.height));
    let mut out = Vec::new();
    for seat in Seat::BOTH {
        let state = &states[seat.index()];
        if !state.visible || state.pursuit.is_none() {
            continue;
        }
        strip(&mut out, clip, strip_rect(half_rect(seat, screen)), state);
    }
    out
}

/// One half's strip: the backing, the heat pips on the left, the busted bar and the cool-down bar on the right.
fn strip(out: &mut Vec<ClippedShape>, clip: Rect, [x, y, w, h]: [f32; 4], state: &HudState) {
    fill(out, clip, [x, y, w, h], BACKING);
    let pad = h * 0.2;
    let pip = h - 2.0 * pad;
    let gap = pip * 0.25;
    let lit = heat_pips(state.vehicle_heat);
    for i in 0..MAX_PIPS {
        let px = x + pad + f32::from(i) * (pip + gap);
        let colour = if i < lit { PIP_LIT } else { PIP_OFF };
        fill(out, clip, [px, y + pad, pip, pip], colour);
    }
    let bars_x = x + pad + f32::from(MAX_PIPS) * pip + f32::from(MAX_PIPS - 1) * gap + pad;
    let bars_w = x + w - pad - bars_x;
    let bar_h = (h - 3.0 * pad) / 2.0;
    let busted_y = y + pad;
    bar(out, clip, [bars_x, busted_y, bars_w, bar_h], share(state.busted), BUSTED);
    let Some(pursuit) = state.pursuit.as_ref().filter(|pursuit| pursuit.cooldown) else { return };
    bar(out, clip, [bars_x, busted_y + bar_h + pad, bars_w, bar_h], share(pursuit.evade), EVADE);
}

/// A bar: its track, then its fill from the left by `amount` (0 to 1) of the track.
fn bar(out: &mut Vec<ClippedShape>, clip: Rect, [x, y, w, h]: [f32; 4], amount: f32, colour: Color32) {
    fill(out, clip, [x, y, w, h], TRACK);
    fill(out, clip, [x, y, w * amount, h], colour);
}

/// A filled rectangle, unless it has no area.
fn fill(out: &mut Vec<ClippedShape>, clip: Rect, [x, y, w, h]: [f32; 4], colour: Color32) {
    if w <= 0.0 || h <= 0.0 {
        return;
    }
    let rect = Rect::from_min_size(Pos2::new(x, y), Vec2::new(w, h));
    out.push(ClippedShape { clip_rect: clip, shape: Shape::rect_filled(rect, 0.0, colour) });
}

/// A meter's share, 0 to 1; a value that is not a number reads as empty.
fn share(value: f32) -> f32 {
    if !value.is_finite() {
        return 0.0;
    }
    value.clamp(0.0, 1.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strip_sits_in_its_half() {
        for (width, height) in [(1280.0, 720.0), (800.0, 1000.0), (2560.0, 1080.0)] {
            for seat in Seat::BOTH {
                let index = seat.index();
                let screen = Screen { width, height, pixels_per_point: 1.0 };
                let half = half_rect(seat, screen);
                let [x, y, w, h] = strip_rect(half);
                let [hx, hy, hw, hh] = half;
                assert!(x >= hx && y >= hy, "half {index} at {width}x{height}");
                assert!(x + w <= hx + hw && y + h <= hy + hh, "half {index} at {width}x{height}");
                assert!(w > 0.0 && h > 0.0, "half {index} at {width}x{height}");
            }
        }
    }

    #[test]
    fn heat_pips_round_down_and_cap_at_five() {
        let cases = [(0.0, 0), (0.99, 0), (1.0, 1), (3.7, 3), (5.0, 5), (7.9, 5), (99.0, 5), (-2.0, 0), (f32::NAN, 0)];
        for (heat, pips) in cases {
            assert_eq!(heat_pips(heat), pips, "heat {heat}");
        }
    }
}

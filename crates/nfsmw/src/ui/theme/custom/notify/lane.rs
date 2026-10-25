//! The feedback lane's chip, just above the telemetry cluster at the bottom right (`docs/specs/custom-hud/layout.md`
//! section 3, `feedback.md` section 1): a small plate with an accent line, the chip's icon and its text. The bounty
//! delta is in the numeric face, in `success`; a near miss counts its repeats (`×3`).
//!
//! The chip rises into place over `slide` and fades in over `fade_in`, then fades out over `fade_out` before it goes;
//! motion reduction leaves the fades only. A repeat folded into it keeps its age, so its entry does not replay.

use blackbox_ui::{LayoutCx, Rect, justify};

use super::super::feed::ChipView;
use super::super::layout::{Anchor, HudBox};
use super::super::plate::{Plate, PlateStyle};
use super::super::text::{Label, Sizing, size};
use super::super::tokens::{Motion, fade, keys, ramp};
use super::icon::icon;
use super::stack::{BADGE, ICON, ICON_GAP, PAD};
use super::{Parts, ease};
use crate::hud::ChipKind;

/// The cluster's height at scale 1 (`docs/specs/custom-hud/telemetry.md` section 1): the lane sits this far, plus
/// [`LANE_GAP`], above the bottom-right anchor.
pub const CLUSTER_HEIGHT: f32 = 68.0;
pub const LANE_GAP: f32 = 6.0;
/// The chip's size at scale 1.
pub const CHIP_WIDTH: f32 = 132.0;
pub const CHIP_HEIGHT: f32 = 22.0;

const Z_PLATE: f32 = 0.42;
const Z_FRONT: f32 = 0.41;

/// The chip's rectangle when it is in place (canvas units).
pub fn rect(frame: &HudBox) -> Rect {
    frame.place(Anchor::BottomRight, (0.0, -(CLUSTER_HEIGHT + LANE_GAP)), (CHIP_WIDTH, CHIP_HEIGHT))
}

/// The chip's opacity: the HUD's `opacity` times its entry and exit fades.
pub fn opacity(chip: &ChipView, motion: Motion, hud_opacity: f32) -> f32 {
    hud_opacity * ramp(chip.age, motion.fade_in) * ramp(chip.remaining, motion.fade_out)
}

/// How far below its place the chip is, `age` seconds after it came up, at HUD scale `s`: 0 with motion reduction.
pub fn rise(age: f32, motion: Motion, s: f32) -> f32 {
    (1.0 - ease(ramp(age, motion.slide))) * motion.slide_distance * s
}

/// The chip's parts, laid out against `cx`; none when there is no chip or it has faded out.
pub fn parts(
    chip: Option<&ChipView>,
    frame: &HudBox,
    sizing: Sizing,
    motion: Motion,
    hud_opacity: f32,
    cx: &LayoutCx<'_>,
) -> Parts {
    let mut parts = Parts::new();
    let Some(chip) = chip else { return parts };
    let alpha = opacity(chip, motion, hud_opacity);
    if alpha <= 0.0 {
        return parts;
    }
    let s = frame.scale();
    let place = rect(frame);
    let rect = Rect { y: place.y + rise(chip.age, motion, s), ..place };
    let style = PlateStyle::SURFACE.with_accent(keys::ACCENT).scaled(s).with_opacity(alpha).with_z(Z_PLATE);
    parts.push(Plate::new(rect, style, cx));
    let icon_rect = Rect::new(rect.min_x() + (PAD + ICON / 2.0) * s, rect.y, ICON * s, ICON * s);
    if let Some(colour) = cx.skin.colour(keys::ACCENT) {
        parts.push(icon(chip.icon, icon_rect, fade(colour, alpha), Z_FRONT));
    }
    let left = rect.min_x() + (PAD + ICON + ICON_GAP) * s;
    let badge = match chip.count > 1 {
        true => BADGE * s,
        false => 0.0,
    };
    let width = (rect.max_x() - PAD * s - left - badge).max(0.0);
    let height = sizing.units(size::NOTIFICATION_LINE);
    let at = Rect::new(left + width / 2.0, rect.y, width, height);
    let label = match chip.kind {
        ChipKind::Bounty => Label::numeric(chip.text.clone(), size::NOTIFICATION_LINE).colour(keys::SUCCESS),
        _ => Label::label(chip.text.clone(), size::NOTIFICATION_LINE),
    };
    parts.push(label.justify(justify::V_CENTRE).opacity(alpha).z(Z_FRONT).laid_out(at, sizing, cx));
    if chip.count <= 1 {
        return parts;
    }
    let at = Rect::new(rect.max_x() - PAD * s - badge / 2.0, rect.y, badge, height);
    let count = crate::i18n::t!("hud-notify-count", count = chip.count);
    let label = Label::numeric(count, size::NOTIFICATION_LINE).colour(keys::TEXT_SECONDARY);
    parts.push(label.justify(justify::H_RIGHT | justify::V_CENTRE).opacity(alpha).z(Z_FRONT).laid_out(at, sizing, cx));
    parts
}

//! The notification stack: up to three plates at the left-middle of the content box, growing downwards from
//! [`STACK_TOP`] of its height, left of the road (`docs/specs/custom-hud/layout.md` section 3,
//! `notifications.md` section 6).
//!
//! - **Order.** Top down, the most urgent first, then the oldest (the view's order): a Critical notice is the top
//!   plate.
//! - **Look.** Critical is a solid `critical` plate with dark text and its icon, at least `SAFETY_OPACITY` whatever
//!   `opacity` says. High is a raised plate with a `caution` bar on its left. Medium and below are the plain plate,
//!   the icon in the category's colour. Every plate has its icon and its text, so no notice is told by colour alone.
//! - **Motion.** A notice slides in from the left over `slide` and fades in over `fade_in`; it fades out over
//!   `fade_out` before it goes. Motion reduction leaves the fades only. A repeat folded into a notice keeps its age, so
//!   its entry does not replay; the count badge (`×3`) shows the repeats.

use blackbox_ui::notify::Priority;
use blackbox_ui::{Component, LayoutCx, Panel, Rect, justify};

use super::super::feed::NoticeView;
use super::super::layout::HudBox;
use super::super::plate::{Plate, PlateStyle};
use super::super::text::{Label, Sizing, size};
use super::super::tokens::{Motion, fade, faded_skin, keys, ramp, safety_opacity};
use super::icon::icon;
use super::{Parts, ease};
use crate::hud::NoticeCategory;

/// The plates shown at most.
pub const MAX_PLATES: usize = 3;
/// Where the stack starts, as a fraction of the content box's height from its top.
pub const STACK_TOP: f32 = 0.36;
/// A plate's width at scale 1, in HUD units: the left quarter of a 16:9 box, clear of the road ahead.
pub const WIDTH: f32 = 196.0;
/// The padding inside a plate, the icon's side, the gap after the icon, the gap between the two lines, the gap
/// between plates and the High plate's bar, at scale 1.
pub const PAD: f32 = 6.0;
pub const ICON: f32 = 14.0;
pub const ICON_GAP: f32 = 6.0;
pub const LINE_GAP: f32 = 2.0;
pub const GAP: f32 = 4.0;
pub const BAR: f32 = 3.0;
/// The room the count badge takes at the right of the first line, at scale 1.
pub const BADGE: f32 = 26.0;

/// The depths: the stack draws over the HUD's other zones, and a Critical plate over everything in the layer.
const Z_PLATE: f32 = 0.40;
const Z_FRONT: f32 = 0.38;
const Z_CRITICAL_PLATE: f32 = 0.30;
const Z_CRITICAL_FRONT: f32 = 0.28;

/// The rectangle of each plate shown, top down (canvas units).
pub fn rects(notices: &[NoticeView], frame: &HudBox, sizing: Sizing) -> Vec<Rect> {
    let s = frame.scale();
    let content = frame.content();
    let left = frame.safe().min_x();
    let mut top = content.min_y() + STACK_TOP * content.h;
    let mut out = Vec::with_capacity(MAX_PLATES);
    for notice in notices.iter().take(MAX_PLATES) {
        let h = height(notice.secondary.is_some(), s, sizing);
        out.push(Rect::new(left + WIDTH * s / 2.0, top + h / 2.0, WIDTH * s, h));
        top += h + GAP * s;
    }
    out
}

/// A plate's height at HUD scale `s`: the padding round one or two lines of text.
fn height(two_lines: bool, s: f32, sizing: Sizing) -> f32 {
    let second = match two_lines {
        true => LINE_GAP * s + sizing.units(size::NOTIFICATION_LINE),
        false => 0.0,
    };
    2.0 * PAD * s + sizing.units(size::NOTIFICATION_TITLE) + second
}

/// A notice's opacity: the HUD's `opacity` (at least `SAFETY_OPACITY` for a Critical one) times its entry and exit
/// fades.
pub fn opacity(notice: &NoticeView, motion: Motion, hud_opacity: f32) -> f32 {
    let base = match notice.priority {
        Priority::Critical => safety_opacity(hud_opacity),
        _ => hud_opacity,
    };
    base * ramp(notice.age, motion.fade_in) * ramp(notice.remaining, motion.fade_out)
}

/// How far left of its place a notice `age` seconds old is, in canvas units at HUD scale `s`: the slide eases out
/// over `slide`, and is 0 with motion reduction.
pub fn slide(age: f32, motion: Motion, s: f32) -> f32 {
    -(1.0 - ease(ramp(age, motion.slide))) * motion.slide_distance * s
}

/// The keys a notice of `priority` and `category` is drawn in.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Look {
    pub plate: PlateStyle,
    /// The bar down the plate's left side, if any.
    pub bar: Option<&'static str>,
    pub text: &'static str,
    pub secondary: &'static str,
    pub icon: &'static str,
}

impl Look {
    pub fn of(priority: Priority, category: NoticeCategory) -> Self {
        match priority {
            Priority::Critical => Self {
                plate: PlateStyle { fill: keys::CRITICAL, ..PlateStyle::SURFACE },
                bar: None,
                text: keys::CRITICAL_TEXT,
                secondary: keys::CRITICAL_TEXT,
                icon: keys::CRITICAL_TEXT,
            },
            Priority::High => Self {
                plate: PlateStyle::RAISED,
                bar: Some(keys::CAUTION),
                text: keys::TEXT,
                secondary: keys::TEXT_SECONDARY,
                icon: keys::CAUTION,
            },
            _ => Self {
                plate: PlateStyle::SURFACE,
                bar: None,
                text: keys::TEXT,
                secondary: keys::TEXT_SECONDARY,
                icon: category_key(category),
            },
        }
    }
}

/// The colour a category's icon is drawn in on a plain plate.
fn category_key(category: NoticeCategory) -> &'static str {
    match category {
        NoticeCategory::Pursuit => keys::DANGER,
        NoticeCategory::Race => keys::NAV,
        NoticeCategory::Vehicle => keys::CAUTION,
        NoticeCategory::Driving => keys::ACCENT,
        NoticeCategory::Career => keys::SUCCESS,
    }
}

/// The parts of every plate shown, laid out against `cx`.
pub fn parts(
    notices: &[NoticeView],
    frame: &HudBox,
    sizing: Sizing,
    motion: Motion,
    hud_opacity: f32,
    cx: &LayoutCx<'_>,
) -> Parts {
    let mut parts = Parts::new();
    for (notice, rect) in notices.iter().zip(rects(notices, frame, sizing)) {
        let alpha = opacity(notice, motion, hud_opacity);
        if alpha <= 0.0 {
            continue;
        }
        let shifted = Rect { x: rect.x + slide(notice.age, motion, frame.scale()), ..rect };
        plate(&mut parts, notice, shifted, frame.scale(), sizing, alpha, cx);
    }
    parts
}

/// One plate: the fill, the bar, the icon, the text, the second line and the count badge.
fn plate(parts: &mut Parts, notice: &NoticeView, rect: Rect, s: f32, sizing: Sizing, alpha: f32, cx: &LayoutCx<'_>) {
    let look = Look::of(notice.priority, notice.category);
    let (z_plate, z_front) = match notice.priority {
        Priority::Critical => (Z_CRITICAL_PLATE, Z_CRITICAL_FRONT),
        _ => (Z_PLATE, Z_FRONT),
    };
    parts.push(Plate::new(rect, look.plate.scaled(s).with_opacity(alpha).with_z(z_plate), cx));
    if let Some(key) = look.bar {
        let skin = faded_skin(cx.skin, &[key], &[], alpha);
        let mut bar = Panel::new(key).with_z(z_front);
        bar.layout(&LayoutCx::new(&skin, cx.time_s), Rect::new(rect.min_x() + BAR * s / 2.0, rect.y, BAR * s, rect.h));
        parts.push(bar);
    }
    let title_h = sizing.units(size::NOTIFICATION_TITLE);
    let title_y = rect.min_y() + PAD * s + title_h / 2.0;
    let icon_rect = Rect::new(rect.min_x() + (PAD + ICON / 2.0) * s, title_y, ICON * s, ICON * s);
    if let Some(colour) = cx.skin.colour(look.icon) {
        parts.push(icon(notice.icon, icon_rect, fade(colour, alpha), z_front));
    }
    let text_left = rect.min_x() + (PAD + ICON + ICON_GAP) * s;
    let badge = match notice.count > 1 {
        true => BADGE * s,
        false => 0.0,
    };
    let text_w = (rect.max_x() - PAD * s - text_left - badge).max(0.0);
    let title = Rect::new(text_left + text_w / 2.0, title_y, text_w, title_h);
    let label = Label::label(notice.text.clone(), size::NOTIFICATION_TITLE).colour(look.text);
    parts.push(label.justify(justify::V_CENTRE).opacity(alpha).z(z_front).laid_out(title, sizing, cx));
    if notice.count > 1 {
        let at = Rect::new(rect.max_x() - PAD * s - badge / 2.0, title_y, badge, title_h);
        let count = crate::i18n::t!("hud-notify-count", count = notice.count);
        let label = Label::numeric(count, size::NOTIFICATION_LINE).colour(look.secondary);
        parts.push(
            label.justify(justify::H_RIGHT | justify::V_CENTRE).opacity(alpha).z(z_front).laid_out(at, sizing, cx),
        );
    }
    let Some(secondary) = &notice.secondary else { return };
    let line_h = sizing.units(size::NOTIFICATION_LINE);
    let line_y = rect.min_y() + PAD * s + title_h + LINE_GAP * s + line_h / 2.0;
    let line = Rect::new(text_left + (text_w + badge) / 2.0, line_y, text_w + badge, line_h);
    let label = Label::label(secondary.clone(), size::NOTIFICATION_LINE).colour(look.secondary);
    parts.push(label.justify(justify::V_CENTRE).opacity(alpha).z(z_front).laid_out(line, sizing, cx));
}

//! The end of a chase in the `Banner` slot (`docs/specs/custom-hud/pursuit.md` section 2): an escape card for
//! [`CARD_SECS`] (`ESCAPED`, the chase's length and the cops wrecked, in `success`), then the layout goes back to free
//! roam. A busted chase gets a short `BUSTED` card only where the bust flow leaves room for it: while the busted overlay
//! is up the overlay is the screen, so no card. The timing is the world's end card's (`PursuitEndCard::since_end`).
//! In career mode the escape card adds the bounty the chase earned (its pending points as last seen while it ran,
//! `Marks::chase_bounty`).

use blackbox_pursuit::EndReason;

use super::super::layout::Anchor;
use super::super::parts::{Parts, centred, edges};
use super::super::plate::PlateStyle;
use super::super::race::intro::COUNT_DOWN;
use super::super::text::{Label, size};
use super::super::tokens::{Motion, keys, ramp};
use super::super::words::{clock, grouped};
use crate::hud::PursuitEndCard;
use crate::i18n::t;

/// How long the card stays, seconds after the chase ended.
pub const CARD_SECS: f32 = 3.0;
/// The card's size at scale 1.
const CARD: (f32, f32) = (220.0, 46.0);

/// How the chase ended, as the card says it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EndKind {
    Escaped,
    Busted,
}

/// The card the end `end` calls for now, if any: within [`CARD_SECS`] of the end, and for a bust only while the busted
/// overlay is not up.
pub fn end_kind(end: &PursuitEndCard, busted_overlay: bool) -> Option<EndKind> {
    if !(0.0..CARD_SECS).contains(&end.since_end) {
        return None;
    }
    match end.reason {
        EndReason::Evaded => Some(EndKind::Escaped),
        EndReason::Busted if !busted_overlay => Some(EndKind::Busted),
        EndReason::Busted | EndReason::None => None,
    }
}

/// The card's fade at `since_end`: in over `fade_in`, out over the last `fade_out` of its time.
pub fn card_opacity(since_end: f32, motion: &Motion) -> f32 {
    let shown = ramp(since_end, motion.fade_in);
    let leaving = 1.0 - ramp(since_end - (CARD_SECS - motion.fade_out), motion.fade_out);
    shown.min(leaving)
}

/// Draws the card for `end` in the upper third, where the countdown would be.
pub fn draw(parts: &mut Parts<'_>, end: &PursuitEndCard, kind: EndKind, bounty: Option<u32>) {
    let rect = parts.place_clear(Anchor::TopCentre, (0.0, COUNT_DOWN - CARD.1 / 2.0), CARD);
    let (accent, title) = match kind {
        EndKind::Escaped => (keys::SUCCESS, t!("hud-pursuit-escaped-card")),
        EndKind::Busted => (keys::DANGER, t!("hud-pursuit-busted-card")),
    };
    parts.plate(rect, PlateStyle::RAISED.with_accent(accent));
    let middle = rect.y + 4.0 * parts.scale();
    let top = edges(rect.min_x(), rect.min_y(), rect.max_x(), middle);
    parts.text(Label::heading(title.to_uppercase(), size::NOTIFICATION_TITLE).colour(accent).justify(centred()), top);
    if kind != EndKind::Escaped {
        return;
    }
    let line = escape_line(end, bounty);
    let bottom = edges(rect.min_x(), middle, rect.max_x(), rect.max_y());
    parts.text(Label::label(line, size::NOTIFICATION_LINE).colour(keys::TEXT_SECONDARY).justify(centred()), bottom);
}

/// The escape card's second line: the chase's length, the cops wrecked and, in career mode, the bounty earned.
pub fn escape_line(end: &PursuitEndCard, bounty: Option<u32>) -> String {
    let mut parts = vec![clock(end.secs), t!("hud-pursuit-cops-wrecked", n = end.cops_destroyed)];
    parts.extend(bounty.map(|bounty| t!("hud-pursuit-bounty-earned", bounty = grouped(bounty))));
    parts.join("   ")
}

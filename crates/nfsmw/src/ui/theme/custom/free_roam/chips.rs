//! The top-left column of free roam, drawn in the `Banner` slot: the entry card of the place the car stands in, then
//! the discovery chips of the places near it.
//!
//! - **Entry card**: the place's name and, where Interact enters or leaves it, the live binding's key cap and the
//!   action's word (`ui/prompts.rs`), so a remapped key or a controller shows what the player has to press. A card
//!   that is only information (a locked race) shows the world's line as it is. It fades with the world's own card
//!   timing.
//! - **Discovery chips**: the kind (a race's type, or `Nearby`), the name (a race's track, or the kind of place) and the
//!   distance, nearest first ([`discovery`](super::discovery)). Only in free roam.

use blackbox_ui::Rect;

use super::super::feed::CustomFeed;
use super::super::layout::Anchor;
use super::super::memory::Frame;
use super::super::mode::HudMode;
use super::super::parts::{Parts, edges, left, right};
use super::super::plate::PlateStyle;
use super::super::text::{Label, size};
use super::super::tokens::{Motion, keys};
use super::super::words::{distance_near, place, race_type, track_name};
use super::discovery::Chip;
use super::{BELOW_RADIO, EXTRAS_HEIGHT, GAP};
use crate::hud::{EntryAction, EntryPrompt, NavMarkerKind};
use crate::i18n::t;
use crate::ui::prompts::Prompt;

/// A card's size at scale 1, and the inner padding.
const CARD: (f32, f32) = (210.0, 32.0);
const PAD: f32 = 6.0;
/// A key cap's size at scale 1.
const KEY: (f32, f32) = (34.0, 13.0);

/// Draws the column for the frame: the entry card in any mode it is up in, the chips in free roam.
pub fn draw(parts: &mut Parts<'_>, feed: &CustomFeed, frame: &Frame, motion: &Motion) {
    let mut top = BELOW_RADIO;
    if feed.settings.free_roam_extras {
        top += EXTRAS_HEIGHT + GAP;
    }
    let shown = frame.opacity(|mode| mode != HudMode::Hidden, motion);
    if let Some(entry) = &feed.hud.context.entry {
        parts.fade(shown * entry.opacity);
        let rect = parts.place(Anchor::TopLeft, (0.0, top), CARD);
        entry_card(parts, rect, entry, feed.interact.as_ref());
        top += CARD.1 + GAP;
    }
    let roaming = frame.opacity(|mode| mode == HudMode::FreeRoam, motion);
    for chip in &frame.chips {
        let fade = roaming * chip.opacity(frame.now, motion);
        if fade <= 0.0 {
            continue;
        }
        parts.fade(fade);
        let rect = parts.place(Anchor::TopLeft, (0.0, top), CARD);
        chip_card(parts, rect, chip, feed);
        top += CARD.1 + GAP;
    }
}

/// The entry card in `rect`.
fn entry_card(parts: &mut Parts<'_>, rect: Rect, entry: &EntryPrompt, key: Option<&Prompt>) {
    parts.plate(rect, PlateStyle::RAISED.with_accent(keys::ACCENT));
    let s = parts.scale();
    let (top, bottom) = halves(rect);
    let title = edges(rect.min_x() + PAD * s, top.min_y(), rect.max_x() - PAD * s, top.max_y());
    parts.text(Label::label(entry.title.clone(), size::NOTIFICATION_TITLE).justify(left()), title);
    let line = edges(rect.min_x() + PAD * s, bottom.min_y(), rect.max_x() - PAD * s, bottom.max_y());
    let (Some(action), Some(key)) = (entry.action, key) else {
        let label = Label::label(entry.line.clone(), size::NOTIFICATION_LINE).colour(keys::TEXT_SECONDARY);
        parts.text(label.justify(left()), line);
        return;
    };
    let cap = Rect::new(line.min_x() + KEY.0 * s / 2.0, line.y, KEY.0 * s, KEY.1 * s);
    parts.key(key, cap);
    let word = match action {
        EntryAction::GoIn => t!("hud-entry-action-go-in"),
        EntryAction::Leave => t!("hud-entry-action-leave"),
    };
    let after = edges(cap.max_x() + PAD * s, line.min_y(), line.max_x(), line.max_y());
    parts.text(Label::label(word, size::NOTIFICATION_LINE).justify(left()), after);
}

/// A discovery chip in `rect`: the kind over the name, the distance at the right.
fn chip_card(parts: &mut Parts<'_>, rect: Rect, chip: &Chip, feed: &CustomFeed) {
    parts.plate(rect, PlateStyle::SURFACE.with_accent(keys::NAV));
    let s = parts.scale();
    let (top, bottom) = halves(rect);
    let (heading, name) = chip_words(chip);
    let inner = |row: Rect| edges(rect.min_x() + PAD * s, row.min_y(), rect.max_x() - PAD * s, row.max_y());
    parts.text(Label::heading(heading.to_uppercase(), size::HEADING).justify(left()), inner(top));
    parts.text(Label::label(name, size::NOTIFICATION_LINE).justify(left()), inner(bottom));
    let distance = Label::numeric(distance_near(chip.distance_m, feed.units), size::NOTIFICATION_LINE);
    parts.text(distance.colour(keys::NAV).justify(right()), inner(bottom));
}

/// The words of a chip: a race's type over its track's name, or `Nearby` over the kind of place.
pub fn chip_words(chip: &Chip) -> (String, String) {
    match (chip.kind, &chip.name) {
        (NavMarkerKind::Race(kind), Some(event)) => (race_type(kind), track_name(event)),
        (kind, _) => (t!("hud-discovery-nearby"), place(kind)),
    }
}

/// The top and the bottom half of `rect`.
fn halves(rect: Rect) -> (Rect, Rect) {
    let middle = rect.y;
    (edges(rect.min_x(), rect.min_y(), rect.max_x(), middle), edges(rect.min_x(), middle, rect.max_x(), rect.max_y()))
}

//! The `Banner` slot of the Custom HUD (`docs/specs/custom-hud/layout.md` section 3): the transient cards, each where
//! its zone is.
//!
//! - **Top-left column** (under the radio card): the race's intro card during the intro; otherwise the entry card and
//!   the discovery chips (`free_roam/chips.rs`).
//! - **Upper third, top-centre**: the countdown (`race/intro.rs`), or the escape and busted cards (`pursuit/end_card.rs`).
//! - **Under the race strip**: the race's banner line (a checkpoint, a lap, the race-over line) and a mod's line
//!   (`bb.hud`), the lines the HUD package's message group and the world's egui card showed.
//!
//! Under Custom the world leaves its egui cards out (`hud/cards.rs`), so this is where the player reads them. Nothing
//! shows while the HUD is hidden.

use blackbox_ui::{Component, DrawList, LayoutCx, Rect};

use super::feed::{CustomFeed, CustomFeedCell};
use super::free_roam::{BELOW_RADIO, chips};
use super::layout::{Anchor, HudBox};
use super::memory::Frame;
use super::mode::HudMode;
use super::parts::{Drawn, Parts, centred};
use super::plate::PlateStyle;
use super::pursuit::end_card;
use super::race::intro::{countdown, intro_card};
use super::text::{Label, size};
use super::tokens::{Motion, keys, ramp};
use crate::hud::ModMessageTone;

/// The lines' box at scale 1, and how far down they sit: under the race strip.
const LINE_BOX: (f32, f32) = (300.0, 22.0);
const LINE_DOWN: f32 = 32.0;
/// A big mod line's size, at scale 1.
const BIG: f32 = 22.0;

/// The Custom `Banner` component.
pub struct Banner {
    feed: CustomFeedCell,
    drawn: Drawn,
}

impl Banner {
    pub fn new(feed: CustomFeedCell) -> Self {
        Self { feed, drawn: Drawn::default() }
    }
}

impl Component for Banner {
    /// The cards place themselves in the HUD's frame, not in `bounds`.
    fn layout(&mut self, cx: &LayoutCx<'_>, _bounds: Rect) {
        let feed = self.feed.get();
        let frame = self.feed.memory().frame(&feed);
        self.drawn = draw(&feed, &frame, cx);
    }

    fn build(&self, out: &mut DrawList) {
        self.drawn.build(out);
    }
}

/// The cards of the frame.
fn draw(feed: &CustomFeed, frame: &Frame, cx: &LayoutCx<'_>) -> Drawn {
    if frame.mode() == HudMode::Hidden {
        return Drawn::default();
    }
    let motion = Motion::for_settings(&feed.settings);
    let mut parts = Parts::new(cx, HudBox::new(feed.screen, feed.settings.scale), &feed.settings);
    let hud = &feed.hud;
    let count = hud.race.and_then(|race| race.count);
    // The top-left column: the intro card until the countdown starts, else the entry card and the chips.
    let intro = hud.context.race_intro.as_ref().filter(|_| frame.mode() == HudMode::RaceIntro && count.is_none());
    match intro {
        Some(intro) => {
            parts.fade(frame.opacity(|mode| mode == HudMode::RaceIntro, &motion));
            intro_card(&mut parts, intro, BELOW_RADIO, frame.clock.age(frame.now), &motion);
        }
        None => chips::draw(&mut parts, feed, frame, &motion),
    }
    // The upper third: the countdown, or the end of a chase.
    if let Some(step) = count {
        parts.fade(1.0);
        let age = frame.marks.count.map_or(0.0, |mark| frame.age(mark.since));
        countdown(&mut parts, step, age, &motion);
    }
    let end = hud.context.pursuit_end.as_ref().filter(|_| count.is_none());
    if let Some((end, kind)) = end.and_then(|end| end_card::end_kind(end, hud.busted_overlay).map(|kind| (end, kind))) {
        parts.fade(end_card::card_opacity(end.since_end, &motion));
        end_card::draw(&mut parts, end, kind);
    }
    // The lines under the strip: the race's, then a mod's.
    let mut down = LINE_DOWN;
    if let Some(message) = hud.message.as_ref().filter(|message| !message.text.is_empty()) {
        let age = frame.marks.message.map_or(0.0, |mark| frame.age(mark.since));
        parts.fade(ramp(age, motion.fade_in));
        line(&mut parts, down, Label::label(message.text.clone(), size::NOTIFICATION_TITLE), keys::ACCENT);
        down += LINE_BOX.1 + 4.0;
    }
    if let Some((text, tone)) = &hud.context.mod_message {
        parts.fade(1.0);
        let (label, accent) = match tone {
            ModMessageTone::Info => (Label::label(text.clone(), size::NOTIFICATION_TITLE), keys::ACCENT),
            ModMessageTone::Warning => {
                (Label::label(text.clone(), size::NOTIFICATION_TITLE).colour(keys::CAUTION), keys::CAUTION)
            }
            ModMessageTone::Big => (Label::label(text.clone(), BIG), keys::ACCENT),
        };
        line(&mut parts, down, label, accent);
    }
    parts.finish()
}

/// A line on a plate at `down` units under the safe area's top, centred.
fn line(parts: &mut Parts<'_>, down: f32, label: Label, accent: &'static str) {
    let rect = parts.place(Anchor::TopCentre, (0.0, down), LINE_BOX);
    parts.plate(rect, PlateStyle::SURFACE.with_accent(accent));
    parts.text(label.justify(centred()), rect);
}

//! The Custom LAN session HUD (`LanHud`): what a LAN session adds over the world, in the Custom tokens and the OS fonts,
//! placed from the content box's anchors (`layout.rs`) at `[custom_hud] scale`, `text_size` and `opacity`.
//!
//! - **Feed.** The chat and the session notices on one plate at the bottom left, above the vehicle condition chip, the
//!   newest line lowest. Each line keeps the fade the LAN front end gives it; a notice is in the secondary colour.
//! - **Countdown and vote.** The countdown in the upper third, centred; a vote on its own plate under it.
//! - **Spectator.** At the top right: the heading, who is followed, and the racers' board.
//!
//! It reads the LAN cell for what to show and the Custom feed for the `[custom_hud]` options; the window's shape comes
//! with the LAN HUD (`LanHud::aspect`).

use blackbox_ui::{Component, DrawList, LayoutCx, Rect, justify};

use super::feed::CustomFeedCell;
use super::layout::{Anchor, HudBox};
use super::plate::{Plate, PlateStyle};
use super::text::{Label, Sizing, size};
use super::tokens::keys;
use crate::i18n::t;
use crate::settings::hud::HudSettings;
use crate::ui::present::{FENG_HEIGHT, Screen};
use crate::ui::theme::{LanCell, LanHud, LanLineKind};

/// The feed's width, its offset up from the bottom-left anchor (clear of the condition chip), the room inside its
/// plate, and its lines' size and distance, at scale 1.
const FEED_WIDTH: f32 = 340.0;
const FEED_RAISE: f32 = 44.0;
const PAD: f32 = 8.0;
const LINE_SIZE: f32 = 12.0;
const LINE_PITCH: f32 = 16.0;
/// The countdown's middle below the top-centre anchor and its size; the vote's plate under it.
const COUNTDOWN_DROP: f32 = 76.0;
const COUNTDOWN_SIZE: f32 = 22.0;
const VOTE_DROP: f32 = 112.0;
const VOTE_WIDTH: f32 = 360.0;
const VOTE_HEIGHT: f32 = 26.0;
/// The spectator's plate width, at scale 1.
const SPECTATOR_WIDTH: f32 = 200.0;
/// The depth of the plates and of the text over them.
const PLATE_Z: f32 = 2.0;
const TEXT_Z: f32 = 1.0;

/// The Custom component of the `LanHud` slot.
pub struct LanHudParts {
    cell: LanCell,
    feed: CustomFeedCell,
    parts: Vec<Box<dyn Component>>,
}

impl LanHudParts {
    /// The session HUD in `cell`, at the options in `feed`.
    pub fn new(cell: LanCell, feed: CustomFeedCell) -> Self {
        Self { cell, feed, parts: Vec::new() }
    }
}

impl Component for LanHudParts {
    fn layout(&mut self, cx: &LayoutCx<'_>, _bounds: Rect) {
        self.parts = match self.cell.hud() {
            Some(hud) => Builder::new(cx, &self.feed.get().settings, hud.aspect).hud(&hud),
            None => Vec::new(),
        };
    }

    fn build(&self, out: &mut DrawList) {
        for part in &self.parts {
            part.build(out);
        }
    }
}

/// The window of a LAN HUD `aspect` wide, as the layout reads a screen.
pub(super) fn screen_of(aspect: f32) -> Screen {
    Screen { width: FENG_HEIGHT * aspect, height: FENG_HEIGHT, pixels_per_point: 1.0 }
}

struct Builder<'a, 'b> {
    cx: &'a LayoutCx<'b>,
    frame: HudBox,
    sizing: Sizing,
    opacity: f32,
    parts: Vec<Box<dyn Component>>,
}

impl<'a, 'b> Builder<'a, 'b> {
    fn new(cx: &'a LayoutCx<'b>, settings: &HudSettings, aspect: f32) -> Self {
        let frame = HudBox::new(screen_of(aspect), settings.scale);
        let sizing = Sizing::of(&frame, settings);
        Self { cx, frame, sizing, opacity: settings.opacity, parts: Vec::new() }
    }

    fn hud(mut self, hud: &LanHud) -> Vec<Box<dyn Component>> {
        self.feed(hud);
        if let Some(countdown) = &hud.countdown {
            let (x, top) = self.frame.point(Anchor::TopCentre);
            let y = top + COUNTDOWN_DROP * self.frame.scale();
            let label = Label::label(countdown.clone(), COUNTDOWN_SIZE).colour(keys::TEXT);
            self.line(label, (x, y), Centred::Yes, 1.0);
        }
        if let Some(vote) = &hud.vote {
            let offset = (0.0, VOTE_DROP - VOTE_HEIGHT / 2.0);
            let plate = self.frame.place(Anchor::TopCentre, offset, (VOTE_WIDTH, VOTE_HEIGHT));
            self.plate(plate, PlateStyle::SURFACE);
            let label = Label::label(vote.clone(), LINE_SIZE + 1.0).colour(keys::TEXT);
            self.line(label, (plate.x, plate.y), Centred::Yes, 1.0);
        }
        if let Some(spectator) = &hud.spectator {
            let mut lines = vec![Label::label(spectator.following.clone(), LINE_SIZE + 1.0).colour(keys::TEXT)];
            lines.extend(
                spectator.board.iter().map(|line| Label::numeric(line.clone(), LINE_SIZE).colour(keys::TEXT_SECONDARY)),
            );
            self.spectator(lines);
        }
        self.parts
    }

    /// The feed on its plate at the bottom left, the newest line lowest.
    fn feed(&mut self, hud: &LanHud) {
        if hud.feed.is_empty() {
            return;
        }
        let count = hud.feed.len() as f32;
        let height = 2.0 * PAD + LINE_PITCH * self.sizing.text_size * count;
        let plate = self.frame.place(Anchor::BottomLeft, (0.0, -FEED_RAISE), (FEED_WIDTH, height));
        self.plate(plate, PlateStyle::SURFACE);
        let pitch = self.sizing.units(LINE_PITCH);
        let x = plate.min_x() + PAD * self.frame.scale();
        let first = plate.min_y() + PAD * self.frame.scale() + pitch / 2.0;
        for (i, line) in hud.feed.iter().enumerate() {
            let colour = match line.kind {
                LanLineKind::Chat => keys::TEXT,
                LanLineKind::Notice => keys::TEXT_SECONDARY,
            };
            let label = Label::label(line.text.clone(), LINE_SIZE).colour(colour);
            self.line(label, (x, first + pitch * i as f32), Centred::No, line.opacity.clamp(0.0, 1.0));
        }
    }

    /// The spectator's plate at the top right: the heading, then `lines`.
    fn spectator(&mut self, lines: Vec<Label>) {
        let pitch = self.sizing.units(LINE_PITCH);
        let rows = lines.len() as f32 + 1.0;
        let height = 2.0 * PAD + LINE_PITCH * self.sizing.text_size * rows;
        let plate = self.frame.place(Anchor::TopRight, (0.0, 0.0), (SPECTATOR_WIDTH, height));
        self.plate(plate, PlateStyle::SURFACE.with_accent(keys::ACCENT));
        let x = plate.min_x() + PAD * self.frame.scale();
        let first = plate.min_y() + PAD * self.frame.scale() + pitch / 2.0;
        let heading = Label::heading(t!("lan-hud-spectating").to_uppercase(), size::HEADING);
        self.line(heading, (x, first), Centred::No, 1.0);
        for (i, label) in lines.into_iter().enumerate() {
            self.line(label, (x, first + pitch * (i + 1) as f32), Centred::No, 1.0);
        }
    }

    fn plate(&mut self, rect: Rect, style: PlateStyle) {
        let style = style.scaled(self.frame.scale()).with_opacity(self.opacity).with_z(PLATE_Z);
        self.parts.push(Box::new(Plate::new(rect, style, self.cx)));
    }

    /// A line whose left edge (or middle) is at `at`, at the HUD's opacity times `fade`, the line's own.
    fn line(&mut self, label: Label, (x, y): (f32, f32), centred: Centred, fade: f32) {
        let width = FENG_HEIGHT * 2.0;
        let height = self.sizing.units(LINE_PITCH * 2.0);
        let (rect, flags) = match centred {
            Centred::Yes => (Rect::new(x, y, width, height), justify::H_CENTRE | justify::V_CENTRE),
            Centred::No => (Rect::new(x + width / 2.0, y, width, height), justify::V_CENTRE),
        };
        let label = label.opacity(self.opacity * fade).justify(flags).z(TEXT_Z);
        self.parts.push(Box::new(label.laid_out(rect, self.sizing, self.cx)));
    }
}

/// Whether a line is centred on its point or starts at it.
#[derive(Clone, Copy)]
enum Centred {
    Yes,
    No,
}

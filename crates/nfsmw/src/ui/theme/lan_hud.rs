//! The session HUD pieces both themes draw the same way and dress with their own font and colours: the feed at the
//! bottom left, the countdown and the vote at the top, the spectator's panel at the top right. The geometry is in FEng
//! units (480 high, origin at the centre, y down); the screen's left edge follows the window's aspect.

use blackbox_ui::{Component, DrawList, LayoutCx, Panel, Rect, Text, justify};

use super::{LanCell, LanHud, LanLineKind};
use crate::i18n::t;

/// The keys of a theme's HUD dress: the font and the colours, as skin keys.
#[derive(Clone, Copy, Debug)]
pub struct HudDress {
    pub font: &'static str,
    /// What the font's size is multiplied by: the scales below are for the raster faces, the game's own fonts run larger
    /// for one scale unit.
    pub scale: f32,
    /// The big countdown and the panel's heading.
    pub accent: &'static str,
    /// The text of a chat line and of the panels.
    pub text: &'static str,
    /// The text of a notice and of the spectator's board.
    pub muted: &'static str,
    /// The plate behind a block of text.
    pub plate: &'static str,
    /// A chat line and a notice at each fade step, brightest first ([`FADE_STEPS`] of them).
    pub chat: [&'static str; FADE_STEPS],
    pub notice: [&'static str; FADE_STEPS],
}

/// How many steps a fading line goes through.
pub const FADE_STEPS: usize = 4;

/// The fade step of a line with `opacity` (0 to 1): 0 is the brightest.
pub fn fade_step(opacity: f32) -> usize {
    let lit = (opacity.clamp(0.0, 1.0) * FADE_STEPS as f32).ceil() as usize;
    FADE_STEPS - lit.clamp(1, FADE_STEPS)
}

/// The colour alpha of fade step `step`: 255 at the brightest, down to a quarter of it.
pub fn fade_alpha(step: usize) -> u8 {
    let lit = FADE_STEPS - step.min(FADE_STEPS - 1);
    (255 * lit / FADE_STEPS) as u8
}

const HALF_HEIGHT: f32 = 240.0;
const MARGIN: f32 = 16.0;
/// The feed's last line, from the screen's top, and the pitch of its lines.
const FEED_BOTTOM: f32 = 96.0;
const FEED_PITCH: f32 = 13.0;
const FEED_SCALE: f32 = 1.3;
const FEED_WIDTH: f32 = 380.0;
const COUNTDOWN_Y: f32 = -150.0;
const COUNTDOWN_SCALE: f32 = 3.0;
const VOTE_Y: f32 = -112.0;
const VOTE_SCALE: f32 = 1.5;
const BANNER_WIDTH: f32 = 560.0;
const SPECTATOR_Y: f32 = -150.0;
const SPECTATOR_PITCH: f32 = 14.0;
const SPECTATOR_SCALE: f32 = 1.35;
const SPECTATOR_WIDTH: f32 = 170.0;

/// The HUD component of one theme: it reads the cell and draws the pieces with `dress`.
pub struct LanHudParts {
    cell: LanCell,
    dress: HudDress,
    parts: Vec<Box<dyn Component>>,
}

impl LanHudParts {
    pub fn new(cell: LanCell, dress: HudDress) -> Self {
        Self { cell, dress, parts: Vec::new() }
    }
}

impl Component for LanHudParts {
    fn layout(&mut self, cx: &LayoutCx<'_>, _bounds: Rect) {
        self.parts = match self.cell.hud() {
            Some(hud) => Builder { cx, dress: self.dress, parts: Vec::new() }.hud(&hud),
            None => Vec::new(),
        };
    }

    fn build(&self, out: &mut DrawList) {
        for part in &self.parts {
            part.build(out);
        }
    }
}

struct Builder<'a, 'b> {
    cx: &'a LayoutCx<'b>,
    dress: HudDress,
    parts: Vec<Box<dyn Component>>,
}

impl Builder<'_, '_> {
    fn hud(mut self, hud: &LanHud) -> Vec<Box<dyn Component>> {
        let left = -hud.aspect * HALF_HEIGHT;
        let right = hud.aspect * HALF_HEIGHT;
        self.feed(hud, left);
        if let Some(countdown) = &hud.countdown {
            self.line(countdown, self.dress.accent, COUNTDOWN_SCALE, (0.0, COUNTDOWN_Y), Align::Centre, BANNER_WIDTH);
        }
        if let Some(vote) = &hud.vote {
            self.plated(vote, self.dress.text, VOTE_SCALE, (0.0, VOTE_Y), Align::Centre, BANNER_WIDTH);
        }
        if let Some(spectator) = &hud.spectator {
            let x = right - MARGIN;
            let mut lines = vec![(spectator.following.as_str(), self.dress.text)];
            lines.extend(spectator.board.iter().map(|line| (line.as_str(), self.dress.muted)));
            self.line(
                &t!("lan-hud-spectating"),
                self.dress.accent,
                SPECTATOR_SCALE,
                (x, SPECTATOR_Y),
                Align::Right,
                SPECTATOR_WIDTH,
            );
            for (i, (line, colour)) in lines.into_iter().enumerate() {
                let y = SPECTATOR_Y + SPECTATOR_PITCH * (i + 1) as f32;
                self.line(line, colour, SPECTATOR_SCALE, (x, y), Align::Right, SPECTATOR_WIDTH);
            }
        }
        self.parts
    }

    /// The feed: the newest line lowest, each at its fade step, on one plate.
    fn feed(&mut self, hud: &LanHud, left: f32) {
        if hud.feed.is_empty() {
            return;
        }
        let count = hud.feed.len();
        let top = FEED_BOTTOM - FEED_PITCH * (count - 1) as f32;
        let plate = Rect::new(
            left + MARGIN + FEED_WIDTH / 2.0 - 4.0,
            (top + FEED_BOTTOM) / 2.0,
            FEED_WIDTH + 8.0,
            FEED_PITCH * count as f32 + 4.0,
        );
        self.add(Panel::new(self.dress.plate), plate);
        for (i, line) in hud.feed.iter().enumerate() {
            let step = fade_step(line.opacity);
            let colour = match line.kind {
                LanLineKind::Chat => self.dress.chat[step],
                LanLineKind::Notice => self.dress.notice[step],
            };
            let y = top + FEED_PITCH * i as f32;
            self.line(&line.text, colour, FEED_SCALE, (left + MARGIN, y), Align::Left, FEED_WIDTH);
        }
    }

    fn add(&mut self, mut part: impl Component + 'static, bounds: Rect) {
        part.layout(self.cx, bounds);
        self.parts.push(Box::new(part));
    }

    /// A line at `at` (its left, centre or right edge, and its middle height).
    fn line(&mut self, text: &str, colour: &str, scale: f32, at: (f32, f32), align: Align, width: f32) {
        let (x, flags) = match align {
            Align::Left => (at.0 + width / 2.0, justify::V_CENTRE),
            Align::Centre => (at.0, justify::H_CENTRE | justify::V_CENTRE),
            Align::Right => (at.0 - width / 2.0, justify::H_RIGHT | justify::V_CENTRE),
        };
        let scale = scale * self.dress.scale;
        let text = Text::new(self.dress.font, text).with_colour_key(colour).with_scale(scale).with_justification(flags);
        self.add(text, Rect::new(x, at.1, width, 20.0));
    }

    /// A line on a plate of its own, as wide as the room it may take.
    fn plated(&mut self, text: &str, colour: &str, scale: f32, at: (f32, f32), align: Align, width: f32) {
        let plate = Rect::new(at.0, at.1, width, 20.0);
        self.add(Panel::new(self.dress.plate), plate);
        self.line(text, colour, scale, at, align, width);
    }
}

#[derive(Clone, Copy)]
enum Align {
    Left,
    Centre,
    Right,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_line_fades_in_steps_from_bright_to_dim() {
        assert_eq!(fade_step(1.0), 0);
        assert_eq!(fade_step(0.8), 0);
        assert_eq!(fade_step(0.6), 1);
        assert_eq!(fade_step(0.3), 2);
        assert_eq!(fade_step(0.1), 3);
        assert_eq!(fade_step(0.0), 3);
        assert_eq!([fade_alpha(0), fade_alpha(1), fade_alpha(2), fade_alpha(3)], [255, 191, 127, 63]);
    }
}

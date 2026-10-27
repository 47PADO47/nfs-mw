//! The pieces the contextual layouts draw with: plates, labels, solid fills, segmented and continuous bars, small line
//! icons and key caps, each laid out at the HUD scale and faded by `[custom_hud] opacity` times the element's own fade.
//! A component collects them in [`Parts`] while it lays out, and draws the [`Drawn`] list in the order they were added,
//! so a plate added before its text draws under it.

use blackbox_ui::{Component, DrawList, Image, LayoutCx, LineJoin, Panel, Polyline, Rect};
use glam::Vec2;

use super::layout::{Anchor, HudBox};
use super::plate::{Plate, PlateStyle};
use super::text::{Label, Sizing, size};
use super::tokens::{fade, faded_skin, keys};
use crate::settings::hud::HudSettings;
use crate::ui::input_icons::{ATLAS, Glyph};
use crate::ui::prompts::Prompt;

/// The gap between two segments of a segmented bar, at scale 1.
const SEGMENT_GAP: f32 = 1.5;

/// The laid-out parts of a component for one frame.
#[derive(Default)]
pub struct Drawn(Vec<Box<dyn Component>>);

impl Drawn {
    /// Emits every part, in the order it was added.
    pub fn build(&self, out: &mut DrawList) {
        for part in &self.0 {
            part.build(out);
        }
    }
}

/// The parts of one component as it lays out: the frame it is placed in, the text sizing and the opacity it draws at.
pub struct Parts<'a> {
    cx: LayoutCx<'a>,
    frame: HudBox,
    sizing: Sizing,
    /// `[custom_hud] opacity`.
    base: f32,
    /// What the next parts draw at: the base times the element's fade.
    opacity: f32,
    drawn: Vec<Box<dyn Component>>,
}

impl<'a> Parts<'a> {
    /// Parts laid out against `cx`'s skin in `frame`, sized and faded by `settings`.
    pub fn new(cx: &LayoutCx<'a>, frame: HudBox, settings: &HudSettings) -> Self {
        let sizing = Sizing::of(&frame, settings);
        Self { cx: *cx, frame, sizing, base: settings.opacity, opacity: settings.opacity, drawn: Vec::new() }
    }

    /// Draws the parts that follow at `fade` (0 to 1) times the HUD's opacity.
    pub fn fade(&mut self, fade: f32) {
        self.opacity = self.base * fade.clamp(0.0, 1.0);
    }

    /// Draws the parts that follow at least at `floor`, whatever the HUD's opacity (a safety warning).
    pub fn at_least(&mut self, floor: f32) {
        self.opacity = self.opacity.max(floor);
    }

    pub fn scale(&self) -> f32 {
        self.frame.scale()
    }

    /// The rectangle of an element `size` units across at scale 1, at `anchor`, moved by `offset` (`HudBox::place`).
    pub fn place(&self, anchor: Anchor, offset: (f32, f32), size: (f32, f32)) -> Rect {
        self.frame.place(anchor, offset, size)
    }

    /// As [`Parts::place`], but a top-centre card is pushed down only as far as it needs to clear the map
    /// (`HudBox::place_clear`).
    pub fn place_clear(&self, anchor: Anchor, offset: (f32, f32), size: (f32, f32)) -> Rect {
        self.frame.place_clear(anchor, offset, size)
    }

    /// A plate over `rect` in `style` (its own opacity times the parts' opacity).
    pub fn plate(&mut self, rect: Rect, style: PlateStyle) {
        let style = style.scaled(self.scale()).with_opacity(style.opacity * self.opacity);
        self.drawn.push(Box::new(Plate::new(rect, style, &self.cx)));
    }

    /// `label` laid out in `rect`.
    pub fn text(&mut self, label: Label, rect: Rect) {
        let text = label.opacity(self.opacity).laid_out(rect, self.sizing, &self.cx);
        self.drawn.push(Box::new(text));
    }

    /// `rect` filled with the skin colour `key`.
    pub fn fill(&mut self, key: &'static str, rect: Rect) {
        let skin = faded_skin(self.cx.skin, &[key], &[], self.opacity);
        let mut panel = Panel::new(key);
        panel.layout(&LayoutCx::new(&skin, self.cx.time_s), rect);
        self.drawn.push(Box::new(panel));
    }

    /// A bar of `count` segments across `rect`, the first `lit` in `on` and the rest in the rule colour.
    pub fn segments(&mut self, rect: Rect, count: usize, lit: usize, on: &'static str) {
        if count == 0 {
            return;
        }
        let gap = SEGMENT_GAP * self.scale();
        let width = ((rect.w - gap * (count - 1) as f32) / count as f32).max(0.0);
        for i in 0..count {
            let left = rect.min_x() + i as f32 * (width + gap);
            let key = match i < lit {
                true => on,
                false => keys::RULE,
            };
            self.fill(key, edges(left, rect.min_y(), left + width, rect.max_y()));
        }
    }

    /// A track across `rect` in the rule colour, filled from the left to `fraction` (0 to 1) in `on`.
    pub fn bar(&mut self, rect: Rect, fraction: f32, on: &'static str) {
        self.fill(keys::RULE, rect);
        let fraction = match fraction.is_nan() {
            true => 0.0,
            false => fraction.clamp(0.0, 1.0),
        };
        if fraction <= 0.0 {
            return;
        }
        self.fill(on, edges(rect.min_x(), rect.min_y(), rect.min_x() + rect.w * fraction, rect.max_y()));
    }

    /// A line through `points` (canvas units), `width` units wide at scale 1, in the skin colour `key`. Nothing when the
    /// skin lacks the colour.
    pub fn stroke(&mut self, points: Vec<Vec2>, width: f32, key: &'static str) {
        let Some(colour) = self.cx.skin.colour(key) else { return };
        let line = Polyline::new(points, width * self.scale(), fade(colour, self.opacity))
            .with_join(LineJoin::Mitre { limit: 2.0 });
        self.drawn.push(Box::new(line));
    }

    /// The key cap of an action's live binding (`ui/prompts.rs`) in `rect`: the controller's glyph, or the key's name
    /// on a small raised plate.
    pub fn key(&mut self, prompt: &Prompt, rect: Rect) {
        match prompt {
            Prompt::Icon(glyph) => self.glyph(*glyph, rect),
            Prompt::Key(name) => {
                self.plate(rect, PlateStyle::RAISED.with_accent(keys::ACCENT));
                let label = Label::label(name.clone(), size::NOTIFICATION_LINE).justify(centred());
                self.text(label, rect);
            }
        }
    }

    /// A controller glyph from the input prompts' atlas, square in `rect`.
    fn glyph(&mut self, glyph: Glyph, rect: Rect) {
        let side = rect.w.min(rect.h);
        let mut image = Image::raw(ATLAS).with_uv(glyph.uv()).with_tint(fade([255, 255, 255, 255], self.opacity));
        image.layout(&self.cx, Rect::new(rect.x, rect.y, side, side));
        self.drawn.push(Box::new(image));
    }

    /// The parts, ready to build. Empty when the parts draw at no opacity.
    pub fn finish(self) -> Drawn {
        if self.base <= 0.0 {
            return Drawn::default();
        }
        Drawn(self.drawn)
    }
}

/// Text centred both ways in its rectangle.
pub fn centred() -> u32 {
    blackbox_ui::justify::H_CENTRE | blackbox_ui::justify::V_CENTRE
}

/// Text at the left, centred vertically.
pub fn left() -> u32 {
    blackbox_ui::justify::V_CENTRE
}

/// Text at the right, centred vertically.
pub fn right() -> u32 {
    blackbox_ui::justify::H_RIGHT | blackbox_ui::justify::V_CENTRE
}

/// The rectangle with the given edges (a `Rect` is centred).
pub fn edges(x0: f32, y0: f32, x1: f32, y1: f32) -> Rect {
    Rect::new((x0 + x1) / 2.0, (y0 + y1) / 2.0, (x1 - x0).max(0.0), (y1 - y0).max(0.0))
}

/// Lays cells out left to right along a row at the HUD scale: each cell is as tall as the row and as wide as asked.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Row {
    x: f32,
    top: f32,
    height: f32,
    scale: f32,
}

impl Row {
    /// A row from the left edge `left`, between `top` and `top + height` (canvas units), with widths at `scale`.
    pub fn new(left: f32, top: f32, height: f32, scale: f32) -> Self {
        Self { x: left, top, height, scale }
    }

    /// The next cell, `width` units wide at scale 1.
    pub fn cell(&mut self, width: f32) -> Rect {
        let w = width * self.scale;
        let rect = edges(self.x, self.top, self.x + w, self.top + self.height);
        self.x += w;
        rect
    }

    /// Leaves `width` units at scale 1 empty.
    pub fn skip(&mut self, width: f32) {
        self.x += width * self.scale;
    }
}

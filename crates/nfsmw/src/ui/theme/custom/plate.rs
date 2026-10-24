//! The plate every Custom component puts behind its text: a translucent charcoal `Rounded` rectangle in a token colour,
//! with an optional one-unit accent line along its top, and a one-unit outline when high contrast is on
//! (`docs/specs/custom-hud/visual-identity.md` sections 1 to 3). The plate fades with its content: the opacity it is
//! built at multiplies every part's alpha, so a plate never stays behind faded text.
//!
//! Like `Rounded`, a plate is laid out when it is made ([`Plate::new`]), against the skin of the `LayoutCx` it is given;
//! its `layout` does nothing.

use blackbox_ui::{Component, DEFAULT_Z, DrawList, LayoutCx, Panel, Rect, Rounded};

use super::tokens::{faded_skin, keys};

/// A plate's corner radius at scale 1, in HUD units. Small: the plates are dark industrial cards, not pills.
pub const PLATE_RADIUS: f32 = 3.0;
/// The width of a rule (the accent line, the outline) at scale 1, in HUD units.
pub const RULE_WIDTH: f32 = 1.0;

/// How a plate is drawn: its fill and accent keys, its corner radius at scale 1, the scale, its opacity and its depth.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PlateStyle {
    /// The fill's skin key ([`keys::SURFACE`], or [`keys::SURFACE_RAISED`] for a selected row).
    pub fill: &'static str,
    /// The top line's skin key, or `None` for no line.
    pub accent: Option<&'static str>,
    /// The corner radius at scale 1.
    pub radius: f32,
    /// The HUD scale the radius and the rules are multiplied by (`HudBox::scale`).
    pub scale: f32,
    /// The alpha every part is multiplied by: `[custom_hud] opacity` times the element's own fade.
    pub opacity: f32,
    /// The depth of every part. A lower `z` draws on top.
    pub z: f32,
}

impl PlateStyle {
    /// The charcoal plate behind HUD text.
    pub const SURFACE: PlateStyle =
        PlateStyle { fill: keys::SURFACE, accent: None, radius: PLATE_RADIUS, scale: 1.0, opacity: 1.0, z: DEFAULT_Z };
    /// The raised plate: a selected menu row, an expanded chip.
    pub const RAISED: PlateStyle = PlateStyle { fill: keys::SURFACE_RAISED, ..PlateStyle::SURFACE };

    /// This style with a top line in the skin colour `key` (usually [`keys::ACCENT`]).
    pub fn with_accent(self, key: &'static str) -> Self {
        Self { accent: Some(key), ..self }
    }

    /// This style at the HUD scale `scale`.
    pub fn scaled(self, scale: f32) -> Self {
        Self { scale, ..self }
    }

    /// This style at `opacity`.
    pub fn with_opacity(self, opacity: f32) -> Self {
        Self { opacity, ..self }
    }

    /// This style with corners of `radius` at scale 1.
    pub fn with_radius(self, radius: f32) -> Self {
        Self { radius, ..self }
    }

    /// This style at depth `z`.
    pub fn with_z(self, z: f32) -> Self {
        Self { z, ..self }
    }
}

impl Default for PlateStyle {
    fn default() -> Self {
        Self::SURFACE
    }
}

/// A laid-out plate: the fill, then the outline (high contrast only), then the accent line, in draw order.
pub struct Plate {
    fill: Rounded,
    outline: Vec<Panel>,
    accent: Option<Panel>,
}

impl Plate {
    /// The plate over `rect` (canvas units) in `style`, its colours read from `cx`'s skin and faded by the style's opacity.
    pub fn new(rect: Rect, style: PlateStyle, cx: &LayoutCx<'_>) -> Self {
        let mut colours = vec![style.fill, keys::OUTLINE];
        colours.extend(style.accent);
        let skin = faded_skin(cx.skin, &colours, &[], style.opacity);
        let local = LayoutCx::new(&skin, cx.time_s);
        let radius = (style.radius * style.scale).min(rect.w / 2.0).min(rect.h / 2.0).max(0.0);
        let rule = RULE_WIDTH * style.scale;
        let fill = Rounded::at_depth(style.fill, rect, radius, style.z, &local);
        let outline = match skin.colour(keys::OUTLINE) {
            Some(_) => outline(rect, radius, rule, style.z, &local),
            None => Vec::new(),
        };
        let accent = style.accent.map(|key| {
            let line = edges(rect.min_x() + radius, rect.min_y(), rect.max_x() - radius, rect.min_y() + rule);
            panel(key, line, style.z, &local)
        });
        Self { fill, outline, accent }
    }
}

impl Component for Plate {
    /// The plate is laid out in [`Plate::new`], so there is nothing to place here.
    fn layout(&mut self, _cx: &LayoutCx<'_>, _bounds: Rect) {}

    fn build(&self, out: &mut DrawList) {
        self.fill.build(out);
        for part in &self.outline {
            part.build(out);
        }
        if let Some(accent) = &self.accent {
            accent.build(out);
        }
    }
}

/// The four rules of the outline, inside `rect`, each stopping where the corner's curve starts.
fn outline(rect: Rect, radius: f32, rule: f32, z: f32, cx: &LayoutCx<'_>) -> Vec<Panel> {
    let (left, top, right, bottom) = (rect.min_x(), rect.min_y(), rect.max_x(), rect.max_y());
    [
        edges(left + radius, top, right - radius, top + rule),
        edges(left + radius, bottom - rule, right - radius, bottom),
        edges(left, top + radius, left + rule, bottom - radius),
        edges(right - rule, top + radius, right, bottom - radius),
    ]
    .into_iter()
    .filter(|part| part.w > 0.0 && part.h > 0.0)
    .map(|part| panel(keys::OUTLINE, part, z, cx))
    .collect()
}

/// A panel in the skin colour `key` over `rect`, laid out against `cx`.
fn panel(key: &'static str, rect: Rect, z: f32, cx: &LayoutCx<'_>) -> Panel {
    let mut panel = Panel::new(key).with_z(z);
    panel.layout(cx, rect);
    panel
}

/// The rectangle with the given edges, as a `Rect` (which is centred).
fn edges(x0: f32, y0: f32, x1: f32, y1: f32) -> Rect {
    Rect::new((x0 + x1) / 2.0, (y0 + y1) / 2.0, (x1 - x0).max(0.0), (y1 - y0).max(0.0))
}

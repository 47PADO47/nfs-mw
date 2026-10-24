//! The Original LAN screens. The original has no LAN screens, so no FEng package to run: these are drawn with the UI
//! toolkit in the game's own look. The fonts are the game's (`FONT_MW_TITLE` for the title, `FONT_MW_BODY` for the rest),
//! the logo is `MW_Logo.tga`, the text is white and the selection is the gold of the icon menu's corner brackets, with the
//! key caps and labels of the original footer. The rows stand where the Remastered list's rows stand
//! ([`remastered::menu::layout`](crate::ui::theme::remastered::menu::layout)), so one hit test serves both themes.

use blackbox_feng::{fe_hash_upper, resource_handle};
use blackbox_ui::{
    Choice, ChoiceStyle, Component, DrawList, Image, LayoutCx, Outline, Panel, Rect, Skin, Text, justify,
};

use crate::ui::input_icons::Glyph;
use crate::ui::prompts::Prompt;
use crate::ui::theme::lan_hud::{FADE_STEPS, HudDress, LanHudParts, fade_alpha};
use crate::ui::theme::remastered::menu::layout as place;
use crate::ui::theme::{LanCell, LanPage, MenuEntry, MenuHint, MenuValue};

/// The skin keys of the Original LAN screens.
pub(super) mod keys {
    pub const FONT_TITLE: &str = "original.lan.font.title";
    pub const FONT_BODY: &str = "original.lan.font.body";
    pub const TEXT: &str = "original.lan.text";
    pub const SELECTED: &str = "original.lan.selected";
    pub const DISABLED: &str = "original.lan.disabled";
    pub const MUTED: &str = "original.lan.muted";
    pub const WARN: &str = "original.lan.warn";
    pub const FILL: &str = "original.lan.fill";
    pub const PLATE: &str = "original.lan.plate";
    pub const BAND: &str = "original.lan.band";
    pub const RULE: &str = "original.lan.rule";
    /// The footer's key caps and the arrow of a value (the Remastered skin's logo, glyph atlas and arrow are the game's own
    /// files and serve here too).
    pub const LOGO: &str = "original.lan.logo";
    pub const ATLAS: &str = "original.lan.atlas";
    pub const ARROW: &str = "original.lan.arrow";
    pub fn chat(step: usize) -> String {
        format!("original.lan.chat.{step}")
    }
    pub fn notice(step: usize) -> String {
        format!("original.lan.notice.{step}")
    }
}

/// The keys the tests read the skin by.
#[cfg(test)]
pub(in crate::ui::theme) const FONT_TITLE_KEY: &str = keys::FONT_TITLE;
#[cfg(test)]
pub(in crate::ui::theme) const FONT_BODY_KEY: &str = keys::FONT_BODY;
#[cfg(test)]
pub(in crate::ui::theme) fn chat_key(step: usize) -> String {
    keys::chat(step)
}
#[cfg(test)]
pub(in crate::ui::theme) fn notice_key(step: usize) -> String {
    keys::notice(step)
}

const WHITE: [u8; 4] = [240, 240, 240, 255];
const GOLD: [u8; 4] = [236, 168, 40, 255];
const GREY: [u8; 4] = [168, 168, 168, 255];

/// The Original LAN screens' entries of the skin: the game's fonts, the white and gold, the translucent plates.
pub fn skin() -> Skin {
    let mut skin = Skin::new()
        .with_font(keys::FONT_TITLE, fe_hash_upper("FONT_MW_TITLE"))
        .with_font(keys::FONT_BODY, fe_hash_upper("FONT_MW_BODY"))
        .with_colour(keys::TEXT, WHITE)
        .with_colour(keys::SELECTED, GOLD)
        .with_colour(keys::DISABLED, [WHITE[0], WHITE[1], WHITE[2], 95])
        .with_colour(keys::MUTED, GREY)
        .with_colour(keys::WARN, [255, 120, 60, 255])
        .with_colour(keys::FILL, [GOLD[0], GOLD[1], GOLD[2], 46])
        .with_colour(keys::PLATE, [0, 0, 0, 120])
        .with_colour(keys::BAND, [0, 0, 0, 150])
        .with_colour(keys::RULE, [WHITE[0], WHITE[1], WHITE[2], 110])
        .with_texture(keys::LOGO, resource_handle("MW_Logo.tga"))
        .with_texture(keys::ATLAS, crate::ui::input_icons::ATLAS)
        .with_texture(keys::ARROW, resource_handle("ARROW_SKINNIER.tga"));
    for step in 0..FADE_STEPS {
        let alpha = fade_alpha(step);
        skin = skin
            .with_colour(keys::chat(step), [WHITE[0], WHITE[1], WHITE[2], alpha])
            .with_colour(keys::notice(step), [GREY[0], GREY[1], GREY[2], alpha]);
    }
    skin
}

/// The HUD dress of the Original LAN: the game's body font, white chat and grey notices on a dark plate. The keys of the
/// fade steps are leaked once: a dress holds `&'static str` and there are [`FADE_STEPS`] of each, for the program's life.
pub fn hud_dress() -> HudDress {
    use std::sync::OnceLock;
    static NAMES: OnceLock<([&'static str; FADE_STEPS], [&'static str; FADE_STEPS])> = OnceLock::new();
    let (chat, notice) = NAMES.get_or_init(|| {
        let leak = |name: String| -> &'static str { Box::leak(name.into_boxed_str()) };
        (std::array::from_fn(|i| leak(keys::chat(i))), std::array::from_fn(|i| leak(keys::notice(i))))
    });
    HudDress {
        font: keys::FONT_BODY,
        scale: 0.52,
        accent: keys::SELECTED,
        text: keys::TEXT,
        muted: keys::MUTED,
        plate: keys::PLATE,
        chat: *chat,
        notice: *notice,
    }
}

/// The Original component of the `LanMenu` slot.
pub struct LanMenu {
    cell: LanCell,
    parts: Vec<Box<dyn Component>>,
}

impl LanMenu {
    pub fn new(cell: LanCell) -> Self {
        Self { cell, parts: Vec::new() }
    }
}

/// The Original component of the `LanHud` slot.
pub fn hud(cell: LanCell) -> LanHudParts {
    LanHudParts::new(cell, hud_dress())
}

impl Component for LanMenu {
    fn layout(&mut self, cx: &LayoutCx<'_>, _bounds: Rect) {
        self.parts = match self.cell.page() {
            Some(page) => Page { cx, parts: Vec::new() }.of(&page),
            None => Vec::new(),
        };
    }

    fn build(&self, out: &mut DrawList) {
        for part in &self.parts {
            part.build(out);
        }
    }
}

// The game's fonts are about twice the size of the raster faces for one scale unit (the Remastered scales are for those).
const TITLE_SCALE: f32 = 1.0;
const BODY_SCALE: f32 = 1.1;
const VALUE_SCALE: f32 = 1.0;
const SMALL_SCALE: f32 = 0.72;
const LABEL_SCALE: f32 = 0.8;
const KEY_SCALE: f32 = 0.64;
const LOGO_WIDTH: f32 = 300.0;
/// The title stands higher than the Remastered one (the game's blackletter is taller), with the subtitle under it.
const TITLE_Y: f32 = place::TITLE_Y - 20.0;
const SUBTITLE_Y: f32 = place::TITLE_Y + 8.0;
const ERROR_Y: f32 = 176.0;
const SIDE_LEFT: f32 = 540.0;
const SIDE_MIN_WIDTH: f32 = 140.0;
const SIDE_LINES: usize = 12;
const SIDE_LINE_Y: f32 = 17.0;
const BAND_HEIGHT: f32 = 44.0;

struct Page<'a, 'b> {
    cx: &'a LayoutCx<'b>,
    parts: Vec<Box<dyn Component>>,
}

impl Page<'_, '_> {
    fn of(mut self, page: &LanPage) -> Vec<Box<dyn Component>> {
        let list = &page.list;
        let left = place::left_edge(list.aspect);
        let right = place::right_edge(list.aspect);
        self.logo(right);
        self.text(keys::FONT_TITLE, &list.title, keys::TEXT, TITLE_SCALE, (left + place::TEXT_LEFT, TITLE_Y), false);
        if !page.subtitle.is_empty() {
            let at = (left + place::TEXT_LEFT, SUBTITLE_Y);
            self.text(keys::FONT_BODY, &page.subtitle, keys::MUTED, SMALL_SCALE, at, false);
        }
        let wide = list.has_values();
        for (i, entry) in list.entries.iter().skip(list.first).take(place::shown(list.entries.len())).enumerate() {
            self.entry(entry, list.first + i == list.selected, place::row_y(i), (left, wide));
        }
        if let Some(error) = &page.error {
            self.text(
                keys::FONT_BODY,
                error,
                keys::WARN,
                SMALL_SCALE * 1.25,
                (left + place::TEXT_LEFT, ERROR_Y),
                false,
            );
        }
        self.side_column(page, left, right);
        self.footer(&list.hints, left, right);
        self.parts
    }

    fn add(&mut self, mut part: impl Component + 'static, bounds: Rect) {
        part.layout(self.cx, bounds);
        self.parts.push(Box::new(part));
    }

    fn text(&mut self, font: &str, text: &str, colour: &str, scale: f32, at: (f32, f32), right: bool) {
        let width = 420.0;
        let (x, flags) = match right {
            true => (at.0 - width / 2.0, justify::H_RIGHT | justify::V_CENTRE),
            false => (at.0 + width / 2.0, justify::V_CENTRE),
        };
        let text = Text::new(font, text).with_colour_key(colour).with_scale(scale).with_justification(flags);
        self.add(text, Rect::new(x, at.1, width, 20.0));
    }

    /// The logo at the top right, as the original's main menu has it.
    fn logo(&mut self, right: f32) {
        let height = LOGO_WIDTH / place::LOGO_ASPECT;
        self.add(Image::new(keys::LOGO), Rect::new(right - 30.0 - LOGO_WIDTH / 2.0, -196.0, LOGO_WIDTH, height));
    }

    fn entry(&mut self, entry: &MenuEntry, selected: bool, y: f32, (left, wide): (f32, bool)) {
        let colour = match (entry.enabled, selected) {
            (false, _) => keys::DISABLED,
            (true, true) => keys::SELECTED,
            (true, false) => keys::TEXT,
        };
        if selected {
            let (rect, _lean) = place::bar(y, left, wide);
            self.add(Panel::new(keys::FILL), rect);
            self.add(Outline::new(keys::SELECTED, rect, 2.0, 1.2, self.cx), rect);
        }
        self.text(keys::FONT_BODY, &entry.label, colour, BODY_SCALE, (left + place::TEXT_LEFT, y), false);
        let MenuValue::Choice { text, arrows } = &entry.value else { return };
        let column = place::value_column(y, left);
        let style = ChoiceStyle {
            font: keys::FONT_BODY,
            text: colour,
            scale: VALUE_SCALE,
            arrow: keys::ARROW,
            arrow_colour: keys::SELECTED,
        };
        let live = selected && entry.enabled && *arrows;
        self.add(Choice::new(style, column, text, live, None, self.cx), column);
    }

    fn side_column(&mut self, page: &LanPage, left: f32, right: f32) {
        if page.side_title.is_empty() {
            return;
        }
        let x = left + SIDE_LEFT;
        if right - x - 12.0 < SIDE_MIN_WIDTH {
            return;
        }
        self.text(keys::FONT_BODY, &page.side_title, keys::SELECTED, SMALL_SCALE * 1.25, (x, TITLE_Y), false);
        let shown = page.side.len().min(SIDE_LINES);
        let first = place::row_y(0) - 8.0;
        let plate = Rect::new(
            x + (right - x - 12.0) / 2.0,
            first + SIDE_LINE_Y * (SIDE_LINES as f32 - 1.0) / 2.0,
            right - x - 12.0,
            SIDE_LINE_Y * SIDE_LINES as f32 + 8.0,
        );
        self.add(Panel::new(keys::PLATE), plate);
        for (i, line) in page.side[page.side.len() - shown..].iter().enumerate() {
            self.text(keys::FONT_BODY, line, keys::TEXT, SMALL_SCALE, (x + 4.0, first + SIDE_LINE_Y * i as f32), false);
        }
    }

    /// The footer: a dark band across the foot of the screen with the key caps and their labels on it.
    fn footer(&mut self, hints: &[MenuHint], left: f32, right: f32) {
        let spots = place::hint_spots(hints, left);
        let y = (place::RULE_TOP_Y + place::RULE_BOTTOM_Y) / 2.0;
        let band = Rect::new((left + right) / 2.0, y, right - left, BAND_HEIGHT);
        self.add(Panel::new(keys::BAND), band);
        for (hint, spot) in hints.iter().zip(&spots) {
            for key in &spot.keys {
                match &key.prompt {
                    Prompt::Icon(glyph) => self.glyph(*glyph, key.rect),
                    Prompt::Key(text) => {
                        self.add(
                            Outline::new(keys::TEXT, key.rect, place::KEY_RADIUS, place::KEY_EDGE, self.cx),
                            key.rect,
                        );
                        let cap = Text::new(keys::FONT_BODY, text.to_uppercase())
                            .with_colour_key(keys::TEXT)
                            .with_scale(KEY_SCALE)
                            .with_justification(justify::H_CENTRE | justify::V_CENTRE);
                        self.add(cap, Rect::new(key.rect.x, key.rect.y, key.rect.w, 20.0));
                    }
                }
            }
            self.text(keys::FONT_BODY, &hint.label, keys::TEXT, LABEL_SCALE, (spot.label_x, spot.area.y), false);
        }
    }

    fn glyph(&mut self, glyph: Glyph, rect: Rect) {
        self.add(Image::new(keys::ATLAS).with_uv(glyph.uv()), rect);
    }
}

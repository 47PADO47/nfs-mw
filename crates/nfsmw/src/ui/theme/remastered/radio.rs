//! The Remastered radio card: the song on the air on a dark plate that fades out to the right, with a gold bar down its left
//! side, the title in bold, the artist and album under it and the time played in gold. It sits where the HUD puts its card
//! (`custom::radio_placement`, the top left of the HUD) and shares the Custom card's timing: up for six seconds, fading over
//! the last one.

use blackbox_ui::{Component, DrawList, LayoutCx, Panel, Rect, Slant, Text, justify};

use super::group::Group;
use super::skin::keys;
use crate::ui::raster_text::{RASTER_MENU_FONT_KEY, RASTER_MENU_TITLE_FONT_KEY, width_units};
use crate::ui::theme::custom::RadioFeedCell;

/// The plate's size in FEng units at scale 1, and the gold bar's width.
const WIDTH: f32 = 262.0;
const HEIGHT: f32 = 50.0;
const BAR: f32 = 2.5;
/// The plate is dark at its left and clear at its right.
const PLATE_STOPS: [(f32, f32); 3] = [(0.0, 0.92), (0.6, 0.7), (1.0, 0.0)];
/// The text's inset from the plate's left edge, and its width.
const INSET: f32 = 12.0;
const TEXT_WIDTH: f32 = 230.0;
/// Each row's centre below the plate's top, and its glyph scale.
const TITLE_Y: f32 = 15.0;
const CREDIT_Y: f32 = 29.0;
const STATUS_Y: f32 = 42.0;
const TITLE_SCALE: f32 = 1.3;
const CREDIT_SCALE: f32 = 1.0;
const STATUS_SCALE: f32 = 0.95;
const ROW_HEIGHT: f32 = 12.0;

pub struct RadioCard {
    feed: RadioFeedCell,
    parts: Vec<Box<dyn Component>>,
}

impl RadioCard {
    pub fn new(feed: RadioFeedCell) -> Self {
        Self { feed, parts: Vec::new() }
    }
}

impl Component for RadioCard {
    /// The card's place comes from the HUD layout in the feed, not from the slot it is drawn in.
    fn layout(&mut self, cx: &LayoutCx<'_>, _bounds: Rect) {
        self.parts.clear();
        let feed = self.feed.snapshot();
        let opacity = feed.opacity();
        let Some(card) = feed.card.as_ref().filter(|_| opacity > 0.0) else { return };
        let scale = feed.anchor.scale;
        let (left, top) = (feed.anchor.left, feed.anchor.top);
        let (width, height) = (WIDTH * scale, HEIGHT * scale);
        let plate = Rect::new(left + width / 2.0, top + height / 2.0, width, height);
        let mut parts: Vec<Box<dyn Component>> =
            vec![Box::new(Slant::at_default_depth(keys::CLUSTER_BG, plate, 0.0, &PLATE_STOPS, cx))];
        let mut bar = Panel::new(keys::UNIT_HIGHLIGHT);
        bar.layout(cx, Rect::new(left + BAR * scale / 2.0, plate.y, BAR * scale, height));
        parts.push(Box::new(bar));
        let mut row = |text: String, font: &str, raster: u32, colour: &str, size: f32, y: f32| {
            let text = fitted(raster, &text, size, TEXT_WIDTH);
            let mut part = Text::new(font, text)
                .with_colour_key(colour)
                .with_scale(size * scale)
                .with_justification(justify::V_CENTRE);
            let x = left + (INSET + TEXT_WIDTH / 2.0) * scale;
            part.layout(cx, Rect::new(x, top + y * scale, TEXT_WIDTH * scale, ROW_HEIGHT * scale));
            parts.push(Box::new(part));
        };
        let (bold, plain) = (RASTER_MENU_TITLE_FONT_KEY, RASTER_MENU_FONT_KEY);
        row(card.title.to_uppercase(), keys::FONT_MENU_TITLE, bold, keys::VALUE, TITLE_SCALE, TITLE_Y);
        row(card.credit(), keys::FONT_MENU, plain, keys::UNIT, CREDIT_SCALE, CREDIT_Y);
        row(card.status(), keys::FONT_MENU, plain, keys::UNIT_HIGHLIGHT, STATUS_SCALE, STATUS_Y);
        self.parts = vec![Box::new(Group::new(parts, opacity))];
    }

    fn build(&self, out: &mut DrawList) {
        for part in &self.parts {
            part.build(out);
        }
    }
}

/// `text` cut short with "..." so it is no wider than `max` units at `scale` in the raster font `font`. The renderer does not
/// squeeze or clip text, so a long title would run off the plate.
fn fitted(font: u32, text: &str, scale: f32, max: f32) -> String {
    let fits = |text: &str| width_units(font, text, scale).is_none_or(|width| width <= max);
    if fits(text) {
        return text.to_owned();
    }
    let mut kept: Vec<char> = text.chars().collect();
    while !kept.is_empty() {
        kept.pop();
        let cut = format!("{}...", kept.iter().collect::<String>().trim_end());
        if fits(&cut) {
            return cut;
        }
    }
    String::new()
}

#[cfg(test)]
mod tests {
    use blackbox_ui::{LayoutCx, UiNode};

    use super::*;
    use crate::audio::NowPlaying;
    use crate::hud::RadioHud;
    use crate::settings::HudLayout;
    use crate::ui::present::Screen;
    use crate::ui::theme::custom::palette;
    use crate::ui::theme::remastered::skin::{palette_skin, skin};

    const SCREEN: Screen = Screen { width: 1920.0, height: 1080.0, pixels_per_point: 1.0 };

    fn song(paused: bool) -> NowPlaying {
        NowPlaying {
            song: 3,
            artist: "Artist".into(),
            title: "Title".into(),
            album: "Album".into(),
            elapsed_secs: 65.0,
            length_secs: 200.0,
            paused,
            serial: 1,
        }
    }

    /// What the card builds `age` seconds after the song was announced, or nothing for an idle radio.
    fn card_at(now: Option<NowPlaying>, age: f32) -> Vec<UiNode> {
        let mut hud = RadioHud::default();
        if let Some(now) = &now {
            hud.update(Some(now), 0.0);
            hud.update(Some(now), age);
        }
        let feed = RadioFeedCell::new();
        feed.set(&hud, SCREEN, HudLayout::Pc);
        let skin = palette::remastered().skin().merged(skin()).merged(palette_skin(&palette::remastered()));
        let mut card = RadioCard::new(feed);
        card.layout(&LayoutCx::new(&skin, 0.0), Rect::SCREEN);
        let mut out = DrawList::new();
        card.build(&mut out);
        out.iter().cloned().collect()
    }

    fn texts(nodes: &[UiNode]) -> Vec<&str> {
        nodes.iter().filter_map(|node| node.text.as_deref()).collect()
    }

    #[test]
    fn a_long_title_is_cut_to_the_plate() {
        let long = "A SONG WITH A VERY LONG TITLE THAT WOULD RUN OFF THE PLATE (FEATURING SOMEONE)";
        let cut = fitted(RASTER_MENU_TITLE_FONT_KEY, long, TITLE_SCALE, TEXT_WIDTH);
        assert!(cut.ends_with("...") && cut.len() < long.len(), "{cut}");
        assert!(width_units(RASTER_MENU_TITLE_FONT_KEY, &cut, TITLE_SCALE).unwrap_or(0.0) <= TEXT_WIDTH);
        assert_eq!(fitted(RASTER_MENU_TITLE_FONT_KEY, "SHORT", TITLE_SCALE, TEXT_WIDTH), "SHORT");
    }

    #[test]
    fn an_idle_radio_has_no_card() {
        assert!(card_at(None, 0.0).is_empty());
    }

    #[test]
    fn the_card_shows_the_title_in_capitals_the_credit_and_the_time() {
        let nodes = card_at(Some(song(false)), 1.0);
        let texts = texts(&nodes);
        assert_eq!(texts[..2], ["TITLE", "Artist - Album"]);
        assert!(texts[2].starts_with("1:05"), "{texts:?}");
        let paused = card_at(Some(song(true)), 1.0);
        assert!(self::texts(&paused)[2].ends_with("PAUSED"));
    }

    #[test]
    fn the_card_fades_over_its_last_second_and_is_gone_after_six() {
        // Half-way through the fade nothing is brighter than half.
        let fading = card_at(Some(song(false)), 5.5);
        assert!(!fading.is_empty() && fading.iter().all(|n| n.colour[3] <= 128), "fading");
        assert!(card_at(Some(song(false)), 6.5).is_empty(), "gone");
    }
}

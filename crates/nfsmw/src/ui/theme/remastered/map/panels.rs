//! The map's chrome: the title block top left, the compass top right, the legend of the tab's icons at the right, and the
//! dark band under the footer. Places are in FEng units from the screen's edges (`half_width` is half the window's width).

use blackbox_ui::{Outline, Panel, Rect, Slant, Text, justify};

use super::super::menu::Parts;
use super::super::skin::keys;
use super::ease_out;
use super::group::Group;
use crate::frontend::world_map::WorldMapFrame;
use crate::frontend::world_map::control::{blip_rgba, icon_name, style_name};
use crate::ui::theme::remastered::menu::layout as menu;

/// The shade behind the title: dark at the left edge, clear at its end.
const TITLE_STOPS: [(f32, f32); 3] = [(0.0, 0.85), (0.7, 0.55), (1.0, 0.0)];
/// The legend's shade: clear at its left, dark at the window's edge.
const LEGEND_STOPS: [(f32, f32); 3] = [(0.0, 0.0), (0.45, 0.5), (1.0, 0.85)];

const TITLE_Y: f32 = -200.0;
const SUBTITLE_Y: f32 = -168.0;
const TITLE_SCALE: f32 = 3.0;
const SUBTITLE_SCALE: f32 = 2.0;
const TEXT_LEFT: f32 = 30.0;

const COMPASS_RADIUS: f32 = 20.0;
const COMPASS_FROM_EDGE: f32 = 46.0;
const COMPASS_Y: f32 = -196.0;

const LEGEND_WIDTH: f32 = 260.0;
const LEGEND_TOP: f32 = -100.0;
const ROW_PITCH: f32 = 32.0;
const ROW_ICON: f32 = 18.0;
const ROW_SCALE: f32 = 1.55;
/// The legend comes in after the map does, and slides in from the right as it does.
const LEGEND_DELAY: f32 = 0.1;
const LEGEND_SECONDS: f32 = 0.3;
const LEGEND_SLIDE: f32 = 30.0;
const PLAYER_ICON: &str = "Minimap_Icon_Car.tga";

/// The dark band the footer sits on, across the bottom of the window.
pub fn bands(parts: &mut Parts, half_width: f32) {
    let height = 240.0 - menu::RULE_TOP_Y + 6.0;
    let rect = Rect::new(0.0, 240.0 - height / 2.0, half_width * 2.0 + 4.0, height);
    parts.push(Panel::new(keys::MAP_BAND), rect);
}

/// "MAP" in the menus' bold face over a dark shade, and the current tab under it in the accent colour.
pub fn title(parts: &mut Parts, frame: &WorldMapFrame, left: f32) {
    let shade = Rect::new(left + 170.0, -190.0, 340.0, 84.0);
    parts.add(Slant::at_default_depth(keys::MENU_SHADE, shade, 0.0, &TITLE_STOPS, parts.cx()));
    let at = left + TEXT_LEFT;
    parts.text(keys::FONT_MENU_TITLE, "MAP", keys::MENU_TEXT_SELECTED, TITLE_SCALE, (at, TITLE_Y), false);
    let tab = frame.session.filter.label().to_uppercase();
    parts.text(keys::FONT_MENU, &tab, keys::MENU_TITLE, SUBTITLE_SCALE, (at, SUBTITLE_Y), false);
}

/// A ring with a north arrow and an N over it, and the zoom under it. The map is always north up.
pub fn compass(parts: &mut Parts, frame: &WorldMapFrame, half_width: f32) {
    let x = half_width - COMPASS_FROM_EDGE;
    let ring = Rect::new(x, COMPASS_Y, COMPASS_RADIUS * 2.0, COMPASS_RADIUS * 2.0);
    parts.add(Outline::new(keys::MENU_RULE, ring, COMPASS_RADIUS, 1.3, parts.cx()));
    // The arrow: two legs that lean together at the top.
    for lean in [5.0, -5.0] {
        let leg = Rect::new(x + lean * -0.3, COMPASS_Y + 1.0, 2.2, 15.0);
        parts.add(Slant::at_default_depth(keys::MENU_BOLT, leg, lean, &[], parts.cx()));
    }
    let n = Text::new(keys::FONT_MENU, "N")
        .with_colour_key(keys::MENU_TEXT)
        .with_scale(1.3)
        .with_justification(justify::H_CENTRE | justify::V_CENTRE);
    parts.push(n, Rect::new(x, COMPASS_Y - COMPASS_RADIUS - 8.0, 20.0, 10.0));
    let zoom = Text::new(keys::FONT_MENU, format!("ZOOM x{:.1}", frame.session.camera.zoom()))
        .with_colour_key(keys::MENU_TAGLINE)
        .with_scale(1.2)
        .with_justification(justify::H_CENTRE | justify::V_CENTRE);
    parts.push(zoom, Rect::new(x, COMPASS_Y + COMPASS_RADIUS + 14.0, 80.0, 10.0));
}

/// The legend: the player, then each icon the current tab shows, with its name and how many are on the map. The icon of
/// the focused blip's style is lit. It slides and fades in after the map, and the legend key hides it.
pub fn legend(parts: &mut Parts, frame: &WorldMapFrame, half_width: f32) {
    if !frame.session.legend_open {
        return;
    }
    let entrance = ease_out((frame.clock - LEGEND_DELAY) / LEGEND_SECONDS);
    let shift = (1.0 - entrance) * LEGEND_SLIDE;
    let styles = frame.session.filter.styles();
    let rows = styles.len() + 1;
    let right = half_width + shift;
    let mut inner = Parts::new(parts.cx());
    let height = rows as f32 * ROW_PITCH + 50.0;
    let shade = Rect::new(right - LEGEND_WIDTH / 2.0, LEGEND_TOP - 20.0 + height / 2.0, LEGEND_WIDTH, height);
    inner.add(Slant::at_default_depth(keys::MENU_SHADE, shade, 0.0, &LEGEND_STOPS, parts.cx()));
    let focused = frame.session.attached.and_then(|index| frame.blips.get(index)).map(|blip| blip.style);
    let icon_x = right - 186.0;
    let name_x = right - 162.0;
    let count_x = right - 28.0;
    let y_of = |row: usize| LEGEND_TOP + row as f32 * ROW_PITCH;
    inner.push(blackbox_ui::Image::new(PLAYER_ICON), Rect::new(icon_x, y_of(0), ROW_ICON, ROW_ICON));
    inner.text(keys::FONT_MENU, "PLAYER", keys::MENU_TEXT, ROW_SCALE, (name_x, y_of(0)), false);
    for (k, style) in styles.iter().enumerate() {
        let y = y_of(k + 1);
        let icon = blackbox_ui::Image::new(icon_name(style)).with_tint(blip_rgba(style));
        inner.push(icon, Rect::new(icon_x, y, ROW_ICON, ROW_ICON));
        let colour = match focused == Some(*style) {
            true => keys::MENU_TITLE,
            false => keys::MENU_TEXT,
        };
        inner.text(keys::FONT_MENU, &style_name(style).to_uppercase(), colour, ROW_SCALE, (name_x, y), false);
        let count = frame.blips.iter().filter(|blip| blip.style == *style).count();
        inner.text(keys::FONT_MENU, &count.to_string(), keys::MENU_TAGLINE, ROW_SCALE, (count_x, y), true);
    }
    parts.add(Group::new(inner.finish(), entrance));
}

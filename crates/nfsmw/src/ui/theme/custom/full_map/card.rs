//! The map's cursor and the selected item's card. The system pointer is hidden over a themed full-screen map
//! (`frontend/plugin_pointer.rs`), so the map draws its own: a ring round the blip the cursor is focused on, or a small
//! cross where the pointer is. The focused blip's card sits at the safe area's bottom left, above the footer: its mark,
//! its name, how far it is from the player and how to set the GPS to it.

use blackbox_ui::{Component, LayoutCx, LineCap, Polyline, Rect};
use glam::Vec2;

use super::chrome::{Align, CONTENT_Z, text};
use super::{fit, places};
use crate::frontend::world_map::control::{format_metres, style_name};
use crate::frontend::world_map::{WorldMapFrame, distance_metres};
use crate::i18n::t;
use crate::ui::theme::custom::map::draw::{colour, mark_parts, ring};
use crate::ui::theme::custom::menu::layout::MenuFrame;
use crate::ui::theme::custom::plate::{Plate, PlateStyle};
use crate::ui::theme::custom::text::Label;
use crate::ui::theme::custom::tokens::keys;

/// The cursor's depth: over the marks and the player's arrow (`map::ARROW_Z`), under the chrome.
pub const CURSOR_Z: f32 = 56.0;
/// The ring round the focused blip: its radius and stroke, at scale 1.
const RING_RADIUS: f32 = 11.0;
const RING_STROKE: f32 = 1.5;
/// The free cursor's cross: the length of each arm, the gap at the middle and the stroke, at scale 1.
const CROSS_ARM: f32 = 7.0;
const CROSS_GAP: f32 = 3.0;
const CROSS_STROKE: f32 = 1.5;
/// The card: its width, its height, its gap above the footer line, its padding and its sizes, at scale 1.
const CARD_WIDTH: f32 = 230.0;
const CARD_HEIGHT: f32 = 62.0;
const CARD_LIFT: f32 = 22.0;
const CARD_PAD: f32 = 12.0;
const CARD_MARK: f32 = 1.6;
const NAME_SIZE: f32 = 16.0;
const DISTANCE_SIZE: f32 = 14.0;
const GPS_SIZE: f32 = 11.0;

/// The cursor: a ring round the focused blip, or a cross at the pointer. Nothing when the pointer is off the window
/// and no blip is focused.
pub fn cursor(map: &WorldMapFrame, scale: f32, cx: &LayoutCx<'_>, out: &mut Vec<Box<dyn Component>>) {
    let camera = map.session.camera;
    if let Some(blip) = places::focused(map) {
        let at = fit::on_screen(camera, blip.map);
        let stroke = RING_STROKE * scale;
        out.push(Box::new(ring(at, RING_RADIUS * scale, stroke, colour(cx, keys::ACCENT), CURSOR_Z)));
        return;
    }
    let Some(view) = map.session.cursor else { return };
    let at = fit::cursor_on_screen(view);
    let reach = (CROSS_GAP + CROSS_ARM) * scale;
    let near = CROSS_GAP * scale;
    for direction in [Vec2::X, Vec2::NEG_X, Vec2::Y, Vec2::NEG_Y] {
        let arm = vec![at + direction * near, at + direction * reach];
        let line = Polyline::new(arm, CROSS_STROKE * scale, colour(cx, keys::TEXT)).with_cap(LineCap::Square);
        out.push(Box::new(line.with_z(CURSOR_Z)));
    }
}

/// The focused blip's card: a raised plate with an accent line, the blip's mark and name, its distance from the player
/// (only when the player's place is known) and how to set the GPS to it. Nothing when no blip is focused.
pub fn card(map: &WorldMapFrame, frame: &MenuFrame, cx: &LayoutCx<'_>, out: &mut Vec<Box<dyn Component>>) {
    let Some(blip) = places::focused(map) else { return };
    let (safe, s, sizing) = (frame.safe(), frame.scale(), frame.sizing());
    let (width, height) = (CARD_WIDTH * s, CARD_HEIGHT * s);
    let bottom = frame.footer_y() - CARD_LIFT * s;
    let rect = Rect::new(safe.min_x() + width / 2.0, bottom - height / 2.0, width, height);
    out.push(Box::new(Plate::new(rect, PlateStyle::RAISED.with_accent(keys::ACCENT).scaled(s), cx)));
    let left = rect.min_x() + CARD_PAD * s;
    let top = rect.min_y() + CARD_PAD * s;
    let name_y = top + NAME_SIZE * s / 2.0;
    let mark = Vec2::new(left + 4.0 * s, name_y);
    mark_parts(places::kind_of(blip.style), mark, (1.0, CONTENT_Z), s * CARD_MARK, cx, out);
    let name_x = left + 16.0 * s;
    out.push(text(Label::label(style_name(blip.style), NAME_SIZE), (name_x, name_y), Align::Left, sizing, cx));
    if map.player_mark.is_some() {
        let distance = format_metres(distance_metres(map.player, blip));
        let distance = Label::numeric(distance, DISTANCE_SIZE).colour(keys::NAV);
        out.push(text(distance, (rect.max_x() - CARD_PAD * s, name_y), Align::Right, sizing, cx));
    }
    let gps = Label::label(t!("map-card-gps"), GPS_SIZE).colour(keys::TEXT_SECONDARY);
    out.push(text(gps, (name_x, rect.max_y() - CARD_PAD * s - GPS_SIZE * s / 2.0), Align::Left, sizing, cx));
}

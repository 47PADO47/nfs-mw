//! The world map's FEng screen (`WorldMapMain.fng`). Each tick it draws the open map's frame (`Cx::world_map`) into the
//! package's own objects: the picture is cropped by the camera, the blips take the icon pools of their style, and the
//! list, the cursor, the card and the tab are the package's objects too. The pure parts are in `place.rs`, the object
//! GUIDs in `pools.rs`. Spec: `docs/specs/world-map.md`.

mod place;
mod pools;
#[cfg(test)]
mod tests;

use blackbox_feng::{ObjectRef, resource_handle};

use super::control::{format_metres, icon_name, style_name, tint_of};
use super::{Camera, Filter, PlacedBlip, WorldMapFrame, distance_metres, layout};
use crate::frontend::logic::{Cx, ScreenLogic};

/// Where the list's text starts (its left edge) and the width it is aligned in, in FEng units.
const LIST_LEFT: f32 = 90.0;
const LIST_WIDTH: f32 = 230.0;
/// The centre of the first row, and the distance between rows.
const LIST_TOP: f32 = -100.0;
const ROW_STEP: f32 = 22.0;
/// The centre of a row's icon. The package's icon sits 4 units right of its group's position.
const ICON_X: f32 = 70.0;
const ICON_OFFSET: f32 = 4.0;
const WHITE: [u8; 3] = [255, 255, 255];
const YELLOW: [u8; 3] = [255, 255, 0];
/// The text the list shows when no blip of the filter is on the map.
const EMPTY: &str = "No blips of this type on the map.";
/// The prompt's text on the card.
const PROMPT_TITLE: &str = "Activate GPS?";
const PROMPT_HINT: &str = "Enter: yes   Esc: no";

/// The map's screen logic. It keeps no state of its own: the frontend hands it the frame each tick.
#[derive(Default)]
pub struct WorldMapLogic;

impl ScreenLogic for WorldMapLogic {
    fn message(&mut self, _cx: &mut Cx, _message: u32) {}

    fn tick(&mut self, cx: &mut Cx, _dt: f32) {
        let Some(frame) = cx.world_map else { return };
        let session = &frame.session;
        let drawn = place::assign(&frame.blips, session.filter);
        draw_picture(cx, session.camera, session.filter);
        draw_blips(cx, frame, &drawn);
        draw_list(cx, frame);
        draw_cursor_and_card(cx, frame);
    }
}

/// The part of the picture the view shows (the picture's texture crop). The tab and the zoom readout are written here
/// too.
fn draw_picture(cx: &mut Cx, camera: Camera, filter: Filter) {
    if let Some(o) = cx.guid(pools::PICTURE) {
        cx.rt.set_uv(o, camera.uv());
    }
    set_text(cx, pools::ZOOM, &format!("Zoom x{:.1}", camera.zoom()));
    set_text(cx, pools::TAB, &filter.label().to_uppercase());
}

/// Every pool object is parked first; the blips that are drawn and inside the view are then placed on the picture.
fn draw_blips(cx: &mut Cx, frame: &WorldMapFrame, drawn: &[place::Drawn]) {
    for guid in pools::POOLS.iter().flat_map(|(_, slots)| slots.iter().copied()) {
        hide(cx, guid, true);
    }
    let camera = frame.session.camera;
    for d in drawn {
        let blip = &frame.blips[d.blip];
        let view = camera.to_view(blip.map);
        if view.iter().any(|v| !(0.0..=1.0).contains(v)) {
            continue;
        }
        let Some(o) = cx.guid(d.guid) else { continue };
        let [x, y] = layout::point_of(view);
        cx.rt.set_position_xy(o, x, y);
        show_icon(cx, o, blip.style);
    }
}

/// The icon of a style on an image: its texture and its tint (the HUD's own for the interiors).
fn show_icon(cx: &mut Cx, o: ObjectRef, style: &str) {
    cx.rt.set_texture(o, resource_handle(icon_name(style)));
    let [r, g, b] = tint_of(style).unwrap_or(WHITE);
    cx.rt.set_colour(o, [r, g, b]);
    cx.rt.set_hidden(o, false);
}

/// The list of the filter's blips, seven rows from the window's top. The attached blip's row is yellow.
fn draw_list(cx: &mut Cx, frame: &WorldMapFrame) {
    let session = &frame.session;
    let listed = session.listed(&frame.blips);
    let top = place::window_top(&listed, session.attached, pools::ROWS.len());
    for (k, row) in pools::ROWS.iter().enumerate() {
        let y = LIST_TOP + k as f32 * ROW_STEP;
        match listed.get(top + k) {
            Some(&index) => {
                let blip = &frame.blips[index];
                let attached = session.attached == Some(index);
                draw_row(cx, row, y, &place::row_text(blip, frame.player), blip, attached);
            }
            None if k == 0 && listed.is_empty() => {
                draw_row_text(cx, row.text, y, EMPTY, WHITE);
                hide(cx, row.icon_group, true);
            }
            None => {
                hide(cx, row.text, true);
                hide(cx, row.icon_group, true);
            }
        }
    }
}

fn draw_row(cx: &mut Cx, row: &pools::Row, y: f32, text: &str, blip: &PlacedBlip, attached: bool) {
    let colour = if attached { YELLOW } else { WHITE };
    draw_row_text(cx, row.text, y, text, colour);
    let Some(group) = cx.guid(row.icon_group) else { return };
    cx.rt.set_position_xy(group, ICON_X - ICON_OFFSET, y);
    cx.rt.set_hidden(group, false);
    if let Some(icon) = cx.guid(row.icon) {
        show_icon(cx, icon, blip.style);
    }
}

fn draw_row_text(cx: &mut Cx, guid: u32, y: f32, text: &str, colour: [u8; 3]) {
    let Some(o) = cx.guid(guid) else { return };
    cx.rt.set_text(o, text);
    cx.rt.set_colour(o, colour);
    cx.rt.set_hidden(o, false);
    cx.align_string(o, LIST_LEFT, LIST_WIDTH, y);
}

/// The cursor sits on the attached blip, and the card under the picture says which blip it is and how far. With the
/// prompt open the card asks for the GPS instead.
fn draw_cursor_and_card(cx: &mut Cx, frame: &WorldMapFrame) {
    let session = &frame.session;
    let attached = session.attached.and_then(|i| frame.blips.get(i));
    let Some(blip) = attached else {
        hide(cx, pools::CURSOR, true);
        hide(cx, pools::CARD, true);
        return;
    };
    let [x, y] = layout::point_of(session.camera.to_view(blip.map));
    if let Some(cursor) = cx.guid(pools::CURSOR) {
        cx.rt.set_position_xy(cursor, x, y);
        cx.rt.set_hidden(cursor, false);
    }
    hide(cx, pools::CARD, false);
    let (title, detail) = match session.prompt {
        true => (PROMPT_TITLE.to_owned(), PROMPT_HINT.to_owned()),
        false => {
            let metres = distance_metres(frame.player, blip);
            (style_name(blip.style).to_owned(), format_metres(metres))
        }
    };
    set_text(cx, pools::CARD_TITLE, &title);
    set_text(cx, pools::CARD_DISTANCE, &detail);
    if let Some(icon) = cx.guid(pools::CARD_ICON) {
        show_icon(cx, icon, blip.style);
    }
}

fn set_text(cx: &mut Cx, guid: u32, text: &str) {
    if let Some(o) = cx.guid(guid) {
        cx.rt.set_text(o, text);
    }
}

fn hide(cx: &mut Cx, guid: u32, hidden: bool) {
    if let Some(o) = cx.guid(guid) {
        cx.rt.set_hidden(o, hidden);
    }
}

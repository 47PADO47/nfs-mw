//! The world map's FEng screen (`WorldMapMain.fng`). Each tick it draws the open map's frame (`Cx::world_map`) into the
//! package's own objects: the picture is cropped by the camera, the blips take the icon pools of their style, and the
//! list, the cursor, the card and the tab are the package's objects too. The pure parts are in `place.rs`, the object
//! GUIDs in `pools.rs`. Spec: `docs/specs/world-map.md`.

mod place;
mod pools;
mod restyle;
#[cfg(test)]
mod tests;

use blackbox_feng::{ObjectRef, resource_handle};
use nfsmw_career::regions::MapVariant;

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

/// The map's screen logic. It keeps no state of its own: the frontend hands it the frame each tick. The variant is the
/// career stage's map (`MapVariant::of_stage`, or the full map outside career mode), chosen when the map opens.
pub struct WorldMapLogic {
    pub variant: MapVariant,
    /// The blip the cursor was on last tick, to replay the snap animation when it changes.
    pub last_attached: Option<usize>,
}

impl ScreenLogic for WorldMapLogic {
    fn message(&mut self, _cx: &mut Cx, _message: u32) {}

    fn tick(&mut self, cx: &mut Cx, _dt: f32) {
        let Some(frame) = cx.world_map else { return };
        let session = &frame.session;
        let attached = session.attached;
        let drawn = place::assign(&frame.blips, session.filter);
        draw_picture(cx, session.camera, session.filter, self.variant);
        draw_blips(cx, frame, &drawn);
        // Every map hides the stripe lines and shows the frozen world through a translucent background. Under the
        // Remastered theme the screen is also recoloured dark in place (keeping its own fullscreen picture, filter
        // switch, button prompts and mouse behaviour), its own list and legend chrome are hidden, and the themed legend
        // is drawn over it; the cursor still sits on the attached blip.
        restyle::transparency(cx);
        if frame.chrome_themed {
            restyle::dark(cx);
            // The themed legend replaces the FEng list, so hide the list rows; the cursor still draws so the mouse keeps
            // snapping onto blips (a click, Enter or right click then asks for GPS in the stock dialog).
            for row in pools::ROWS.iter() {
                hide(cx, row.text, true);
                hide(cx, row.icon_group, true);
            }
            // The themed legend shows the focused blip's distance, so the FEng card stays hidden; the cursor still draws.
            draw_cursor_and_card(cx, frame);
            hide(cx, pools::CARD, true);
        } else {
            draw_list(cx, frame);
            draw_cursor_and_card(cx, frame);
        }
        // The "Activate GPS" button is greyed until a blip is selected.
        set_activate_enabled(cx, attached.is_some());
        show_key_prompts(cx);
        // Replay the cursor's snap animation when it moves onto (or off) a blip.
        if attached != self.last_attached {
            let message = if attached.is_some() { pools::CURSOR_SNAP } else { pools::CURSOR_UNSNAP };
            for guid in pools::CURSOR_CROSSHAIRS {
                if let Some(o) = cx.guid(guid) {
                    cx.rt.post(cx.package, message, Some(o));
                }
            }
            self.last_attached = attached;
        }
    }
}

/// Colours the "Activate GPS" button: bright when a blip is selected, muted grey when nothing is.
fn set_activate_enabled(cx: &mut Cx, selected: bool) {
    let colour = if selected { [226, 230, 238] } else { [110, 115, 125] };
    if let Some(o) = cx.guid(pools::ACTIVATE_TEXT) {
        cx.rt.set_colour(o, colour);
    }
}

/// Shows the keyboard-prompt icons before the Back and Activate GPS labels (the package starts them hidden for gamepad
/// mode; the port is keyboard-and-mouse driven).
fn show_key_prompts(cx: &mut Cx) {
    for guid in [pools::KEY_BACK, pools::KEY_ACTIVATE] {
        if let Some(o) = cx.guid(guid) {
            cx.rt.set_colour_rgba(o, [255, 255, 255, 255]);
            cx.rt.set_hidden(o, false);
        }
    }
}

/// The part of the picture the view shows (the picture's texture crop) and the variant's texture of it. The tab and
/// the zoom readout are written here too.
fn draw_picture(cx: &mut Cx, camera: Camera, filter: Filter, variant: MapVariant) {
    if let Some(o) = cx.guid(pools::PICTURE) {
        cx.rt.set_uv(o, camera.uv());
        set_picture_texture(cx, o, variant);
    }
    set_text(cx, pools::ZOOM, &format!("Zoom x{:.1}", camera.zoom()));
    set_text(cx, pools::TAB, &filter.label().to_uppercase());
}

/// The picture's texture is the variant's world map texture. Without it in the install the package keeps its own.
fn set_picture_texture(cx: &mut Cx, o: ObjectRef, variant: MapVariant) {
    let key = resource_handle(variant.world_map_texture());
    if cx.assets.texture(key).is_none() {
        return;
    }
    cx.rt.set_texture(o, key);
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

/// The cursor follows the mouse as a cross and sits on the focused blip as a square (the snap animation runs when it
/// changes). The card under the picture names the focused blip and its distance.
fn draw_cursor_and_card(cx: &mut Cx, frame: &WorldMapFrame) {
    let session = &frame.session;
    // The round glow inside the cursor group is the package's "normal" pointer; hide it so only the cross/square
    // crosshairs show.
    hide(cx, pools::CURSOR_GLOW, true);
    let attached = session.attached.and_then(|i| frame.blips.get(i));
    // The cursor sits on the focused blip, else it follows the free mouse pointer.
    let view = match attached {
        Some(blip) => Some(session.camera.to_view(blip.map)),
        None => session.cursor,
    };
    match view.filter(|v| v.iter().all(|c| (0.0..=1.0).contains(c))) {
        Some(view) => {
            let [x, y] = layout::point_of(view);
            if let Some(cursor) = cx.guid(pools::CURSOR) {
                cx.rt.set_position_xy(cursor, x, y);
                cx.rt.set_hidden(cursor, false);
            }
        }
        None => hide(cx, pools::CURSOR, true),
    }
    // The card shows only for the focused blip.
    let Some(blip) = attached else {
        hide(cx, pools::CARD, true);
        return;
    };
    hide(cx, pools::CARD, false);
    set_text(cx, pools::CARD_TITLE, style_name(blip.style));
    set_text(cx, pools::CARD_DISTANCE, &format_metres(distance_metres(frame.player, blip)));
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

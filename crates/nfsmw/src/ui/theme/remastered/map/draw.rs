//! What is drawn on the map's picture: the picture itself, the blips as the game's own icons, the player's car, and the
//! cursor. Places come from the session's camera through `Picture::FULL` (the picture is the screen's height, centred; the
//! window shows the map beyond it).

use blackbox_feng::resource_handle;
use blackbox_ui::{Image, Panel, Rect, Rounded, Slant, Text, justify};

use super::super::menu::Parts;
use super::super::skin::keys;
use super::{MapSnapshot, ease_out};
use crate::frontend::world_map::control::{blip_rgba, format_metres, icon_name, style_name};
use crate::frontend::world_map::layout::Picture;
use crate::frontend::world_map::{PlacedBlip, WorldMapFrame, distance_metres};
use crate::ui::raster_text::{RASTER_MENU_FONT_KEY, width_units};

/// The picture's tint (it multiplies the map's own colours, which are light), the width of the fade at its left and right
/// edges, and the fade's ramp on the left edge (the right edge is its mirror).
const PICTURE_TINT: [u8; 4] = [235, 238, 246, 255];
const EDGE: f32 = 70.0;
const EDGE_STOPS: [(f32, f32); 3] = [(0.0, 1.0), (0.6, 0.5), (1.0, 0.0)];

/// The size of a blip's icon, in FEng units, and how much larger the focused one is drawn.
const ICON: f32 = 16.0;
const FOCUSED_SCALE: f32 = 1.3;
/// The player's car icon.
const PLAYER_ICON: &str = "Minimap_Icon_Car.tga";
const PLAYER: f32 = 20.0;
/// Past this far from the screen's centre a blip is off the window.
const OFF_X: f32 = 520.0;
const OFF_Y: f32 = 270.0;

/// The snap: how long the cursor takes to close onto a blip, and the size of its brackets before and after.
const SNAP_SECONDS: f32 = 0.18;
const BRACKET_FAR: f32 = 46.0;
const BRACKET_NEAR: f32 = 27.0;
const BRACKET_ARM: f32 = 7.0;
const BRACKET_EDGE: f32 = 1.8;
/// The free cursor: the length of each arm of its cross, the gap at the middle and the stroke.
const CROSS_ARM: f32 = 8.0;
const CROSS_GAP: f32 = 3.0;
const CROSS_EDGE: f32 = 1.4;
/// The label beside a focused blip: the text size, the padding round it and how far it sits from the brackets.
const TIP_SCALE: f32 = 1.45;
const TIP_PAD: f32 = 6.0;
const TIP_HEIGHT: f32 = 17.0;
const TIP_GAP: f32 = 8.0;

/// The screen point of a view position of the picture.
fn at(view: [f32; 2]) -> (f32, f32) {
    let [x, y] = Picture::FULL.point_of(view);
    (x, y)
}

/// The career stage's picture, sized by the zoom and placed by the camera.
pub fn picture(parts: &mut Parts, frame: &WorldMapFrame) {
    let camera = frame.session.camera;
    let (x, y) = at(camera.to_view([0.5, 0.5]));
    let side = Picture::FULL.side * camera.zoom();
    let key = frame.event_picture.unwrap_or_else(|| resource_handle(frame.texture));
    let map = Image::raw(key).with_tint(PICTURE_TINT);
    parts.push(map, Rect::new(x, y, side, side));
    // A dark veil over it, so the roads read against the land and the icons stand out; and the left and right edges of the
    // picture fade into the ground instead of ending hard.
    parts.push(Panel::new(keys::MAP_VEIL), Rect::new(x, y, side, side));
    for (dx, stops) in [(-1.0, EDGE_STOPS), (1.0, [(0.0, 0.0), (0.4, 0.5), (1.0, 1.0)])] {
        let edge = Rect::new(x + dx * (side / 2.0 - EDGE / 2.0), y, EDGE, side);
        parts.add(Slant::at_default_depth(keys::MAP_GROUND, edge, 0.0, &stops, parts.cx()));
    }
}

/// The icon of every blip of the current tab that is in the window. The focused blip is drawn a little larger.
pub fn blips(parts: &mut Parts, frame: &WorldMapFrame) {
    let camera = frame.session.camera;
    for index in frame.session.listed(&frame.blips) {
        let blip = &frame.blips[index];
        let (x, y) = at(camera.to_view(blip.map));
        if x.abs() > OFF_X || y.abs() > OFF_Y {
            continue;
        }
        let size = match frame.session.attached == Some(index) {
            true => ICON * FOCUSED_SCALE,
            false => ICON,
        };
        parts.push(Image::new(icon_name(blip.style)).with_tint(blip_rgba(blip.style)), Rect::new(x, y, size, size));
    }
    live(parts, frame);
}

/// The racers and the police cars, on every tab: the car arrow in their colour, turned to where they point.
fn live(parts: &mut Parts, frame: &WorldMapFrame) {
    let camera = frame.session.camera;
    for car in &frame.live {
        let (x, y) = at(camera.to_view(car.map));
        if x.abs() > OFF_X || y.abs() > OFF_Y {
            continue;
        }
        let icon = Image::new(PLAYER_ICON).with_tint(car.rgba(frame.clock)).with_rotation(car.bearing.to_radians());
        parts.push(icon, Rect::new(x, y, ICON, ICON));
    }
}

/// The player's car, turned to its bearing.
pub fn player(parts: &mut Parts, frame: &WorldMapFrame) {
    let Some(mark) = frame.player_mark else { return };
    let (x, y) = at(frame.session.camera.to_view(mark.map));
    if x.abs() > OFF_X || y.abs() > OFF_Y {
        return;
    }
    let car = Image::new(PLAYER_ICON).with_rotation(mark.bearing.to_radians());
    parts.push(car, Rect::new(x, y, PLAYER, PLAYER));
}

/// The cursor: brackets closed on the focused blip with its label, or a cross where the pointer is.
pub fn cursor(parts: &mut Parts, snapshot: &MapSnapshot) {
    let frame = &snapshot.frame;
    let session = &frame.session;
    match session.attached.and_then(|index| frame.blips.get(index)) {
        Some(blip) => latched(parts, snapshot, blip),
        None => free(parts, session.cursor),
    }
}

/// A cross of four short strokes round a gap, at the pointer. Nothing when the pointer is outside the window.
fn free(parts: &mut Parts, cursor: Option<[f32; 2]>) {
    let Some(view) = cursor else { return };
    let (x, y) = at(view);
    let reach = CROSS_GAP + CROSS_ARM / 2.0;
    for (dx, dy, w, h) in [
        (-reach, 0.0, CROSS_ARM, CROSS_EDGE),
        (reach, 0.0, CROSS_ARM, CROSS_EDGE),
        (0.0, -reach, CROSS_EDGE, CROSS_ARM),
        (0.0, reach, CROSS_EDGE, CROSS_ARM),
    ] {
        parts.push(Panel::new(keys::MENU_TEXT_SELECTED), Rect::new(x + dx, y + dy, w, h));
    }
}

/// Four corner brackets that close from far to near over [`SNAP_SECONDS`] after the blip is focused, and the label.
fn latched(parts: &mut Parts, snapshot: &MapSnapshot, blip: &PlacedBlip) {
    let frame = &snapshot.frame;
    let (x, y) = at(frame.session.camera.to_view(blip.map));
    let t = ease_out(frame.snap_age / SNAP_SECONDS);
    let half = (BRACKET_FAR + (BRACKET_NEAR - BRACKET_FAR) * t) / 2.0;
    for (sx, sy) in [(-1.0, -1.0), (1.0, -1.0), (-1.0, 1.0), (1.0, 1.0)] {
        let across = Rect::new(
            x + sx * (half - BRACKET_ARM / 2.0),
            y + sy * (half - BRACKET_EDGE / 2.0),
            BRACKET_ARM,
            BRACKET_EDGE,
        );
        let down = Rect::new(
            x + sx * (half - BRACKET_EDGE / 2.0),
            y + sy * (half - BRACKET_ARM / 2.0),
            BRACKET_EDGE,
            BRACKET_ARM,
        );
        parts.push(Panel::new(keys::MENU_BOLT), across);
        parts.push(Panel::new(keys::MENU_BOLT), down);
    }
    let label =
        format!("{}  ·  {}", style_name(blip.style).to_uppercase(), format_metres(distance_metres(frame.player, blip)));
    label_beside(parts, &label, (x, y), half, snapshot.aspect * 240.0);
}

/// The focused blip's label in a small dark plate, to the right of the brackets, or to the left near the window's edge.
fn label_beside(parts: &mut Parts, label: &str, at: (f32, f32), half: f32, half_width: f32) {
    let width = width_units(RASTER_MENU_FONT_KEY, label, TIP_SCALE).unwrap_or(label.chars().count() as f32 * 5.0);
    let plate = width + 2.0 * TIP_PAD;
    let right = at.0 + half + TIP_GAP + plate;
    let left = match right < half_width - 12.0 {
        true => at.0 + half + TIP_GAP,
        false => at.0 - half - TIP_GAP - plate,
    };
    let rect = Rect::new(left + plate / 2.0, at.1, plate, TIP_HEIGHT);
    let radius = TIP_HEIGHT / 2.0;
    let background = Rounded::new(keys::MAP_TIP, rect, radius, parts.cx());
    parts.add(background);
    let text = Text::new(keys::FONT_MENU, label)
        .with_colour_key(keys::MENU_TEXT_SELECTED)
        .with_scale(TIP_SCALE)
        .with_justification(justify::V_CENTRE);
    parts.push(text, Rect::new(left + TIP_PAD + width / 2.0, at.1, width, 12.0));
}

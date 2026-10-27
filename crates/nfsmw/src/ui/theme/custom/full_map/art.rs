//! The game's own art on the full-screen map: the picture of the career stage's map (or the running race's own, with its
//! route), and the icon textures of the blips, the cars and the checkpoint, the same files the Remastered map draws
//! (`frontend/world_map/control.rs` `icon_name`, `blip_rgba`; `frame.rs` `LiveBlip::icon`). Nothing here is drawn by hand:
//! the roads are the picture's, and every mark is an icon of the game. The picture and the icons are placed through the
//! frontend's camera onto `Picture::FULL`, the place the cursor picks the blips by, so a blip lies where it is found.
//!
//! What is the Custom look's: the picture's tint and the veil over it are the theme's tokens (`theme.line`,
//! `theme.scrim`), the second player's car is the `info` colour and the route's end is the `guide` colour. The icons keep
//! the game's own tints (the interiors' colours, the cop's light bar).

use blackbox_feng::resource_handle;
use blackbox_ui::{Component, Image, LayoutCx, Rect, token_keys};
use glam::Vec2;

use super::chrome::panel;
use super::fit::on_screen;
use crate::frontend::world_map::control::{CHECKPOINT_ICON, blip_rgba, icon_name};
use crate::frontend::world_map::layout::Picture;
use crate::frontend::world_map::{PlayerMark, WorldMapFrame};
use crate::hud::HELICOPTER;
use crate::ui::theme::custom::map::draw::colour;
use crate::ui::theme::custom::map::{ARROW_Z, MARK_Z, ROAD_Z};
use crate::ui::theme::custom::tokens::fade;

/// The depths of the art: the picture farthest, the veil just in front of it, then the cars, the blips and the
/// player's car. The route's strokes (`ROUTE_Z`) lie between the veil and the icons.
pub const PICTURE_Z: f32 = ROAD_Z;
pub const VEIL_Z: f32 = ROAD_Z - 0.2;
const LIVE_Z: f32 = MARK_Z - 0.5;
const HELICOPTER_Z: f32 = ARROW_Z - 0.5;
/// The share of the scrim's alpha that lies over the picture, so the roads read against the land and the icons stand out.
const VEIL_SHARE: f32 = 0.4;
/// The icons' sides at scale 1, in canvas units: a blip, the player's car, the police helicopter (as large as the HUD
/// package draws it, so it shows round the car), and how much larger the focused blip and the route's end are drawn.
pub const ICON: f32 = 16.0;
const PLAYER: f32 = 22.0;
const HELICOPTER_SIZE: f32 = 30.0;
const FOCUSED_GAIN: f32 = 1.3;
const TARGET_GAIN: f32 = 1.25;
/// The player's car icon, and the tint that leaves a texture as it is.
pub const PLAYER_ICON: &str = "Minimap_Icon_Car.tga";
pub const NO_TINT: [u8; 4] = [255; 4];

/// The icon file `file` centred on `at`, `side` across, in `tint` and turned `turn` radians clockwise, at depth `z`.
pub fn icon(
    file: &str,
    at: Vec2,
    side: f32,
    (tint, turn): ([u8; 4], f32),
    z: f32,
    cx: &LayoutCx<'_>,
) -> Box<dyn Component> {
    let mut image = Image::new(file).with_tint(tint).with_rotation(turn).with_z(z);
    image.layout(cx, Rect::new(at.x, at.y, side, side));
    Box::new(image)
}

/// Whether `at` is in `window`, or within an icon's reach of it.
fn near(window: Rect, at: Vec2) -> bool {
    let reach = HELICOPTER_SIZE;
    (at.x - window.x).abs() <= window.w / 2.0 + reach && (at.y - window.y).abs() <= window.h / 2.0 + reach
}

/// The career stage's picture, or the running race's own once it is read, sized by the camera's zoom and centred where
/// the camera puts it, with the veil over it. Both fade in with `open` (0 to 1).
pub fn picture(map: &WorldMapFrame, open: f32, cx: &LayoutCx<'_>, out: &mut Vec<Box<dyn Component>>) {
    let camera = map.session.camera;
    let at = on_screen(camera, [0.5, 0.5]);
    let side = Picture::FULL.side * camera.zoom();
    let rect = Rect::new(at.x, at.y, side, side);
    let key = map.event_picture.unwrap_or_else(|| resource_handle(map.texture));
    let tint = fade(colour(cx, token_keys::LINE), open);
    let mut image = Image::raw(key).with_tint(tint).with_z(PICTURE_Z);
    image.layout(cx, rect);
    out.push(Box::new(image));
    out.push(panel(token_keys::SCRIM, rect, (VEIL_SHARE * open, VEIL_Z), cx));
}

/// The icon of every blip the current tab shows that is in the window; the focused one is drawn a little larger.
pub fn blips(
    map: &WorldMapFrame,
    (scale, open): (f32, f32),
    window: Rect,
    cx: &LayoutCx<'_>,
    out: &mut Vec<Box<dyn Component>>,
) {
    let camera = map.session.camera;
    for index in map.session.listed(&map.blips) {
        let Some(blip) = map.blips.get(index) else { continue };
        let at = on_screen(camera, blip.map);
        if !near(window, at) {
            continue;
        }
        let gain = match map.session.attached == Some(index) {
            true => FOCUSED_GAIN,
            false => 1.0,
        };
        let tint = fade(blip_rgba(blip.style), open);
        out.push(icon(icon_name(blip.style), at, ICON * scale * gain, (tint, 0.0), MARK_Z, cx));
    }
}

/// The racers, the police cars, the checkpoint and the helicopter, which every tab shows: the game's icon in its colour,
/// turned to where it points. The helicopter is drawn last, over the cars.
pub fn live(
    map: &WorldMapFrame,
    (scale, open): (f32, f32),
    window: Rect,
    cx: &LayoutCx<'_>,
    out: &mut Vec<Box<dyn Component>>,
) {
    let camera = map.session.camera;
    for air in [false, true] {
        for car in map.live.iter().filter(|car| (car.style == HELICOPTER) == air) {
            let at = on_screen(camera, car.map);
            if !near(window, at) {
                continue;
            }
            let (side, z) = match air {
                true => (HELICOPTER_SIZE, HELICOPTER_Z),
                false => (ICON, LIVE_Z),
            };
            let tint = fade(car.rgba(map.clock), open);
            out.push(icon(car.icon(), at, side * scale, (tint, car.bearing.to_radians()), z, cx));
        }
    }
}

/// The players' cars: the second player's in the `info` colour, then the first's, each the game's car arrow turned to its
/// bearing.
pub fn cars(
    map: &WorldMapFrame,
    (scale, open): (f32, f32),
    window: Rect,
    cx: &LayoutCx<'_>,
    out: &mut Vec<Box<dyn Component>>,
) {
    let camera = map.session.camera;
    let second = colour(cx, token_keys::INFO);
    let marks: [(Option<PlayerMark>, [u8; 4]); 2] = [(map.second_mark, second), (map.player_mark, NO_TINT)];
    for (mark, tint) in marks {
        let Some(mark) = mark else { continue };
        let at = on_screen(camera, mark.map);
        if !near(window, at) {
            continue;
        }
        let tint = fade(tint, open);
        out.push(icon(PLAYER_ICON, at, PLAYER * scale, (tint, mark.bearing.to_radians()), ARROW_Z, cx));
    }
}

/// The GPS route's end: the game's checkpoint icon in the route's colour. Nothing without a route or the city's
/// calibration, which places the end on the picture.
pub fn route_end(map: &WorldMapFrame, (scale, open): (f32, f32), cx: &LayoutCx<'_>, out: &mut Vec<Box<dyn Component>>) {
    let (Some(route), Some(calibration)) = (&map.route, map.calibration) else { return };
    let at = on_screen(map.session.camera, calibration.to_map(route.target));
    let tint = fade(colour(cx, token_keys::GUIDE), open);
    out.push(icon(CHECKPOINT_ICON, at, ICON * scale * TARGET_GAIN, (tint, 0.0), MARK_Z, cx));
}

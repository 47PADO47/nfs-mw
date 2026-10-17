//! The PC menus' mouse pointer: the game's own `PC_CURSOR` texture, read from the front-end packages' textures and set
//! as the window cursor. The arrow's tip is the hotspot. Without the texture the system pointer stays.

use bevy_asset::{Assets, Handle, RenderAssetUsages};
use bevy_ecs::prelude::*;
use bevy_image::Image;
use bevy_window::{CursorIcon, CustomCursor, CustomCursorImage, PrimaryWindow, Window};
use blackbox_feng::fe_hash_upper;
use glam::Vec2;
use wgpu::{Extent3d, TextureDimension, TextureFormat};

use crate::settings::Settings;
use crate::ui::present::FENG_HEIGHT;
use crate::ui::{SharedAssets, UiAssets};

/// The window's pointer in FEng units: the 480-unit height fills the window, the origin is the centre of the
/// screen and y points down. `None` while the pointer is outside the window.
pub fn feng_point(window: &Window) -> Option<Vec2> {
    let cursor = window.cursor_position()?;
    let scale = window.height() / FENG_HEIGHT;
    Some(Vec2::new((cursor.x - window.width() * 0.5) / scale, (cursor.y - window.height() * 0.5) / scale))
}

/// The texture's name in the install.
const POINTER: &str = "PC_CURSOR";

/// The arrow's tip, in pixels from the top left of the 16 by 16 texture.
const HOTSPOT: (u16, u16) = (2, 2);
/// The stock texture's size in pixels.
const STOCK_SIZE: (u32, u32) = (16, 16);

/// The texture as a Bevy image, `None` when the install has no such texture.
pub fn pointer_image(assets: &UiAssets) -> Option<Image> {
    let pixels = assets.image(fe_hash_upper(POINTER))?;
    let size = Extent3d { width: pixels.width, height: pixels.height, depth_or_array_layers: 1 };
    Some(Image::new(size, TextureDimension::D2, pixels.rgba, TextureFormat::Rgba8Unorm, RenderAssetUsages::default()))
}

/// The hotspot in pixels for a cursor texture of `size`. With the HD cursor option a texture larger than the stock 16 by
/// 16 keeps the tip on the same part of the art: the stock tip scaled by the size ratio, rounded, and kept inside the
/// texture. Stock content and the option off keep the stock tip. Rule: `docs/specs/hd-content-support.md`, section 3.
pub fn hotspot(size: (u32, u32), hd_cursor: bool) -> (u16, u16) {
    (axis(HOTSPOT.0, size.0, STOCK_SIZE.0, hd_cursor), axis(HOTSPOT.1, size.1, STOCK_SIZE.1, hd_cursor))
}

fn axis(tip: u16, len: u32, stock: u32, hd_cursor: bool) -> u16 {
    let ratio = match hd_cursor && len > stock {
        true => len as f32 / stock as f32,
        false => 1.0,
    };
    let at = (f32::from(tip) * ratio).round() as u32;
    at.min(len.saturating_sub(1)) as u16
}

/// The loaded pointer image and its size, kept once the texture is read.
type Loaded = (Handle<Image>, (u32, u32));

/// Sets the primary window's cursor to the game's pointer when the front end's assets are loaded, and sets it again
/// when the HD cursor option changes (the hotspot follows the texture size).
pub fn apply(
    mut commands: Commands,
    assets: Res<SharedAssets>,
    settings: Res<Settings>,
    mut images: ResMut<Assets<Image>>,
    windows: Query<Entity, With<PrimaryWindow>>,
    mut loaded: Local<Option<Loaded>>,
    mut applied_hd: Local<Option<bool>>,
) {
    let hd_cursor = settings.hd_content.cursor;
    if *applied_hd == Some(hd_cursor) {
        return;
    }
    let Ok(window) = windows.single() else { return };
    let (handle, size) = match loaded.clone() {
        Some(done) => done,
        None => {
            *applied_hd = Some(hd_cursor);
            let Some(image) = pointer_image(&assets.0) else {
                log::warn!("the install has no {POINTER} texture: the system pointer stays");
                return;
            };
            let size = (image.width(), image.height());
            let done = (images.add(image), size);
            *loaded = Some(done.clone());
            log::info!("the pointer is the game's {POINTER} texture");
            done
        }
    };
    *applied_hd = Some(hd_cursor);
    let cursor = CustomCursorImage { handle, hotspot: hotspot(size, hd_cursor), ..Default::default() };
    commands.entity(window).insert(CursorIcon::Custom(CustomCursor::Image(cursor)));
}

#[cfg(test)]
#[path = "pointer_tests.rs"]
mod tests;

//! The PC menus' mouse pointer: the game's own `PC_CURSOR` texture, read from the front-end packages' textures and set
//! as the window cursor. The arrow's tip is the hotspot. Without the texture the system pointer stays.

use bevy_asset::{Assets, RenderAssetUsages};
use bevy_ecs::prelude::*;
use bevy_image::Image;
use bevy_window::{CursorIcon, CustomCursor, CustomCursorImage, PrimaryWindow, Window};
use blackbox_feng::fe_hash_upper;
use glam::Vec2;
use wgpu::{Extent3d, TextureDimension, TextureFormat};

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

/// The texture as a Bevy image, `None` when the install has no such texture.
pub fn pointer_image(assets: &UiAssets) -> Option<Image> {
    let pixels = assets.image(fe_hash_upper(POINTER))?;
    let size = Extent3d { width: pixels.width, height: pixels.height, depth_or_array_layers: 1 };
    Some(Image::new(size, TextureDimension::D2, pixels.rgba, TextureFormat::Rgba8Unorm, RenderAssetUsages::default()))
}

/// Sets the primary window's cursor to the game's pointer, once, when the front end's assets are loaded.
pub fn apply(
    mut commands: Commands,
    assets: Res<SharedAssets>,
    mut images: ResMut<Assets<Image>>,
    windows: Query<Entity, (With<PrimaryWindow>, Without<CursorIcon>)>,
    mut done: Local<bool>,
) {
    if *done {
        return;
    }
    let Ok(window) = windows.single() else { return };
    *done = true;
    let Some(image) = pointer_image(&assets.0) else {
        log::warn!("the install has no {POINTER} texture: the system pointer stays");
        return;
    };
    let handle = images.add(image);
    let cursor = CustomCursorImage { handle, hotspot: HOTSPOT, ..Default::default() };
    commands.entity(window).insert(CursorIcon::Custom(CustomCursor::Image(cursor)));
    log::info!("the pointer is the game's {POINTER} texture");
}

#[cfg(test)]
#[path = "pointer_tests.rs"]
mod tests;

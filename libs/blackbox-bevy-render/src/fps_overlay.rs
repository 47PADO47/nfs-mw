//! The frame-rate overlay: Bevy's own `FpsOverlayPlugin` (text and frame-time graph), drawn on the primary window.
//!
//! The overlay is hidden until the app turns it on through [`FpsOverlayConfig`]. The plugin spawns no camera, so
//! this adds a 2D camera that draws the overlay's UI over the scene, which the scene camera leaves alone.

use bevy_app::{App, Plugin, Startup};
use bevy_camera::{Camera, Camera2d, ClearColorConfig};
use bevy_dev_tools::fps_overlay::{FpsOverlayConfig as DevConfig, FpsOverlayPlugin};
use bevy_ecs::prelude::*;
use bevy_image::TextureAtlasPlugin;
use bevy_input_focus::InputFocusPlugin;
use bevy_sprite_render::SpriteRenderPlugin;
use bevy_text::TextPlugin;
use bevy_ui::UiPlugin;
use bevy_ui_render::UiRenderPlugin;

pub use bevy_dev_tools::fps_overlay::{FpsOverlayConfig, FrameTimeGraphConfig};

/// Marks the camera that draws the overlay on top of the scene.
#[derive(Component)]
struct FpsOverlayCamera;

/// Bevy's frame-rate overlay, starting hidden. Set [`FpsOverlayConfig::enabled`] to show it.
pub struct BlackboxFpsOverlayPlugin;

impl Plugin for BlackboxFpsOverlayPlugin {
    fn build(&self, app: &mut App) {
        // The overlay is Bevy UI text; the render plugins are the ones `DefaultPlugins` would add.
        app.add_plugins((
            TextPlugin,
            TextureAtlasPlugin,
            InputFocusPlugin,
            SpriteRenderPlugin,
            UiPlugin,
            UiRenderPlugin,
        ))
        .add_plugins(FpsOverlayPlugin { config: DevConfig { enabled: false, ..Default::default() } })
        .add_systems(Startup, spawn_camera);
    }
}

fn spawn_camera(mut commands: Commands) {
    // Drawn after the scene (order 1), over it, without clearing what the scene drew.
    commands.spawn((
        Camera2d,
        Camera { order: 1, clear_color: ClearColorConfig::None, ..Default::default() },
        FpsOverlayCamera,
    ));
}

//! A window that runs one [`Scene`]: input, camera, streaming and drawing are
//! the scene's job; the viewer owns the window, the renderer and the loop.

mod app;
pub mod camera;
mod cursor;
mod input;
mod limiter;
mod screenshot;

use anyhow::Result;
use blackbox_render::{FrameParams, Instance, Renderer};

pub use input::Input;
pub use limiter::MaxFps;

use crate::cli::ViewArgs;

pub trait Scene {
    fn title(&self) -> String;
    /// Upload the initial resources.
    fn init(&mut self, renderer: &mut Renderer) -> Result<()>;
    /// Advance by `dt` seconds: camera, streaming, uploads.
    fn update(&mut self, renderer: &mut Renderer, input: &Input, dt: f32);
    /// The frame to draw. Instances of the same mesh should be adjacent.
    fn frame(&mut self, aspect: f32) -> (FrameParams, &[Instance]);
    /// For screenshots: whether everything the first view needs has loaded.
    fn ready(&self) -> bool {
        true
    }
    /// Mouse look: hide and hold the cursor so mouse motion always turns the camera.
    /// Esc releases the cursor and a click captures it again.
    fn captures_mouse(&self) -> bool {
        false
    }
    /// Extra text for the window title (streaming progress, counts).
    fn status(&self) -> Option<String> {
        None
    }
}

/// Open a window and run `scene` until the user quits (or write a screenshot and exit).
pub fn run(scene: Box<dyn Scene>, args: &ViewArgs) -> Result<()> {
    app::run(scene, args)
}

//! What a viewer scene is, and the cameras scenes share. The window, loop and renderer belong to
//! [`crate::app`]; input arrives as actions ([`crate::input`]).

pub mod camera;

use anyhow::Result;
use blackbox_render::{FrameParams, Instance, Renderer};

use crate::input::ActionState;

pub trait Scene {
    fn title(&self) -> String;
    /// Upload the initial resources.
    fn init(&mut self, renderer: &mut Renderer) -> Result<()>;
    /// Advance by `dt` seconds: camera, streaming, uploads.
    fn update(&mut self, renderer: &mut Renderer, input: &ActionState, dt: f32);
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

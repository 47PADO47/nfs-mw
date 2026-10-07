//! State the app's systems share. It is `NonSend`: the renderer and the scene stay on the main thread.

use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::Instant;

use bevy_ecs::resource::Resource;
use blackbox_render::Renderer;

use super::pacing::FrameLimiter;
use crate::settings::Settings;
use crate::viewer::Scene;

pub struct Host {
    pub scene: Box<dyn Scene>,
    pub screenshot: Option<PathBuf>,
    /// Created once the window exists.
    pub renderer: Option<Renderer>,
    /// The window size the renderer was last resized to.
    pub size: (u32, u32),
    pub limiter: FrameLimiter,
    pub title_timer: Instant,
    pub frames: u32,
}

impl Host {
    pub fn new(scene: Box<dyn Scene>, settings: &Settings, screenshot: Option<PathBuf>) -> Self {
        Self {
            scene,
            screenshot,
            renderer: None,
            size: (0, 0),
            limiter: FrameLimiter::new(settings.max_fps),
            title_timer: Instant::now(),
            frames: 0,
        }
    }
}

/// Where a failing system leaves its error for [`super::run`] to return.
#[derive(Resource, Clone)]
pub struct ErrorSlot(pub Arc<Mutex<Option<anyhow::Error>>>);

impl ErrorSlot {
    pub fn set(&self, error: anyhow::Error) {
        *self.0.lock().unwrap_or_else(|e| e.into_inner()) = Some(error);
    }

    pub fn take(&self) -> Option<anyhow::Error> {
        self.0.lock().unwrap_or_else(|e| e.into_inner()).take()
    }
}

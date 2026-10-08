//! State the app's systems share. It is `NonSend`: the renderer and the scene stay on the main thread.

use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::Instant;

use bevy_ecs::resource::Resource;
use blackbox_render::Renderer;

use super::pacing::FrameLimiter;
use crate::settings::{Settings, Transmission};
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
    /// The front end uses Escape (pause, back), so Escape does not release the mouse or quit.
    pub cancel_handled: bool,
    /// The front end decides when a scene is over (a finished movie is not the end of the program).
    pub flow_driven: bool,
    /// A screenshot run waits while this is set (a scripted menu is still running).
    pub hold_capture: bool,
    /// The transmission setting as of the last frame, given to every scene that is put in the window.
    pub transmission: Transmission,
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
            cancel_handled: false,
            flow_driven: false,
            hold_capture: false,
            transmission: settings.transmission,
        }
    }

    /// Puts another scene in the window: it is initialised with the renderer, and in a screenshot run it is given
    /// the time to load.
    pub fn replace_scene(&mut self, mut scene: Box<dyn Scene>) -> anyhow::Result<()> {
        let renderer = self.renderer.as_mut().ok_or_else(|| anyhow::anyhow!("the renderer is not ready"))?;
        scene.init(renderer)?;
        scene.set_transmission(self.transmission);
        if self.screenshot.is_some() {
            super::screenshot::wait_ready(scene.as_mut(), renderer);
        }
        self.scene = scene;
        Ok(())
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

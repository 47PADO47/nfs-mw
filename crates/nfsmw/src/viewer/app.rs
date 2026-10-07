//! The winit application: window, renderer, event loop.

use std::sync::Arc;
use std::time::Instant;

use anyhow::{Context, Result, anyhow};
use blackbox_render::{Renderer, RendererOptions};
use winit::application::ApplicationHandler;
use winit::event::{DeviceEvent, DeviceId, ElementState, WindowEvent};
use winit::event_loop::{ActiveEventLoop, EventLoop};
use winit::keyboard::{KeyCode, PhysicalKey};
use winit::window::{Window, WindowId};

use super::{Input, Scene, screenshot};
use crate::cli::ViewArgs;

struct Running {
    window: Arc<Window>,
    renderer: Renderer,
    last_frame: Instant,
    title_timer: Instant,
    frames: u32,
}

struct App {
    scene: Box<dyn Scene>,
    args: ViewArgs,
    input: Input,
    running: Option<Running>,
    error: Option<anyhow::Error>,
}

impl App {
    fn start(&mut self, event_loop: &ActiveEventLoop) -> Result<()> {
        let mut attrs = Window::default_attributes().with_title(self.scene.title());
        attrs = if self.args.screenshot.is_some() {
            let (w, h) = screenshot::SIZE;
            attrs.with_visible(false).with_inner_size(winit::dpi::PhysicalSize::new(w, h))
        } else {
            attrs.with_inner_size(winit::dpi::LogicalSize::new(1280.0, 720.0))
        };
        let window = Arc::new(event_loop.create_window(attrs).context("creating the window")?);
        let options = RendererOptions { backend: self.args.backend, vsync: !self.args.no_vsync };
        let mut renderer = Renderer::new(window.clone(), event_loop.owned_display_handle(), options)?;
        log::info!("renderer: {} (requested backend: {})", renderer.adapter_summary(), self.args.backend);
        self.scene.init(&mut renderer)?;

        if let Some(path) = self.args.screenshot.clone() {
            screenshot::capture(self.scene.as_mut(), &mut renderer, &path)?;
            event_loop.exit();
            return Ok(());
        }
        window.request_redraw();
        let now = Instant::now();
        self.running = Some(Running { window, renderer, last_frame: now, title_timer: now, frames: 0 });
        Ok(())
    }

    fn redraw(&mut self) -> Result<()> {
        let Some(r) = self.running.as_mut() else { return Ok(()) };
        let now = Instant::now();
        let dt = (now - r.last_frame).as_secs_f32().min(0.1);
        r.last_frame = now;
        self.scene.update(&mut r.renderer, &self.input, dt);
        self.input.end_frame();
        let (params, instances) = self.scene.frame(r.renderer.aspect_ratio());
        r.renderer.render(&params, instances)?;
        r.frames += 1;
        let elapsed = r.title_timer.elapsed().as_secs_f32();
        if elapsed >= 1.0 {
            let status = self.scene.status().map(|s| format!(" — {s}")).unwrap_or_default();
            let fps = r.frames as f32 / elapsed;
            r.window.set_title(&format!(
                "{} — {} — {fps:.0} fps{status}",
                self.scene.title(),
                r.renderer.adapter_summary()
            ));
            (r.title_timer, r.frames) = (Instant::now(), 0);
        }
        r.window.request_redraw();
        Ok(())
    }

    fn fail(&mut self, event_loop: &ActiveEventLoop, e: anyhow::Error) {
        self.error = Some(e);
        event_loop.exit();
    }
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.running.is_none()
            && self.error.is_none()
            && let Err(e) = self.start(event_loop)
        {
            self.fail(event_loop, e);
        }
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::KeyboardInput { event, .. } => {
                if let PhysicalKey::Code(code) = event.physical_key {
                    if code == KeyCode::Escape && event.state == ElementState::Pressed {
                        event_loop.exit();
                    }
                    self.input.on_key(code, event.state);
                }
            }
            WindowEvent::MouseInput { state, button, .. } => self.input.on_button(button, state),
            WindowEvent::MouseWheel { delta, .. } => self.input.on_scroll(delta),
            WindowEvent::Focused(false) => self.input.release_all(),
            WindowEvent::Resized(size) => {
                if let Some(r) = self.running.as_mut() {
                    r.renderer.resize(size.width, size.height);
                }
            }
            WindowEvent::RedrawRequested => {
                if let Err(e) = self.redraw() {
                    self.fail(event_loop, e);
                }
            }
            _ => {}
        }
    }

    fn device_event(&mut self, _event_loop: &ActiveEventLoop, _id: DeviceId, event: DeviceEvent) {
        if let DeviceEvent::MouseMotion { delta } = event {
            self.input.on_motion(delta.0, delta.1);
        }
    }
}

pub fn run(scene: Box<dyn Scene>, args: &ViewArgs) -> Result<()> {
    let event_loop = EventLoop::new().context("creating the event loop")?;
    let mut app = App { scene, args: args.clone(), input: Input::default(), running: None, error: None };
    event_loop.run_app(&mut app).map_err(|e| anyhow!("event loop: {e}"))?;
    app.error.map_or(Ok(()), Err)
}

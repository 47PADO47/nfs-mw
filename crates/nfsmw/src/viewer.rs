//! A window showing one car with an orbit camera.

use std::path::PathBuf;
use std::sync::Arc;

use anyhow::{Context, Result, anyhow};
use glam::{Mat4, Vec3};
use nfsmw_render::{
    Backend, DrawRange, FrameParams, MeshDesc, MeshHandle, PixelFormat, Renderer, RendererOptions, TextureDesc,
    TextureHandle,
};
use nfsmw_texture::{PixelFormat as TpkFormat, Texture};
use winit::application::ApplicationHandler;
use winit::event::{ElementState, MouseButton, MouseScrollDelta, WindowEvent};
use winit::event_loop::{ActiveEventLoop, EventLoop};
use winit::keyboard::{Key, NamedKey};
use winit::window::{Window, WindowId};

use crate::car::{CarModel, blend_mode};

pub struct Options {
    pub model: CarModel,
    pub backend: Backend,
    pub vsync: bool,
    pub screenshot: Option<PathBuf>,
    pub yaw_degrees: f32,
}

const SCREENSHOT_SIZE: (u32, u32) = (1280, 720);

/// Orbit camera around the model. The game's world is Z-up.
struct OrbitCamera {
    target: Vec3,
    distance: f32,
    yaw: f32,
    pitch: f32,
}

impl OrbitCamera {
    fn view_proj(&self, aspect: f32) -> Mat4 {
        let dir = Vec3::new(self.yaw.cos() * self.pitch.cos(), self.yaw.sin() * self.pitch.cos(), self.pitch.sin());
        let eye = self.target + dir * self.distance;
        let near = (self.distance * 0.01).max(0.01);
        // wgpu's clip space is D3D-style: Y up, depth in [0, 1].
        glam::camera::rh::proj::directx::perspective(55f32.to_radians(), aspect, near, self.distance * 10.0)
            * glam::camera::rh::view::look_at_mat4(eye, self.target, Vec3::Z)
    }
}

struct Scene {
    window: Arc<Window>,
    renderer: Renderer,
    meshes: Vec<MeshHandle>,
    camera: OrbitCamera,
    dragging: bool,
    last_cursor: Option<(f64, f64)>,
}

struct App {
    options: Options,
    scene: Option<Scene>,
    error: Option<anyhow::Error>,
}

fn texture_desc<'a>(t: &'a Texture, supports_bc: bool, rgba: &'a mut Vec<u8>) -> Option<TextureDesc<'a>> {
    let bc = match t.format {
        TpkFormat::Dxt1 => Some(PixelFormat::Bc1),
        TpkFormat::Dxt3 => Some(PixelFormat::Bc2),
        TpkFormat::Dxt5 => Some(PixelFormat::Bc3),
        _ => None,
    };
    match bc {
        // wgpu needs block-compressed textures to be a whole number of blocks.
        Some(format) if supports_bc && t.width.is_multiple_of(4) && t.height.is_multiple_of(4) => {
            let mut mips = Vec::new();
            let mut offset = 0;
            for level in 0..t.mip_levels {
                let size = nfsmw_texture::mip_level_size(t.format, t.width, t.height, level);
                mips.push(t.data.get(offset..offset + size)?);
                offset += size;
            }
            Some(TextureDesc { label: &t.name, width: t.width, height: t.height, format, mips })
        }
        _ => {
            *rgba = nfsmw_texture::decode_rgba8(t)?;
            Some(TextureDesc {
                label: &t.name,
                width: t.width,
                height: t.height,
                format: PixelFormat::Rgba8,
                mips: vec![rgba],
            })
        }
    }
}

fn upload(renderer: &mut Renderer, model: &CarModel) -> Vec<MeshHandle> {
    let mut handles: std::collections::HashMap<u32, TextureHandle> = std::collections::HashMap::new();
    for (&hash, t) in &model.textures {
        let mut rgba = Vec::new();
        match texture_desc(t, renderer.supports_bc(), &mut rgba) {
            Some(desc) => {
                handles.insert(hash, renderer.create_texture(&desc));
            }
            None => log::warn!("{}: {:?} {}x{} is not supported yet", t.name, t.format, t.width, t.height),
        }
    }
    model
        .solids
        .iter()
        .filter(|s| !s.vertices.is_empty())
        .map(|s| {
            let vertices: Vec<nfsmw_render::Vertex> = s
                .vertices
                .iter()
                .map(|v| nfsmw_render::Vertex {
                    position: v.position,
                    normal: v.normal,
                    color_bgra: v.color_bgra,
                    uv: v.uv,
                })
                .collect();
            let draws = s
                .groups
                .iter()
                .map(|g| {
                    let hash = g.diffuse_texture(s);
                    DrawRange {
                        first_index: g.first_index,
                        index_count: g.num_indices,
                        texture: hash.and_then(|h| handles.get(&h).copied()),
                        blend: blend_mode(hash.and_then(|h| model.textures.get(&h))),
                    }
                })
                .collect();
            renderer.create_mesh(&MeshDesc { label: &s.name, vertices: &vertices, indices: &s.indices, draws })
        })
        .collect()
}

impl App {
    fn frame_params(&self, scene: &Scene, aspect: f32) -> FrameParams {
        let _ = self;
        FrameParams {
            view_proj: scene.camera.view_proj(aspect),
            light_dir: Vec3::new(-0.4, -0.3, -1.0),
            clear_color: [0.18, 0.2, 0.24],
        }
    }

    fn start(&mut self, event_loop: &ActiveEventLoop) -> Result<()> {
        let o = &self.options;
        let title = format!("nfsmw — {}", o.model.name);
        let mut attrs = Window::default_attributes().with_title(title.clone());
        attrs = if o.screenshot.is_some() {
            attrs
                .with_visible(false)
                .with_inner_size(winit::dpi::PhysicalSize::new(SCREENSHOT_SIZE.0, SCREENSHOT_SIZE.1))
        } else {
            attrs.with_inner_size(winit::dpi::LogicalSize::new(1280.0, 720.0))
        };
        let window = Arc::new(event_loop.create_window(attrs).context("creating the window")?);
        let mut renderer = Renderer::new(
            window.clone(),
            event_loop.owned_display_handle(),
            RendererOptions { backend: o.backend, vsync: o.vsync },
        )?;
        log::info!("renderer: {} (requested backend: {})", renderer.adapter_summary(), o.backend);
        window.set_title(&format!("{title} — {}", renderer.adapter_summary()));
        let meshes = upload(&mut renderer, &o.model);

        let center = (o.model.bounds_min + o.model.bounds_max) * 0.5;
        let radius = (o.model.bounds_max - o.model.bounds_min).length() * 0.5;
        let camera =
            OrbitCamera { target: center, distance: radius * 2.2, yaw: o.yaw_degrees.to_radians(), pitch: 0.35 };
        let scene = Scene { window, renderer, meshes, camera, dragging: false, last_cursor: None };

        if let Some(path) = &o.screenshot {
            let (w, h) = SCREENSHOT_SIZE;
            let mut scene = scene;
            let params = self.frame_params(&scene, w as f32 / h as f32);
            let pixels = scene.renderer.capture(w, h, &params, &scene.meshes)?;
            save_png(path, w, h, &pixels)?;
            println!("wrote {}", path.display());
            event_loop.exit();
            return Ok(());
        }
        scene.window.request_redraw();
        self.scene = Some(scene);
        Ok(())
    }
}

fn save_png(path: &std::path::Path, width: u32, height: u32, rgba: &[u8]) -> Result<()> {
    let file = std::fs::File::create(path).with_context(|| format!("creating {}", path.display()))?;
    let mut encoder = png::Encoder::new(std::io::BufWriter::new(file), width, height);
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    encoder.write_header()?.write_image_data(rgba)?;
    Ok(())
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.scene.is_none()
            && self.error.is_none()
            && let Err(e) = self.start(event_loop)
        {
            self.error = Some(e);
            event_loop.exit();
        }
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        let Some(scene) = self.scene.as_mut() else { return };
        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::KeyboardInput { event, .. }
                if event.state == ElementState::Pressed && event.logical_key == Key::Named(NamedKey::Escape) =>
            {
                event_loop.exit();
            }
            WindowEvent::Resized(size) => scene.renderer.resize(size.width, size.height),
            WindowEvent::MouseInput { state, button: MouseButton::Left, .. } => {
                scene.dragging = state == ElementState::Pressed;
            }
            WindowEvent::CursorMoved { position, .. } => {
                if let (true, Some((x, y))) = (scene.dragging, scene.last_cursor) {
                    scene.camera.yaw -= (position.x - x) as f32 * 0.008;
                    scene.camera.pitch = (scene.camera.pitch + (position.y - y) as f32 * 0.008).clamp(-1.5, 1.5);
                }
                scene.last_cursor = Some((position.x, position.y));
            }
            WindowEvent::MouseWheel { delta, .. } => {
                let steps = match delta {
                    MouseScrollDelta::LineDelta(_, y) => y,
                    MouseScrollDelta::PixelDelta(p) => p.y as f32 / 60.0,
                };
                scene.camera.distance = (scene.camera.distance * 0.9f32.powf(steps)).max(0.1);
            }
            WindowEvent::RedrawRequested => {
                let aspect = scene.renderer.aspect_ratio();
                let params = FrameParams {
                    view_proj: scene.camera.view_proj(aspect),
                    light_dir: Vec3::new(-0.4, -0.3, -1.0),
                    clear_color: [0.18, 0.2, 0.24],
                };
                if let Err(e) = scene.renderer.render(&params, &scene.meshes) {
                    self.error = Some(e.into());
                    event_loop.exit();
                    return;
                }
                scene.window.request_redraw();
            }
            _ => {}
        }
    }
}

pub fn run(options: Options) -> Result<()> {
    let event_loop = EventLoop::new().context("creating the event loop")?;
    let mut app = App { options, scene: None, error: None };
    event_loop.run_app(&mut app).map_err(|e| anyhow!("event loop: {e}"))?;
    match app.error {
        Some(e) => Err(e),
        None => Ok(()),
    }
}

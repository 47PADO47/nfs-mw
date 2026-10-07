//! `--screenshot`: render one frame off-screen and write it as PNG.

use std::path::Path;
use std::time::{Duration, Instant};

use anyhow::{Context, Result};
use blackbox_render::Renderer;

use crate::input::ActionState;
use crate::viewer::Scene;

pub const SIZE: (u32, u32) = (1280, 720);
/// How long to wait for a scene to report [`Scene::ready`].
const READY_TIMEOUT: Duration = Duration::from_secs(300);

pub fn capture(scene: &mut dyn Scene, renderer: &mut Renderer, path: &Path) -> Result<()> {
    let input = ActionState::default();
    let start = Instant::now();
    // Let the scene stream until its first view is complete.
    loop {
        scene.update(renderer, &input, 0.0);
        if scene.ready() || start.elapsed() > READY_TIMEOUT {
            break;
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    if !scene.ready() {
        log::warn!("scene not fully loaded after {READY_TIMEOUT:?}; capturing anyway");
    }
    let (w, h) = SIZE;
    let (params, instances) = scene.frame(w as f32 / h as f32);
    let pixels = renderer.capture(w, h, &params, instances)?;
    save_png(path, w, h, &pixels)?;
    println!("wrote {} ({:.1}s)", path.display(), start.elapsed().as_secs_f32());
    if let Some(status) = scene.status() {
        println!("{status}");
    }
    Ok(())
}

fn save_png(path: &Path, width: u32, height: u32, rgba: &[u8]) -> Result<()> {
    let file = std::fs::File::create(path).with_context(|| format!("creating {}", path.display()))?;
    let mut encoder = png::Encoder::new(std::io::BufWriter::new(file), width, height);
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    encoder.write_header()?.write_image_data(rgba)?;
    Ok(())
}

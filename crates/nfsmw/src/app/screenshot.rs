//! `--screenshot`: wait for the scene to load, then render one frame off-screen and write it as PNG.

use std::path::Path;
use std::time::{Duration, Instant};

use anyhow::{Context, Result};
use blackbox_render::{FrameParams, Instance, Renderer};

use crate::input::ActionState;
use crate::viewer::Scene;

pub const SIZE: (u32, u32) = (1280, 720);
/// How long to wait for a scene to report [`Scene::ready`].
const READY_TIMEOUT: Duration = Duration::from_secs(300);

/// Let the scene stream until its first view is complete (or give up after [`READY_TIMEOUT`]).
pub fn wait_ready(scene: &mut dyn Scene, renderer: &mut Renderer) {
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
}

/// Render `instances` (and the UI layer) off-screen and write the PNG.
pub fn capture(
    renderer: &mut Renderer,
    params: &FrameParams,
    instances: &[Instance],
    (w, h): (u32, u32),
    path: &Path,
) -> Result<()> {
    let pixels = renderer.capture(w, h, params, instances)?;
    save_png(path, w, h, &pixels)?;
    println!("wrote {}", path.display());
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

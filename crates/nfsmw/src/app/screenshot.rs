//! `--screenshot`: wait for the scene to load, then ask the renderer for one off-screen frame and write it
//! as PNG once it is ready.

use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use anyhow::{Context, Result};
use blackbox_gfx::{CaptureId, FrameParams, Instance, RenderBackend};

use crate::input::ActionState;
use crate::viewer::Scene;

pub const SIZE: (u32, u32) = (1280, 720);
/// How long to wait for a scene to report [`Scene::ready`].
const READY_TIMEOUT: Duration = Duration::from_secs(300);

/// Let the scene stream until its first view is complete (or give up after [`READY_TIMEOUT`]).
pub fn wait_ready(scene: &mut dyn Scene, renderer: &mut dyn RenderBackend) {
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

/// Ask for `instances` (and the UI layer) to be rendered off-screen at `size`.
pub fn request(
    renderer: &mut dyn RenderBackend,
    params: &FrameParams,
    instances: &[Instance],
    size: (u32, u32),
) -> Result<CaptureId> {
    Ok(renderer.request_capture([size.0, size.1], params, instances)?)
}

/// Check on a requested capture: once the image is ready, write the PNG and return `true`. A renderer may take
/// a few frames, so this is called every frame until it is done.
pub fn poll(renderer: &mut dyn RenderBackend, id: CaptureId, path: &Path) -> Result<bool> {
    let Some(image) = renderer.poll_capture(id) else { return Ok(false) };
    let image = image?;
    save_png(path, image.width, image.height, &image.rgba)?;
    println!("wrote {}", path.display());
    Ok(true)
}

fn save_png(path: &Path, width: u32, height: u32, rgba: &[u8]) -> Result<()> {
    let file = std::fs::File::create(path).with_context(|| format!("creating {}", path.display()))?;
    let mut encoder = png::Encoder::new(std::io::BufWriter::new(file), width, height);
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    encoder.write_header()?.write_image_data(rgba)?;
    Ok(())
}

/// When the captures of a screenshot run are taken: the first after `delay` seconds (counted from the scene being
/// ready), then the others `interval` seconds apart, all in wall-clock time. With more than one capture the files
/// are numbered: `out.png` becomes `out-1.png`, `out-2.png`, and so on.
pub struct Plan {
    path: PathBuf,
    delay: Duration,
    interval: Duration,
    count: u32,
    taken: u32,
    started: Option<Instant>,
    next: Option<Instant>,
}

impl Plan {
    pub fn new(path: PathBuf, delay_secs: f32, count: u32, interval_secs: f32) -> Self {
        Self {
            path,
            delay: Duration::from_secs_f32(delay_secs.max(0.0)),
            interval: Duration::from_secs_f32(interval_secs.max(0.0)),
            count: count.max(1),
            taken: 0,
            started: None,
            next: None,
        }
    }

    /// The file to write for the capture that is due at `now`; `None` while the next one is still to wait for.
    pub fn due(&mut self, now: Instant) -> Option<PathBuf> {
        if self.finished() {
            return None;
        }
        let started = *self.started.get_or_insert(now);
        let at = self.next.unwrap_or(started + self.delay);
        if now < at {
            return None;
        }
        self.next = Some(now + self.interval);
        let path = numbered(&self.path, self.taken, self.count);
        self.taken += 1;
        Some(path)
    }

    /// Every capture has been taken.
    pub fn finished(&self) -> bool {
        self.taken >= self.count
    }
}

/// The file of capture `index` (from 0) out of `count`.
fn numbered(path: &Path, index: u32, count: u32) -> PathBuf {
    if count <= 1 {
        return path.to_path_buf();
    }
    let stem = path.file_stem().unwrap_or_default().to_string_lossy();
    let name = match path.extension() {
        Some(ext) => format!("{stem}-{}.{}", index + 1, ext.to_string_lossy()),
        None => format!("{stem}-{}", index + 1),
    };
    path.with_file_name(name)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn one_capture_keeps_the_name_and_several_are_numbered() {
        assert_eq!(numbered(Path::new("out.png"), 0, 1), PathBuf::from("out.png"));
        assert_eq!(numbered(Path::new("shots/out.png"), 1, 3), PathBuf::from("shots/out-2.png"));
        assert_eq!(numbered(Path::new("out"), 0, 2), PathBuf::from("out-1"));
    }

    #[test]
    fn the_first_capture_waits_for_the_delay_then_the_others_wait_for_the_interval() {
        let t0 = Instant::now();
        let mut plan = Plan::new(PathBuf::from("a.png"), 2.0, 3, 0.5);
        assert_eq!(plan.due(t0), None);
        assert_eq!(plan.due(t0 + Duration::from_secs(1)), None);
        assert_eq!(plan.due(t0 + Duration::from_secs(2)), Some(PathBuf::from("a-1.png")));
        assert_eq!(plan.due(t0 + Duration::from_millis(2400)), None);
        assert_eq!(plan.due(t0 + Duration::from_millis(2500)), Some(PathBuf::from("a-2.png")));
        assert!(!plan.finished());
        assert_eq!(plan.due(t0 + Duration::from_secs(3)), Some(PathBuf::from("a-3.png")));
        assert!(plan.finished());
        assert_eq!(plan.due(t0 + Duration::from_secs(9)), None);
    }
}

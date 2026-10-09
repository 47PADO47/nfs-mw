//! Capturing a frame to memory without a window.
//!
//! A capture gets its own camera that renders into an image, and a one-shot GPU readback of that image. The
//! camera draws a few frames first, so pipelines, materials and meshes are all ready, then the readback is
//! queued; Bevy answers one or two frames later through an observer, which unpads the rows and files the
//! picture where `poll_capture` looks.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use bevy_asset::{Assets, Handle};
use bevy_camera::RenderTarget;
use bevy_ecs::entity::Entity;
use bevy_ecs::observer::On;
use bevy_ecs::system::Commands;
use bevy_image::Image;
use bevy_render::gpu_readback::{ReadbackComplete, ReadbackOnce};
use blackbox_gfx::{CaptureId, RenderError, RgbaImage};
use wgpu::{TextureFormat, TextureUsages};

use super::camera;
use crate::ops::{CameraSettings, CaptureRequest, FrameData, Shared, lock};

/// Frames the capture camera draws before the picture is read back.
const WARMUP_FRAMES: u32 = 3;

/// The format of capture images: the sRGB format a window would use, so a capture looks like the screen.
const FORMAT: TextureFormat = TextureFormat::Rgba8UnormSrgb;

pub struct ActiveCapture {
    pub id: CaptureId,
    pub camera: Entity,
    /// Keeps the render target alive.
    pub image: Handle<Image>,
    pub frames: u32,
    pub requested: bool,
    /// Set by the readback observer once the result is filed (the caller may take it before the next frame).
    pub done: Arc<AtomicBool>,
    pub data: FrameData,
}

/// Start a capture: the image and its camera.
pub fn begin(
    commands: &mut Commands,
    images: &mut Assets<Image>,
    request: CaptureRequest,
    settings: &CameraSettings,
) -> ActiveCapture {
    let CaptureRequest { id, size, data } = request;
    let mut target = Image::new_target_texture(size[0], size[1], FORMAT, None);
    target.texture_descriptor.usage |= TextureUsages::COPY_SRC;
    let image = images.add(target);
    let bundle = camera::bundle(&data.frame, RenderTarget::Image(image.clone().into()), settings);
    let camera = commands.spawn(bundle).id();
    ActiveCapture { id, camera, image, frames: 0, requested: false, done: Arc::default(), data }
}

/// One more frame of an active capture. Returns `true` once its result was filed and its camera removed.
pub fn advance(
    commands: &mut Commands,
    active: &mut ActiveCapture,
    shared: &Arc<Mutex<Shared>>,
    size: [u32; 2],
) -> bool {
    if active.done.load(Ordering::Acquire) {
        commands.entity(active.camera).despawn();
        return true;
    }
    active.frames += 1;
    if active.requested || active.frames < WARMUP_FRAMES {
        return false;
    }
    active.requested = true;
    let (shared, id, done) = (shared.clone(), active.id, active.done.clone());
    commands.spawn(ReadbackOnce::texture(active.image.clone())).observe(move |read: On<ReadbackComplete>| {
        let result = unpad(&read.data, size);
        lock(&shared).finished.insert(id.raw(), result);
        done.store(true, Ordering::Release);
    });
    false
}

/// The packed RGBA picture from a readback whose rows are padded to wgpu's copy alignment.
fn unpad(data: &[u8], [width, height]: [u32; 2]) -> Result<RgbaImage, RenderError> {
    let row = (width as usize) * 4;
    let stride = row.next_multiple_of(wgpu::COPY_BYTES_PER_ROW_ALIGNMENT as usize);
    if data.len() < stride * height as usize {
        return Err(RenderError::Device(format!(
            "the readback of a {width}x{height} capture returned {} bytes",
            data.len()
        )));
    }
    let mut rgba = Vec::with_capacity(row * height as usize);
    for y in 0..height as usize {
        rgba.extend_from_slice(&data[y * stride..y * stride + row]);
    }
    RgbaImage::new(width, height, rgba)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn padded_rows_are_packed() {
        // 3 pixels wide: 12 bytes per row, padded to 256.
        let mut data = vec![0u8; 256 * 2];
        data[..12].copy_from_slice(&[1; 12]);
        data[256..268].copy_from_slice(&[2; 12]);
        let image = unpad(&data, [3, 2]).unwrap();
        assert_eq!(image.rgba.len(), 24);
        assert_eq!(&image.rgba[..12], &[1; 12]);
        assert_eq!(&image.rgba[12..], &[2; 12]);
    }

    #[test]
    fn unpadded_rows_pass_through() {
        let data: Vec<u8> = (0..64u32).flat_map(|i| [i as u8; 4]).collect();
        // 64 pixels wide is 256 bytes: already aligned.
        let image = unpad(&data, [64, 1]).unwrap();
        assert_eq!(image.rgba, data);
    }

    #[test]
    fn a_short_readback_is_an_error() {
        assert!(unpad(&[0; 100], [3, 2]).is_err());
    }
}

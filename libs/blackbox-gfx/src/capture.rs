//! Reading a rendered frame back: screenshots and tests.

use crate::{RenderError, UiTextureId};

/// A tightly packed RGBA8 image, top row first.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RgbaImage {
    pub width: u32,
    pub height: u32,
    /// `width * height * 4` bytes.
    pub rgba: Vec<u8>,
}

impl RgbaImage {
    /// Wrap `rgba`, which must be exactly `width * height * 4` bytes.
    pub fn new(width: u32, height: u32, rgba: Vec<u8>) -> Result<Self, RenderError> {
        let expected = u64::from(width) * u64::from(height) * 4;
        if rgba.len() as u64 != expected {
            return Err(RenderError::Device(format!(
                "a {width}x{height} image needs {expected} bytes, got {}",
                rgba.len()
            )));
        }
        Ok(Self { width, height, rgba })
    }

    /// The pixel at (`x`, `y`), or `None` outside the image.
    pub fn pixel(&self, x: u32, y: u32) -> Option<[u8; 4]> {
        if x >= self.width || y >= self.height {
            return None;
        }
        let at = (y as usize * self.width as usize + x as usize) * 4;
        self.rgba[at..at + 4].try_into().ok()
    }
}

/// Where a view capture goes when it is shown as a UI texture without a readback
/// ([`RenderBackend::request_view_capture_ui`](crate::RenderBackend::request_view_capture_ui)), and how it is
/// shaded on the way.
///
/// Each pixel of the capture becomes, in `texture`, the colour multiplied by `tint` (255 leaves a channel as it
/// is) and then by the alpha of `mask` at the same pixel; the alpha of the result is the mask's. The colour is
/// premultiplied, like the rest of the UI. Rounding is to the nearest 1/255 at each of the two products.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ViewCaptureUi {
    /// The UI texture that receives the picture. It is created or resized to the capture's size.
    pub texture: UiTextureId,
    /// A UI texture of exactly the capture's size whose alpha is the coverage. It must exist already.
    pub mask: UiTextureId,
    /// Per-channel colour multiplier, 0 to 255.
    pub tint: [u8; 3],
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_buffer_must_match_the_size() {
        assert!(RgbaImage::new(2, 2, vec![0; 16]).is_ok());
        assert!(RgbaImage::new(2, 2, vec![0; 15]).is_err());
        assert!(RgbaImage::new(0, 5, Vec::new()).is_ok());
    }

    #[test]
    fn pixels_are_read_row_major_and_bounds_checked() {
        let rgba: Vec<u8> = (0..16).collect();
        let image = RgbaImage::new(2, 2, rgba).unwrap();
        assert_eq!(image.pixel(0, 0), Some([0, 1, 2, 3]));
        assert_eq!(image.pixel(1, 0), Some([4, 5, 6, 7]));
        assert_eq!(image.pixel(0, 1), Some([8, 9, 10, 11]));
        assert_eq!(image.pixel(2, 0), None);
        assert_eq!(image.pixel(0, 2), None);
    }
}

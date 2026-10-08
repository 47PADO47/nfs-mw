//! Decoded pictures and the YUV to RGBA converter.

/// A decoded picture in planar YUV 4:2:0, rows packed without padding.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct YuvFrame {
    /// Width in pixels (luma).
    pub width: usize,
    /// Height in pixels (luma).
    pub height: usize,
    /// Luma plane, `width * height` bytes.
    pub y: Vec<u8>,
    /// Blue-difference chroma plane, `chroma_width() * chroma_height()` bytes.
    pub u: Vec<u8>,
    /// Red-difference chroma plane, same size as `u`.
    pub v: Vec<u8>,
}

impl YuvFrame {
    /// Width of the chroma planes.
    pub fn chroma_width(&self) -> usize {
        self.width.div_ceil(2)
    }

    /// Height of the chroma planes.
    pub fn chroma_height(&self) -> usize {
        self.height.div_ceil(2)
    }

    /// Size in bytes of the RGBA image from [`YuvFrame::to_rgba`].
    pub fn rgba_len(&self) -> usize {
        self.width * self.height * 4
    }

    /// Converts to RGBA8, row-major, alpha 255. Fills `out`, which must hold [`YuvFrame::rgba_len`]
    /// bytes (extra bytes are left alone). Uses BT.601 coefficients with studio range (luma 16 to
    /// 235, chroma 16 to 240), as VP6 content is mastered.
    pub fn to_rgba(&self, out: &mut [u8]) {
        let (cw, ch) = (self.chroma_width(), self.chroma_height());
        let complete = self.width > 0
            && self.y.len() >= self.width * self.height
            && self.u.len() >= cw * ch
            && self.v.len() >= cw * ch
            && out.len() >= self.rgba_len();
        if !complete {
            return;
        }
        for (row, out_row) in out.chunks_exact_mut(self.width * 4).take(self.height).enumerate() {
            let luma = &self.y[row * self.width..][..self.width];
            let u_row = &self.u[(row / 2) * cw..][..cw];
            let v_row = &self.v[(row / 2) * cw..][..cw];
            let (pixels, _) = out_row.as_chunks_mut::<4>();
            for (x, (px, &y)) in pixels.iter_mut().zip(luma).enumerate() {
                let rgb = yuv_to_rgb(y, u_row[x / 2], v_row[x / 2]);
                px[..3].copy_from_slice(&rgb);
                px[3] = 255;
            }
        }
    }

    /// Converts to a freshly allocated RGBA8 image.
    pub fn to_rgba_vec(&self) -> Vec<u8> {
        let mut out = vec![0u8; self.rgba_len()];
        self.to_rgba(&mut out);
        out
    }
}

/// BT.601 studio-range YUV to RGB in 16.16 fixed point: 1.164, 1.596, 0.391, 0.813 and 2.018.
fn yuv_to_rgb(y: u8, u: u8, v: u8) -> [u8; 3] {
    let c = (i32::from(y) - 16) * 76_309; // 255 / 219
    let d = i32::from(u) - 128;
    let e = i32::from(v) - 128;
    let r = c + 104_597 * e;
    let g = c - 25_675 * d - 53_279 * e;
    let b = c + 132_201 * d;
    [clamp8(r), clamp8(g), clamp8(b)]
}

fn clamp8(value: i32) -> u8 {
    ((value + 32_768) >> 16).clamp(0, 255) as u8
}

//! Procedural textures with a mip chain, and uploading them through a backend.

use blackbox_gfx::{PixelFormat, RenderBackend, TextureDesc, TextureHandle};

use crate::bc1;

/// The smallest mip edge: BC1 works in 4x4 blocks, so both formats stop here and look alike.
const SMALLEST_MIP: u32 = 4;

/// An RGBA8 image with its mip chain (largest first, down to 4x4).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Image {
    pub width: u32,
    pub height: u32,
    pub mips: Vec<Vec<u8>>,
}

impl Image {
    /// Build the base level from `texel(x, y)` and every smaller level by averaging 2x2 texels.
    /// `width` and `height` must be powers of two, at least 4.
    pub fn from_fn(width: u32, height: u32, texel: impl Fn(u32, u32) -> [u8; 4]) -> Self {
        assert!(width.is_power_of_two() && height.is_power_of_two(), "powers of two only");
        assert!(width >= SMALLEST_MIP && height >= SMALLEST_MIP);
        let mut base = Vec::with_capacity((width * height * 4) as usize);
        for y in 0..height {
            for x in 0..width {
                base.extend_from_slice(&texel(x, y));
            }
        }
        let mut mips = vec![base];
        let (mut w, mut h) = (width, height);
        while w > SMALLEST_MIP && h > SMALLEST_MIP {
            let next = halve(&mips[mips.len() - 1], w, h);
            (w, h) = (w / 2, h / 2);
            mips.push(next);
        }
        Self { width, height, mips }
    }

    /// Alternating cells of two colours, `cell` texels wide.
    pub fn checker(size: u32, cell: u32, a: [u8; 3], b: [u8; 3]) -> Self {
        Self::from_fn(size, size, |x, y| {
            let c = if (x / cell + y / cell).is_multiple_of(2) { a } else { b };
            [c[0], c[1], c[2], 255]
        })
    }

    /// A colour ramp along x with a darker band every 16 texels in y.
    pub fn ramp(size: u32, from: [u8; 3], to: [u8; 3]) -> Self {
        Self::from_fn(size, size, |x, y| {
            let t = x as f32 / (size - 1) as f32;
            let band = if (y / 16) % 2 == 0 { 1.0 } else { 0.8 };
            let mix = |i: usize| ((f32::from(from[i]) * (1.0 - t) + f32::from(to[i]) * t) * band) as u8;
            [mix(0), mix(1), mix(2), 255]
        })
    }

    /// A white disc whose alpha is 1 inside and falls to 0 over the outer fifth of the radius, with a
    /// vein darker than the rest: a stand-in for a foliage card.
    pub fn leaf_card(size: u32) -> Self {
        Self::from_fn(size, size, |x, y| {
            let (u, v) = ((x as f32 + 0.5) / size as f32 * 2.0 - 1.0, (y as f32 + 0.5) / size as f32 * 2.0 - 1.0);
            let radius = (u * u + v * v).sqrt();
            let alpha = ((1.0 - radius) / 0.2).clamp(0.0, 1.0);
            let vein = if u.abs() < 0.06 { 0.7 } else { 1.0 };
            let grey = (255.0 * vein) as u8;
            [grey, grey, grey, (alpha * 255.0) as u8]
        })
    }

    /// A white radial glow: alpha (and brightness) falls off smoothly from the centre.
    pub fn glow(size: u32) -> Self {
        Self::from_fn(size, size, |x, y| {
            let (u, v) = ((x as f32 + 0.5) / size as f32 * 2.0 - 1.0, (y as f32 + 0.5) / size as f32 * 2.0 - 1.0);
            let falloff = (1.0 - (u * u + v * v).sqrt()).clamp(0.0, 1.0);
            let a = (falloff * falloff * 255.0) as u8;
            [255, 255, 255, a]
        })
    }

    /// This image after a round trip through BC1: what a backend without BC support is given so that
    /// it shows the same pixels as one that uploads the blocks.
    pub fn through_bc1(&self) -> Self {
        let mips = self.bc1_mips().iter().zip(self.sizes()).map(|(data, (w, h))| bc1::decode(data, w, h)).collect();
        Self { mips, ..self.clone() }
    }

    /// Each mip as BC1 blocks.
    pub fn bc1_mips(&self) -> Vec<Vec<u8>> {
        self.mips.iter().zip(self.sizes()).map(|(rgba, (w, h))| bc1::encode(rgba, w, h)).collect()
    }

    /// The (width, height) of each mip.
    pub fn sizes(&self) -> Vec<(u32, u32)> {
        (0..self.mips.len()).map(|level| ((self.width >> level).max(1), (self.height >> level).max(1))).collect()
    }
}

/// Average each 2x2 group of an RGBA8 level.
fn halve(rgba: &[u8], width: u32, height: u32) -> Vec<u8> {
    let (w, h) = (width as usize, height as usize);
    let at = |x: usize, y: usize, c: usize| u32::from(rgba[(y * w + x) * 4 + c]);
    let mut out = Vec::with_capacity(w / 2 * (h / 2) * 4);
    for y in 0..h / 2 {
        for x in 0..w / 2 {
            for c in 0..4 {
                let sum = at(2 * x, 2 * y, c)
                    + at(2 * x + 1, 2 * y, c)
                    + at(2 * x, 2 * y + 1, c)
                    + at(2 * x + 1, 2 * y + 1, c);
                out.push(((sum + 2) / 4) as u8);
            }
        }
    }
    out
}

/// How a texture reaches the backend.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Storage {
    /// RGBA8 as generated.
    Rgba8,
    /// BC1 when the backend can take it, else the same pixels decoded to RGBA8.
    Bc1,
}

/// Upload `image` through `backend` as `storage`. The pixels are the same whether or not the backend
/// supports compressed textures.
pub fn upload(backend: &mut dyn RenderBackend, label: &str, image: &Image, storage: Storage) -> TextureHandle {
    let compressed = storage == Storage::Bc1 && backend.capabilities().compressed_bc;
    let (format, mips) = match (storage, compressed) {
        (Storage::Rgba8, _) => (PixelFormat::Rgba8, image.mips.clone()),
        (Storage::Bc1, true) => (PixelFormat::Bc1, image.bc1_mips()),
        (Storage::Bc1, false) => (PixelFormat::Rgba8, image.through_bc1().mips),
    };
    let desc = TextureDesc {
        label,
        width: image.width,
        height: image.height,
        format,
        mips: mips.iter().map(Vec::as_slice).collect(),
    };
    backend.create_texture(&desc)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_mip_chain_stops_at_four_texels() {
        let image = Image::checker(64, 8, [255, 0, 0], [0, 0, 255]);
        assert_eq!(image.mips.len(), 5, "64, 32, 16, 8, 4");
        assert_eq!(image.sizes().last(), Some(&(4, 4)));
        for (mip, (w, h)) in image.mips.iter().zip(image.sizes()) {
            assert_eq!(mip.len(), (w * h * 4) as usize);
        }
    }

    #[test]
    fn halving_averages_a_checker_to_grey() {
        let image = Image::checker(8, 1, [0, 0, 0], [200, 200, 200]);
        assert!(image.mips[1].as_chunks::<4>().0.iter().all(|t| t[0] == 100 && t[3] == 255));
    }

    #[test]
    fn the_leaf_card_is_solid_in_the_middle_and_clear_outside() {
        let image = Image::leaf_card(32);
        let alpha = |x: usize, y: usize| image.mips[0][(y * 32 + x) * 4 + 3];
        assert_eq!(alpha(20, 16), 255);
        assert_eq!(alpha(0, 0), 0);
    }

    #[test]
    fn bc1_keeps_a_checker_close() {
        let image = Image::checker(16, 4, [220, 60, 30], [30, 120, 220]);
        let lossy = image.through_bc1();
        let worst = image.mips[0].iter().zip(&lossy.mips[0]).map(|(a, b)| a.abs_diff(*b)).max().unwrap_or(0);
        assert!(worst <= 10, "worst error {worst}");
        assert_eq!(lossy.sizes(), image.sizes());
    }
}

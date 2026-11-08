//! Colour grading with 3D look-up tables (cubes): the data a scene hands to a renderer, and the reference arithmetic
//! every renderer's pass must agree with. Spec: `docs/specs/colour-grading.md`.
//!
//! A [`ColourCube`] is a cube of `size`^3 display-space colours. The input colour picks a position in the cube (red
//! along x, green along y, blue along z, each from the first entry at 0 to the last at 1) and the output is the
//! trilinear interpolation of the 8 entries around it. A [`ColourGrade`] names the cube in force (and optionally a
//! second one to blend with, for a time of day that lies between two looks) and the strength of the whole grade. A
//! scene sends one every frame while it grades and `None` when it does not, so a scene that grades nothing pays nothing.

use std::sync::Arc;

/// The smallest and largest edge of a cube. 16 and 32 are the usual ones; the limit keeps a bad file from asking for
/// a texture of hundreds of megabytes (64^3 is 1 MiB as RGBA8).
pub const CUBE_SIZE_RANGE: (u32, u32) = (2, 64);

/// A 3D look-up table: `size`^3 entries of 8-bit RGB, red varying fastest, then green, then blue (entry `x + size * y +
/// size^2 * z` is the output for the input at `(x, y, z)`).
#[derive(Debug, PartialEq, Eq)]
pub struct ColourCube {
    size: u32,
    rgb: Vec<u8>,
    id: u64,
}

impl ColourCube {
    /// A cube from `rgb` (3 bytes an entry, red first); `None` when `size` is out of [`CUBE_SIZE_RANGE`] or the data is
    /// not `size`^3 entries.
    pub fn new(size: u32, rgb: Vec<u8>) -> Option<Self> {
        let (lo, hi) = CUBE_SIZE_RANGE;
        if !(lo..=hi).contains(&size) || rgb.len() != (size as usize).pow(3) * 3 {
            return None;
        }
        let id = fingerprint(size, &rgb);
        Some(Self { size, rgb, id })
    }

    /// The cube of `size` that leaves every colour as it is (every entry at its own position, rounded to a byte).
    pub fn identity(size: u32) -> Option<Self> {
        let (lo, hi) = CUBE_SIZE_RANGE;
        if !(lo..=hi).contains(&size) {
            return None;
        }
        let n = size as usize;
        let step = 255.0 / (n - 1) as f32;
        let mut rgb = Vec::with_capacity(n * n * n * 3);
        for z in 0..n {
            for y in 0..n {
                for x in 0..n {
                    rgb.extend([x, y, z].map(|i| (i as f32 * step).round() as u8));
                }
            }
        }
        Self::new(size, rgb)
    }

    /// The edge of the cube.
    pub fn size(&self) -> u32 {
        self.size
    }

    /// The entries: 3 bytes (red, green, blue) each, red varying fastest.
    pub fn rgb(&self) -> &[u8] {
        &self.rgb
    }

    /// A fingerprint of the size and the entries: equal cubes have equal ids, so a renderer can keep one texture for
    /// a cube that arrives again.
    pub fn id(&self) -> u64 {
        self.id
    }

    /// The bytes as RGBA8 (alpha 255), the layout of the textures a renderer uploads.
    pub fn rgba(&self) -> Vec<u8> {
        self.rgb.chunks_exact(3).flat_map(|c| [c[0], c[1], c[2], 255]).collect()
    }

    /// The entry at `(x, y, z)` as 0 to 1 floats; the indices are clamped into the cube.
    pub fn entry(&self, x: u32, y: u32, z: u32) -> [f32; 3] {
        let last = self.size - 1;
        let n = self.size as usize;
        let at = (x.min(last) as usize + n * y.min(last) as usize + n * n * z.min(last) as usize) * 3;
        [self.rgb[at], self.rgb[at + 1], self.rgb[at + 2]].map(|b| f32::from(b) / 255.0)
    }

    /// Whether every entry is its own position to within `tolerance` of a byte: a cube that changes nothing.
    pub fn is_identity(&self, tolerance: f32) -> bool {
        let Some(identity) = Self::identity(self.size) else { return false };
        self.rgb.iter().zip(&identity.rgb).all(|(a, b)| (f32::from(*a) - f32::from(*b)).abs() <= tolerance)
    }

    /// The trilinear lookup of `rgb` (each channel clamped to 0 to 1): the arithmetic of the GPU's linear filter on the
    /// cube texture, which the passes' tests compare against.
    pub fn sample(&self, rgb: [f32; 3]) -> [f32; 3] {
        let scale = (self.size - 1) as f32;
        let at = rgb.map(|c| if c.is_finite() { c.clamp(0.0, 1.0) * scale } else { 0.0 });
        let base = at.map(|p| p.floor().min(scale - 1.0));
        let frac = [at[0] - base[0], at[1] - base[1], at[2] - base[2]];
        let (x, y, z) = (base[0] as u32, base[1] as u32, base[2] as u32);
        let mut out = [0.0; 3];
        for corner in 0..8u32 {
            let (dx, dy, dz) = (corner & 1, (corner >> 1) & 1, (corner >> 2) & 1);
            let weight = [(dx, frac[0]), (dy, frac[1]), (dz, frac[2])]
                .iter()
                .map(|&(d, f)| if d == 1 { f } else { 1.0 - f })
                .product::<f32>();
            let entry = self.entry(x + dx, y + dy, z + dz);
            for channel in 0..3 {
                out[channel] += entry[channel] * weight;
            }
        }
        out
    }
}

/// FNV-1a over the size and the bytes.
fn fingerprint(size: u32, rgb: &[u8]) -> u64 {
    const PRIME: u64 = 0x0000_0100_0000_01b3;
    let seed = size.to_le_bytes().iter().fold(0xcbf2_9ce4_8422_2325u64, |h, b| (h ^ u64::from(*b)).wrapping_mul(PRIME));
    rgb.iter().fold(seed, |h, b| (h ^ u64::from(*b)).wrapping_mul(PRIME))
}

/// The grade a scene asks for, for the frame: `strength` of the way from the original colour to `from` looked up, or
/// to `from` and `to` blended by `mix` (0 is all `from`, 1 all `to`).
#[derive(Debug, Clone)]
pub struct ColourGrade {
    pub from: Arc<ColourCube>,
    pub to: Option<Arc<ColourCube>>,
    /// The share of `to` in the blend of the two cubes, 0 to 1. Ignored without a second cube.
    pub mix: f32,
    /// How far the frame goes from its own colour (0) to the graded colour (1).
    pub strength: f32,
}

impl ColourGrade {
    /// One cube at `strength`.
    pub fn single(cube: Arc<ColourCube>, strength: f32) -> Self {
        Self { from: cube, to: None, mix: 0.0, strength }
    }

    /// The grade with `mix` and `strength` clamped to 0 to 1 (0 when not finite), and a second cube of another size or no
    /// blend dropped.
    pub fn sanitized(&self) -> Self {
        let unit = |v: f32| if v.is_finite() { v.clamp(0.0, 1.0) } else { 0.0 };
        let mix = unit(self.mix);
        let to = self.to.clone().filter(|_| mix > 0.0);
        Self {
            from: self.from.clone(),
            to,
            mix: if self.to.is_some() { mix } else { 0.0 },
            strength: unit(self.strength),
        }
    }

    /// Whether the grade changes nothing (no strength), so nothing need be drawn.
    pub fn is_neutral(&self) -> bool {
        self.sanitized().strength <= 0.0
    }

    /// The graded colour of `rgb` (0 to 1) as the pass computes it: the cubes looked up and blended, then the colour
    /// moved `strength` of the way. A value above 1 keeps its excess: only the part inside 0 to 1 is graded, so a bright
    /// colour is not clipped (a scene that is not tone mapped holds values above 1).
    pub fn apply(&self, rgb: [f32; 3]) -> [f32; 3] {
        let g = self.sanitized();
        let inside = rgb.map(|c| if c.is_finite() { c.clamp(0.0, 1.0) } else { 0.0 });
        let mut graded = g.from.sample(inside);
        if let Some(to) = &g.to {
            let other = to.sample(inside);
            for channel in 0..3 {
                graded[channel] += (other[channel] - graded[channel]) * g.mix;
            }
        }
        let mut out = [0.0; 3];
        for channel in 0..3 {
            let excess = if rgb[channel].is_finite() { rgb[channel] - inside[channel] } else { 0.0 };
            out[channel] = inside[channel] + (graded[channel] - inside[channel]) * g.strength + excess;
        }
        out
    }
}

#[cfg(test)]
mod tests;

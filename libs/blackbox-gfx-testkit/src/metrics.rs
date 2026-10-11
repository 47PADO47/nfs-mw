//! Image differences: how far apart two renderings of the same scene are.
//!
//! Distances are absolute differences of the red, green and blue samples (alpha is ignored), on the
//! 0..=255 scale. A [`Mask`] limits the comparison to part of the image.

use std::fmt;

use blackbox_gfx::RgbaImage;

/// A pixel rectangle, `x0..x1` by `y0..y1` (end exclusive).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Region {
    pub x0: u32,
    pub y0: u32,
    pub x1: u32,
    pub y1: u32,
}

impl Region {
    pub const fn new(x0: u32, y0: u32, x1: u32, y1: u32) -> Self {
        Self { x0, y0, x1, y1 }
    }

    pub fn contains(&self, x: u32, y: u32) -> bool {
        (self.x0..self.x1).contains(&x) && (self.y0..self.y1).contains(&y)
    }
}

/// Which pixels count. A pixel counts when it is inside `only` (or `only` is empty) and outside every
/// `exclude` rectangle.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Mask {
    only: Vec<Region>,
    exclude: Vec<Region>,
}

impl Mask {
    /// Every pixel counts.
    pub fn all() -> Self {
        Self::default()
    }

    /// Only pixels inside `region` count (add more with [`Mask::and_only`]).
    pub fn only(region: Region) -> Self {
        Self { only: vec![region], exclude: Vec::new() }
    }

    pub fn and_only(mut self, region: Region) -> Self {
        self.only.push(region);
        self
    }

    /// Pixels inside `region` do not count.
    pub fn excluding(mut self, region: Region) -> Self {
        self.exclude.push(region);
        self
    }

    pub fn counts(&self, x: u32, y: u32) -> bool {
        let inside = self.only.is_empty() || self.only.iter().any(|r| r.contains(x, y));
        inside && !self.exclude.iter().any(|r| r.contains(x, y))
    }
}

/// Why two images cannot be compared.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CompareError {
    SizeMismatch { a: (u32, u32), b: (u32, u32) },
    NothingCounted,
}

impl fmt::Display for CompareError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::SizeMismatch { a, b } => write!(f, "images differ in size: {}x{} vs {}x{}", a.0, a.1, b.0, b.1),
            Self::NothingCounted => f.write_str("the mask leaves no pixel to compare"),
        }
    }
}

impl std::error::Error for CompareError {}

/// How far apart two images are.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ImageDiff {
    /// The largest single-sample difference.
    pub max: u8,
    /// The mean sample difference.
    pub mean: f64,
    /// The 99th percentile of the sample differences: 99 % of samples differ by this much or less.
    pub p99: u8,
    /// Pixels counted.
    pub pixels: u64,
}

impl ImageDiff {
    /// Whether the difference is within `tolerance`.
    pub fn within(&self, tolerance: &Tolerance) -> bool {
        self.max <= tolerance.max && self.mean <= tolerance.mean && self.p99 <= tolerance.p99
    }
}

impl fmt::Display for ImageDiff {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "max {}, mean {:.3}, p99 {} over {} px", self.max, self.mean, self.p99, self.pixels)
    }
}

/// Upper limits on an [`ImageDiff`], all out of 255.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Tolerance {
    pub max: u8,
    pub mean: f64,
    pub p99: u8,
}

impl Tolerance {
    /// Anything goes (use for the limits a comparison does not care about).
    pub const ANY: Self = Self { max: 255, mean: 255.0, p99: 255 };

    pub const fn new(max: u8, mean: f64, p99: u8) -> Self {
        Self { max, mean, p99 }
    }

    /// `Ok` when `diff` is within the limits, else an error that says by how much it is not.
    pub fn check(&self, diff: &ImageDiff) -> Result<(), String> {
        match diff.within(self) {
            true => Ok(()),
            false => Err(format!("{diff} exceeds max {}, mean {}, p99 {}", self.max, self.mean, self.p99)),
        }
    }
}

/// Compare two images over the pixels `mask` counts.
pub fn compare(a: &RgbaImage, b: &RgbaImage, mask: &Mask) -> Result<ImageDiff, CompareError> {
    if (a.width, a.height) != (b.width, b.height) {
        return Err(CompareError::SizeMismatch { a: (a.width, a.height), b: (b.width, b.height) });
    }
    let mut histogram = [0u64; 256];
    let (mut sum, mut max, mut pixels) = (0u64, 0u8, 0u64);
    for y in 0..a.height {
        for x in 0..a.width {
            if !mask.counts(x, y) {
                continue;
            }
            let at = (y as usize * a.width as usize + x as usize) * 4;
            pixels += 1;
            for c in 0..3 {
                let d = a.rgba[at + c].abs_diff(b.rgba[at + c]);
                histogram[d as usize] += 1;
                sum += u64::from(d);
                max = max.max(d);
            }
        }
    }
    if pixels == 0 {
        return Err(CompareError::NothingCounted);
    }
    let samples = pixels * 3;
    // The smallest difference that 99 % of the samples do not exceed.
    let target = (samples * 99).div_ceil(100);
    let mut seen = 0;
    let p99 = (0..=255u8)
        .find(|&d| {
            seen += histogram[d as usize];
            seen >= target
        })
        .unwrap_or(255);
    Ok(ImageDiff { max, mean: sum as f64 / samples as f64, p99, pixels })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn flat(width: u32, height: u32, rgb: [u8; 3]) -> RgbaImage {
        let rgba = (0..width * height).flat_map(|_| [rgb[0], rgb[1], rgb[2], 255]).collect();
        RgbaImage::new(width, height, rgba).unwrap()
    }

    #[test]
    fn identical_images_have_no_difference() {
        let a = flat(8, 8, [10, 20, 30]);
        let d = compare(&a, &a, &Mask::all()).unwrap();
        assert_eq!((d.max, d.p99, d.pixels), (0, 0, 64));
        assert_eq!(d.mean, 0.0);
    }

    #[test]
    fn a_uniform_offset_shows_in_every_metric() {
        let d = compare(&flat(4, 4, [10, 10, 10]), &flat(4, 4, [13, 10, 10]), &Mask::all()).unwrap();
        assert_eq!((d.max, d.p99), (3, 3));
        assert!((d.mean - 1.0).abs() < 1e-9, "one of three samples differs by 3");
    }

    #[test]
    fn p99_ignores_a_rare_outlier_but_max_does_not() {
        let a = flat(20, 20, [0, 0, 0]);
        let mut b = a.clone();
        b.rgba[0] = 200;
        let d = compare(&a, &b, &Mask::all()).unwrap();
        assert_eq!((d.max, d.p99), (200, 0), "1 sample of 1200 is under 1 %");
        assert!(d.mean > 0.0);
    }

    #[test]
    fn alpha_is_ignored() {
        let a = flat(2, 2, [5, 5, 5]);
        let mut b = a.clone();
        b.rgba[3] = 0;
        assert_eq!(compare(&a, &b, &Mask::all()).unwrap().max, 0);
    }

    #[test]
    fn a_mask_limits_and_excludes() {
        let a = flat(8, 8, [0, 0, 0]);
        let mut b = a.clone();
        let at = (4 * 8 + 4) * 4;
        b.rgba[at] = 90;
        let near = Region::new(0, 0, 4, 4);
        assert_eq!(compare(&a, &b, &Mask::only(near)).unwrap().max, 0);
        assert_eq!(compare(&a, &b, &Mask::only(Region::new(4, 4, 8, 8))).unwrap().max, 90);
        assert_eq!(compare(&a, &b, &Mask::all().excluding(Region::new(4, 4, 5, 5))).unwrap().max, 0);
        let two = Mask::only(near).and_only(Region::new(4, 4, 8, 8));
        assert_eq!(compare(&a, &b, &two).unwrap().pixels, 16 + 16);
    }

    #[test]
    fn mismatches_and_empty_masks_are_errors() {
        let a = flat(4, 4, [0; 3]);
        assert!(matches!(compare(&a, &flat(4, 2, [0; 3]), &Mask::all()), Err(CompareError::SizeMismatch { .. })));
        assert_eq!(compare(&a, &a, &Mask::only(Region::new(9, 9, 10, 10))), Err(CompareError::NothingCounted));
    }

    #[test]
    fn tolerances_compare_every_metric() {
        let d = ImageDiff { max: 4, mean: 0.4, p99: 2, pixels: 10 };
        assert!(d.within(&Tolerance::new(4, 0.5, 2)));
        assert!(!d.within(&Tolerance::new(3, 0.5, 2)));
        assert!(!d.within(&Tolerance::new(4, 0.3, 2)));
        assert!(!d.within(&Tolerance::new(4, 0.5, 1)));
        assert!(d.within(&Tolerance::ANY));
        assert!(Tolerance::new(1, 0.1, 1).check(&d).unwrap_err().contains("exceeds"));
    }
}

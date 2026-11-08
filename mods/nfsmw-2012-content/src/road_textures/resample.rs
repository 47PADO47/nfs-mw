//! Resizing RGBA8 pictures: a separable triangle filter that widens with the reduction, so a picture that shrinks is averaged and
//! one that grows is interpolated.

/// A picture of 8-bit RGBA rows.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Rgba {
    pub width: u32,
    pub height: u32,
    pub pixels: Vec<u8>,
}

impl Rgba {
    pub fn new(width: u32, height: u32, pixels: Vec<u8>) -> Self {
        debug_assert_eq!(pixels.len(), (width * height * 4) as usize);
        Self { width, height, pixels }
    }

    /// A picture of one colour.
    pub fn filled(width: u32, height: u32, colour: [u8; 4]) -> Self {
        Self { width, height, pixels: colour.repeat((width * height) as usize) }
    }

    /// The pixel at `x`, `y`.
    pub fn at(&self, x: u32, y: u32) -> [u8; 4] {
        let at = ((y * self.width + x) * 4) as usize;
        [self.pixels[at], self.pixels[at + 1], self.pixels[at + 2], self.pixels[at + 3]]
    }

    /// The part of the picture inside `x`, `y`, `width`, `height` (clamped to the picture; never empty).
    pub fn crop(&self, x: u32, y: u32, width: u32, height: u32) -> Rgba {
        let x = x.min(self.width - 1);
        let y = y.min(self.height - 1);
        let width = width.clamp(1, self.width - x);
        let height = height.clamp(1, self.height - y);
        let mut pixels = Vec::with_capacity((width * height * 4) as usize);
        for row in y..y + height {
            let start = ((row * self.width + x) * 4) as usize;
            pixels.extend_from_slice(&self.pixels[start..start + (width * 4) as usize]);
        }
        Rgba { width, height, pixels }
    }

    /// The picture turned `turns` quarter turns clockwise.
    pub fn rotate(&self, turns: u8) -> Rgba {
        let mut picture = self.clone();
        for _ in 0..turns % 4 {
            picture = picture.quarter();
        }
        picture
    }

    fn quarter(&self) -> Rgba {
        let (w, h) = (self.width, self.height);
        let mut pixels = vec![0; self.pixels.len()];
        for y in 0..h {
            for x in 0..w {
                // (x, y) goes to (h - 1 - y, x) in a picture h wide and w high.
                let to = ((x * h + (h - 1 - y)) * 4) as usize;
                pixels[to..to + 4].copy_from_slice(&self.at(x, y));
            }
        }
        Rgba { width: h, height: w, pixels }
    }
}

/// The taps of one output pixel along one axis: the first input index and the weights.
fn taps(out: u32, input: u32, at: u32) -> (u32, Vec<f32>) {
    let ratio = input as f32 / out as f32;
    let centre = (at as f32 + 0.5) * ratio - 0.5;
    let radius = ratio.max(1.0);
    let first = (centre - radius).ceil().max(0.0) as i64;
    let last = ((centre + radius).floor() as i64).min(i64::from(input) - 1);
    let mut weights = Vec::new();
    for index in first..=last {
        weights.push((1.0 - (index as f32 - centre).abs() / radius).max(0.0));
    }
    let sum: f32 = weights.iter().sum();
    if sum <= 0.0 {
        // Every tap fell outside: take the nearest input pixel.
        let nearest = centre.round().clamp(0.0, (input - 1) as f32) as u32;
        return (nearest, vec![1.0]);
    }
    weights.iter_mut().for_each(|weight| *weight /= sum);
    (first.max(0) as u32, weights)
}

/// `picture` resized to `width` by `height`.
pub fn resize(picture: &Rgba, width: u32, height: u32) -> Rgba {
    if picture.width == width && picture.height == height {
        return picture.clone();
    }
    // Horizontal pass into a picture `width` wide and as high as the input.
    let mut across = vec![0f32; (width * picture.height * 4) as usize];
    for x in 0..width {
        let (first, weights) = taps(width, picture.width, x);
        for y in 0..picture.height {
            let mut sum = [0f32; 4];
            for (offset, weight) in weights.iter().enumerate() {
                let pixel = picture.at(first + offset as u32, y);
                for channel in 0..4 {
                    sum[channel] += weight * f32::from(pixel[channel]);
                }
            }
            let to = ((y * width + x) * 4) as usize;
            across[to..to + 4].copy_from_slice(&sum);
        }
    }
    let mut pixels = vec![0u8; (width * height * 4) as usize];
    for y in 0..height {
        let (first, weights) = taps(height, picture.height, y);
        for x in 0..width {
            let mut sum = [0f32; 4];
            for (offset, weight) in weights.iter().enumerate() {
                let from = (((first + offset as u32) * width + x) * 4) as usize;
                for channel in 0..4 {
                    sum[channel] += weight * across[from + channel];
                }
            }
            let to = ((y * width + x) * 4) as usize;
            for channel in 0..4 {
                pixels[to + channel] = sum[channel].round().clamp(0.0, 255.0) as u8;
            }
        }
    }
    Rgba { width, height, pixels }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ramp(width: u32, height: u32) -> Rgba {
        let mut pixels = Vec::new();
        for y in 0..height {
            for x in 0..width {
                pixels.extend_from_slice(&[(x * 10) as u8, (y * 10) as u8, 0, 255]);
            }
        }
        Rgba::new(width, height, pixels)
    }

    #[test]
    fn a_flat_picture_stays_flat_at_any_size() {
        let flat = Rgba::filled(16, 16, [10, 120, 250, 255]);
        for (w, h) in [(4, 4), (32, 8), (7, 33)] {
            let small = resize(&flat, w, h);
            assert!(small.pixels.chunks(4).all(|p| p == [10, 120, 250, 255]), "{w}x{h}");
        }
    }

    #[test]
    fn halving_averages_neighbours() {
        let mut picture = Rgba::filled(2, 1, [0, 0, 0, 255]);
        picture.pixels[0..4].copy_from_slice(&[200, 0, 0, 255]);
        let one = resize(&picture, 1, 1);
        assert!((95..=105).contains(&one.pixels[0]), "{}", one.pixels[0]);
    }

    #[test]
    fn a_quarter_turn_clockwise_moves_the_top_left_to_the_top_right() {
        let picture = ramp(3, 2);
        let turned = picture.rotate(1);
        assert_eq!((turned.width, turned.height), (2, 3));
        assert_eq!(turned.at(1, 0), picture.at(0, 0));
        assert_eq!(turned.at(0, 0), picture.at(0, 1));
        assert_eq!(picture.rotate(4), picture);
    }

    #[test]
    fn a_crop_is_clamped_to_the_picture() {
        let picture = ramp(8, 8);
        let part = picture.crop(6, 6, 10, 10);
        assert_eq!((part.width, part.height), (2, 2));
        assert_eq!(part.at(0, 0), picture.at(6, 6));
    }
}

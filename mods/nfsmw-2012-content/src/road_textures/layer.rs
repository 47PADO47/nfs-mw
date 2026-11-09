//! A texture made of layers: each takes a part of a 2012 picture, turns it, sizes it to a rectangle of the result and blends it in.
//!
//! This is all the converter does to a picture: no layer needs more than a crop, a quarter turn, a resize, a tiling and a mask.
//! Layers are described in fractions of the result, so one recipe makes a texture at any size.

use std::sync::Arc;

use anyhow::{Result, ensure};

use super::resample::{Rgba, resize};

/// A rectangle of the result as fractions of its width and height.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Rect {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}

impl Rect {
    pub const FULL: Rect = Rect::new(0.0, 0.0, 1.0, 1.0);

    pub const fn new(x: f32, y: f32, w: f32, h: f32) -> Self {
        Self { x, y, w, h }
    }
}

/// A part of a source picture in its own pixels at full size.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Crop {
    pub x: u32,
    pub y: u32,
    pub w: u32,
    pub h: u32,
}

/// How a layer's picture meets what is already there.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Blend {
    /// It replaces the pixels.
    Opaque,
    /// Its brightness is its opacity: black is clear, `hi` and above is solid (a painted line on a black picture).
    Luma { lo: u8, hi: u8 },
    /// Only the disc inscribed in the rectangle (a round cover on a square picture).
    Disc,
}

/// One layer of a [`Recipe`](super::recipe::Recipe).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Layer {
    /// The 2012 texture, by the low 32 bits of its resource id in `GLOBALRESOURCES.BNDL`.
    pub src: u32,
    pub crop: Option<Crop>,
    /// Quarter turns clockwise, applied to the crop.
    pub turns: u8,
    pub dst: Rect,
    /// How many times the picture repeats across and down inside `dst`; 0 on one axis makes the tile square.
    pub tiles: (u32, u32),
    /// Cut the crop (from its middle) to the shape of one tile before it is resized, so nothing is stretched.
    pub keep_aspect: bool,
    pub blend: Blend,
    /// Show the layer only in dashes along the rectangle's height: `(period, on, phase)` as fractions of it, the first dash starting at
    /// `phase` (a painted lane line is dashed; the 2005 strip it stands in for has about six dashes).
    pub dash: Option<(f32, f32, f32)>,
}

impl Layer {
    /// The whole of `src` over the whole result, opaque, once.
    pub const fn new(src: u32) -> Self {
        Self {
            src,
            crop: None,
            turns: 0,
            dst: Rect::FULL,
            tiles: (1, 1),
            keep_aspect: false,
            blend: Blend::Opaque,
            dash: None,
        }
    }

    pub const fn crop(mut self, x: u32, y: u32, w: u32, h: u32) -> Self {
        self.crop = Some(Crop { x, y, w, h });
        self
    }

    pub const fn turns(mut self, turns: u8) -> Self {
        self.turns = turns;
        self
    }

    pub const fn dst(mut self, x: f32, y: f32, w: f32, h: f32) -> Self {
        self.dst = Rect::new(x, y, w, h);
        self
    }

    pub const fn tiles(mut self, across: u32, down: u32) -> Self {
        self.tiles = (across, down);
        self
    }

    pub const fn keep_aspect(mut self) -> Self {
        self.keep_aspect = true;
        self
    }

    pub const fn dash(mut self, period: f32, on: f32, phase: f32) -> Self {
        self.dash = Some((period, on, phase));
        self
    }

    pub const fn luma(mut self, lo: u8, hi: u8) -> Self {
        self.blend = Blend::Luma { lo, hi };
        self
    }

    pub const fn disc(mut self) -> Self {
        self.blend = Blend::Disc;
        self
    }
}

/// Where the pictures of the 2012 game come from.
pub trait Pictures {
    /// Width, height and mip count of a texture.
    fn size(&self, key: u32) -> Result<(u32, u32, u32)>;
    /// Mip `level` of a texture as RGBA8.
    fn level(&self, key: u32, level: u32) -> Result<Arc<Rgba>>;
}

/// The pixel rectangle of `rect` on a picture `width` by `height`, never empty and never outside.
fn pixels(rect: Rect, width: u32, height: u32) -> (u32, u32, u32, u32) {
    let x = ((rect.x * width as f32).round() as u32).min(width - 1);
    let y = ((rect.y * height as f32).round() as u32).min(height - 1);
    let w = ((rect.w * width as f32).round() as u32).clamp(1, width - x);
    let h = ((rect.h * height as f32).round() as u32).clamp(1, height - y);
    (x, y, w, h)
}

/// How many tiles a layer repeats: a 0 asks for square tiles on that axis.
fn tile_counts(tiles: (u32, u32), w: u32, h: u32) -> (u32, u32) {
    match tiles {
        (0, 0) => (1, 1),
        (0, down) => (((w * down) as f32 / h as f32).round().max(1.0) as u32, down),
        (across, 0) => (across, ((h * across) as f32 / w as f32).round().max(1.0) as u32),
        both => both,
    }
}

/// `crop`, cut from its middle to the shape `want_w` by `want_h`.
fn matching_shape(crop: Crop, want_w: u32, want_h: u32) -> Crop {
    let (cw, ch) = (u64::from(crop.w), u64::from(crop.h));
    let (ww, wh) = (u64::from(want_w), u64::from(want_h));
    // Compare the shapes without division: cw / ch against ww / wh.
    if cw * wh > ww * ch {
        let w = ((ch * ww / wh) as u32).max(1);
        return Crop { x: crop.x + (crop.w - w) / 2, w, ..crop };
    }
    let h = ((cw * wh / ww) as u32).max(1);
    Crop { y: crop.y + (crop.h - h) / 2, h, ..crop }
}

/// The picture of one tile of `layer`, `tile_w` by `tile_h`: the crop taken from the smallest mip that still has the pixels.
fn tile_picture(layer: &Layer, pictures: &dyn Pictures, tile_w: u32, tile_h: u32) -> Result<Rgba> {
    let (width, height, mips) = pictures.size(layer.src)?;
    let whole = Crop { x: 0, y: 0, w: width, h: height };
    let mut crop = layer.crop.unwrap_or(whole);
    crop.x = crop.x.min(width - 1);
    crop.y = crop.y.min(height - 1);
    crop.w = crop.w.clamp(1, width - crop.x);
    crop.h = crop.h.clamp(1, height - crop.y);
    let turned = layer.turns % 2 == 1;
    let (want_w, want_h) = match turned {
        true => (tile_h, tile_w),
        false => (tile_w, tile_h),
    };
    if layer.keep_aspect {
        crop = matching_shape(crop, want_w, want_h);
    }
    let mut level = 0;
    while level + 1 < mips && (crop.w >> (level + 1)) >= want_w && (crop.h >> (level + 1)) >= want_h {
        level += 1;
    }
    let picture = pictures.level(layer.src, level)?;
    let part = picture.crop(crop.x >> level, crop.y >> level, (crop.w >> level).max(1), (crop.h >> level).max(1));
    let turned_part = part.rotate(layer.turns);
    Ok(resize(&turned_part, tile_w, tile_h))
}

fn smooth(value: f32) -> f32 {
    let t = value.clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

/// How much of the layer's pixel shows: 1 for opaque, the mask for the others.
fn opacity(blend: Blend, pixel: [u8; 4], tx: u32, ty: u32, tile_w: u32, tile_h: u32) -> f32 {
    match blend {
        Blend::Opaque => 1.0,
        Blend::Luma { lo, hi } => {
            let luma =
                (u32::from(pixel[0]) * 299 + u32::from(pixel[1]) * 587 + u32::from(pixel[2]) * 114) as f32 / 1000.0;
            smooth((luma - f32::from(lo)) / (f32::from(hi) - f32::from(lo)).max(1.0))
        }
        Blend::Disc => {
            let radius = tile_w.min(tile_h) as f32 / 2.0 * 0.985;
            let (dx, dy) = (tx as f32 + 0.5 - tile_w as f32 / 2.0, ty as f32 + 0.5 - tile_h as f32 / 2.0);
            (0.5 + radius - (dx * dx + dy * dy).sqrt()).clamp(0.0, 1.0)
        }
    }
}

/// Draw `layer` onto `canvas`.
pub fn draw(canvas: &mut Rgba, layer: &Layer, pictures: &dyn Pictures) -> Result<()> {
    let (x0, y0, dw, dh) = pixels(layer.dst, canvas.width, canvas.height);
    let (across, down) = tile_counts(layer.tiles, dw, dh);
    let (tile_w, tile_h) = ((dw / across).max(1), (dh / down).max(1));
    let tile = tile_picture(layer, pictures, tile_w, tile_h)?;
    ensure!(tile.width == tile_w && tile.height == tile_h, "a tile came out {}x{}", tile.width, tile.height);
    for y in 0..dh {
        if let Some((period, on, phase)) = layer.dash
            && (((y as f32 + 0.5) / dh as f32 - phase).rem_euclid(period)) >= on
        {
            continue;
        }
        for x in 0..dw {
            let (tx, ty) = (x % tile_w, y % tile_h);
            let pixel = tile.at(tx, ty);
            let amount = opacity(layer.blend, pixel, tx, ty, tile_w, tile_h);
            if amount <= 0.0 {
                continue;
            }
            let to = (((y0 + y) * canvas.width + x0 + x) * 4) as usize;
            for channel in 0..3 {
                let below = f32::from(canvas.pixels[to + channel]);
                let value = below + (f32::from(pixel[channel]) - below) * amount;
                canvas.pixels[to + channel] = value.round().clamp(0.0, 255.0) as u8;
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Pictures that are flat colours: texture `key` is `key` as a grey level, 64 by 64 with 3 mips.
    pub(crate) struct Flat;

    impl Pictures for Flat {
        fn size(&self, _key: u32) -> Result<(u32, u32, u32)> {
            Ok((64, 64, 3))
        }

        fn level(&self, key: u32, level: u32) -> Result<Arc<Rgba>> {
            let side = 64 >> level;
            Ok(Arc::new(Rgba::filled(side, side, [key as u8, key as u8, key as u8, 255])))
        }
    }

    #[test]
    fn an_opaque_layer_fills_its_rectangle_and_leaves_the_rest() {
        let mut canvas = Rgba::filled(16, 16, [1, 1, 1, 255]);
        draw(&mut canvas, &Layer::new(200).dst(0.5, 0.0, 0.5, 1.0), &Flat).unwrap();
        assert_eq!(canvas.at(0, 5), [1, 1, 1, 255]);
        assert_eq!(canvas.at(12, 5), [200, 200, 200, 255]);
    }

    #[test]
    fn a_luma_layer_over_black_is_clear_and_over_bright_is_solid() {
        let mut canvas = Rgba::filled(8, 8, [50, 50, 50, 255]);
        draw(&mut canvas, &Layer::new(0).luma(10, 100), &Flat).unwrap();
        assert_eq!(canvas.at(3, 3), [50, 50, 50, 255], "black is clear");
        draw(&mut canvas, &Layer::new(220).luma(10, 100), &Flat).unwrap();
        assert_eq!(canvas.at(3, 3), [220, 220, 220, 255], "bright is solid");
    }

    #[test]
    fn a_disc_leaves_the_corners() {
        let mut canvas = Rgba::filled(32, 32, [0, 0, 0, 255]);
        draw(&mut canvas, &Layer::new(255).disc(), &Flat).unwrap();
        assert_eq!(canvas.at(16, 16), [255, 255, 255, 255]);
        assert_eq!(canvas.at(0, 0), [0, 0, 0, 255]);
    }

    #[test]
    fn a_dashed_layer_shows_only_its_dashes() {
        let mut canvas = Rgba::filled(4, 100, [0, 0, 0, 255]);
        draw(&mut canvas, &Layer::new(200).dash(0.5, 0.2, 0.0), &Flat).unwrap();
        let lit: Vec<bool> = (0..100).map(|y| canvas.at(1, y)[0] == 200).collect();
        assert!(lit[5] && !lit[30] && lit[55] && !lit[80], "dashes of 20 rows every 50");
        assert_eq!(lit.iter().filter(|on| **on).count(), 40);
    }

    #[test]
    fn square_tiles_repeat_down_a_strip() {
        assert_eq!(tile_counts((1, 0), 64, 256), (1, 4));
        assert_eq!(tile_counts((0, 1), 256, 64), (4, 1));
        assert_eq!(tile_counts((2, 3), 10, 10), (2, 3));
    }

    #[test]
    fn keeping_the_aspect_cuts_from_the_middle() {
        let wide = matching_shape(Crop { x: 0, y: 0, w: 100, h: 50 }, 1, 1);
        assert_eq!(wide, Crop { x: 25, y: 0, w: 50, h: 50 });
        let tall = matching_shape(Crop { x: 0, y: 0, w: 50, h: 100 }, 2, 1);
        assert_eq!(tall, Crop { x: 0, y: 37, w: 50, h: 25 });
    }

    #[test]
    fn the_smallest_mip_that_has_the_pixels_is_taken() {
        // A 64 px source at a 16 px tile: level 2 is 16 px, so the crop is taken from there and stays 16 px.
        let tile = tile_picture(&Layer::new(9), &Flat, 16, 16).unwrap();
        assert_eq!((tile.width, tile.height), (16, 16));
    }
}

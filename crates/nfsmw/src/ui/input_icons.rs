//! Independently licensed input glyphs. See assets/input-prompts/README.md.

use anyhow::{Context, Result};

use super::assets::Image;

pub const ATLAS: u32 = 0xE0C0_0001;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum Glyph {
    A,
    B,
    X,
    Y,
    Lb,
    Rb,
    Lt,
    Rt,
    Menu,
    View,
    Ls,
    Rs,
    Up,
    Down,
    Left,
    Right,
    LeftUp,
    LeftDown,
    LeftLeft,
    LeftRight,
    RightUp,
    RightDown,
    RightLeft,
    RightRight,
    Horizontal,
    Vertical,
    Controller,
}

impl Glyph {
    pub fn uv(self) -> [f32; 4] {
        let i = self as u8;
        let (x, y) = (f32::from(i % 8) / 8.0, f32::from(i / 8) / 4.0);
        [x, y, x + 1.0 / 8.0, y + 1.0 / 4.0]
    }
}

pub fn load() -> Result<Image> {
    let data = include_bytes!("../../../../assets/input-prompts/kenney-xbox.png");
    let mut reader = png::Decoder::new(std::io::Cursor::new(data)).read_info()?;
    let mut rgba = vec![0; reader.output_buffer_size().context("input atlas dimensions")?];
    let frame = reader.next_frame(&mut rgba)?;
    anyhow::ensure!(frame.color_type == png::ColorType::Rgba, "input atlas must be RGBA");
    rgba.truncate(frame.buffer_size());
    Ok(Image { width: frame.width, height: frame.height, rgba, blend: 1 })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn shipped_atlas_has_transparent_padding_and_every_glyph_is_in_bounds() {
        let image = load().unwrap();
        assert_eq!((image.width, image.height), (1024, 512));
        assert_eq!(image.rgba[3], 0);
        for glyph in [Glyph::A, Glyph::Menu, Glyph::Controller] {
            let uv = glyph.uv();
            assert!(uv[0] >= 0.0 && uv[1] >= 0.0 && uv[2] <= 1.0 && uv[3] <= 1.0);
        }
        for cell in 0..=Glyph::Controller as usize {
            let (x, y) = ((cell % 8) * 128, (cell / 8) * 128);
            assert!((y..y + 128).any(|row| (x..x + 128).any(|col| image.rgba[(row * 1024 + col) * 4 + 3] > 0)));
        }
    }
}

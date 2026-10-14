//! The Discord mark for the Discord Rich Presence category icon. See assets/discord/README.md.

use anyhow::Result;

use super::assets::{Image, decode_png_rgba};

/// The texture key of the mark, next to the controller prompts' atlas (`input_icons::ATLAS`).
pub const TEXTURE: u32 = 0xE0C0_0002;

pub fn load() -> Result<Image> {
    decode_png_rgba(include_bytes!("../../../../assets/discord/discord.png"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shipped_mark_is_a_128_pixel_white_mark_on_transparency() {
        let image = load().unwrap();
        assert_eq!((image.width, image.height), (128, 128));
        assert!(image.rgba.as_chunks::<4>().0.iter().any(|p| p[3] == 255 && p[..3] == [255, 255, 255]));
        assert!(image.rgba.as_chunks::<4>().0.iter().any(|p| p[3] == 0));
    }
}

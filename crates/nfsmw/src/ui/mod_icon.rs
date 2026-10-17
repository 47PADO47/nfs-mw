//! The mod manager's category icon: a puzzle piece with a switch cut out of it (docs/specs/mod-manager.md, section 3).
//! The picture is an original drawing bundled with the game (`assets/mods/`); see its README.

use anyhow::Result;

use super::assets::{Image, decode_png_rgba};

/// The texture key of the icon, next to the Discord mark (`discord_icon::TEXTURE`).
pub const TEXTURE: u32 = 0xE0C0_0003;

pub fn load() -> Result<Image> {
    decode_png_rgba(include_bytes!("../../../../assets/mods/mod-manager.png"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shipped_icon_is_a_128_pixel_white_glyph_on_transparency() {
        let image = load().unwrap();
        assert_eq!((image.width, image.height), (128, 128));
        assert!(image.rgba.as_chunks::<4>().0.iter().any(|p| p[3] == 255 && p[..3] == [255, 255, 255]));
        assert!(image.rgba.as_chunks::<4>().0.iter().any(|p| p[3] == 0));
    }

    #[test]
    fn the_key_is_its_own_texture() {
        assert_ne!(TEXTURE, super::super::discord_icon::TEXTURE);
        assert_ne!(TEXTURE, super::super::input_icons::ATLAS);
    }
}

//! The rider gear category icon: a helmet, for the garage's Rider entries (docs/specs/ride6-rider-gear.md, section 2).
//! The picture is an original drawing bundled with the game (`assets/mods/`); see its README.

use anyhow::Result;

use super::assets::{Image, decode_png_rgba};

/// The texture key of the icon, next to the mod manager's (`mod_icon::TEXTURE`).
pub const TEXTURE: u32 = 0xE0C0_0004;

pub fn load() -> Result<Image> {
    decode_png_rgba(include_bytes!("../../../../assets/mods/rider-gear.png"))
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
        assert_ne!(TEXTURE, super::super::mod_icon::TEXTURE);
        assert_ne!(TEXTURE, super::super::discord_icon::TEXTURE);
        assert_ne!(TEXTURE, super::super::input_icons::ATLAS);
    }
}

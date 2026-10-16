//! The colours of the bar, as linear RGBA. The spec gives sRGB bytes; they are converted here, once per use.

/// The border of a bar (`BORDER_COLOUR`).
pub const BORDER_COLOUR: [u8; 3] = [0x10, 0x10, 0x10];
/// The border of a destroyed cop, and its flash colour (`BORDER_DESTROYED_COLOUR`).
pub const BORDER_DESTROYED_COLOUR: [u8; 3] = [0x60, 0x10, 0x10];
/// The pale lagging segment (`LAG_COLOUR`).
pub const LAG_COLOUR: [u8; 3] = [0xf4, 0xf4, 0xf4];

/// One sRGB channel, as a byte, to linear light (the standard sRGB transfer curve).
pub fn srgb_to_linear(channel: u8) -> f32 {
    let c = f32::from(channel) / 255.0;
    if c <= 0.04045 {
        return c / 12.92;
    }
    ((c + 0.055) / 1.055).powf(2.4)
}

/// An sRGB colour and an alpha, as linear RGBA.
pub fn rgba(rgb: [u8; 3], alpha: f32) -> [f32; 4] {
    [srgb_to_linear(rgb[0]), srgb_to_linear(rgb[1]), srgb_to_linear(rgb[2]), alpha]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::health_bar::approx;

    #[test]
    fn channel_ends_map_to_zero_and_one() {
        assert!(approx(srgb_to_linear(0), 0.0));
        assert!(approx(srgb_to_linear(255), 1.0));
    }

    #[test]
    fn mid_grey_is_darker_in_linear_light() {
        assert!(approx(srgb_to_linear(128), 0.2158605));
    }

    #[test]
    fn rgba_keeps_the_alpha_given() {
        let colour = rgba([255, 0, 0], 0.7);
        assert!(approx(colour[0], 1.0));
        assert!(approx(colour[1], 0.0));
        assert!(approx(colour[3], 0.7));
    }
}

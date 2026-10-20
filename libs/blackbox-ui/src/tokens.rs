//! The theme tokens: the palette and the shape a theme declares for its components.
//!
//! A theme returns its [`Tokens`] from [`Theme::tokens`](crate::Theme::tokens). The registry writes them into the skin
//! of the active theme under [`keys`], so every component reads the active theme's colours, radius and padding without
//! knowing which theme it is. A mod theme declares its tokens the same way.

use crate::skin::Skin;

/// The skin keys the tokens are written under. Components read these, never a theme's own name.
pub mod keys {
    pub const PRIMARY: &str = "theme.primary";
    pub const SECONDARY: &str = "theme.secondary";
    pub const BACKGROUND: &str = "theme.background";
    pub const BORDER: &str = "theme.border";
    pub const MUTED: &str = "theme.muted";
    pub const RADIUS: &str = "theme.radius";
    pub const PADDING: &str = "theme.padding";
    pub const TEXT_SCALE: &str = "theme.text_scale";
    pub const ICONS: &str = "theme.icons";
}

/// What a theme declares: five colours (`[r, g, b, a]`) and two shape values in FEng units.
///
/// - `primary`: the main text and digits.
/// - `secondary`: the accent, such as a warning colour.
/// - `background`: the fill of a panel.
/// - `border`: rules, tracks and the empty part of a gauge.
/// - `muted`: labels and secondary text.
/// - `radius`: the corner radius of a panel.
/// - `padding`: the space between a panel's edge and its content.
/// - `text_scale`: the size of the theme's text, as a multiple of the size its components set (1 keeps it).
/// - `icons`: whether a menu shows the picture of an entry beside its text, where the entry has one (off by default).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Tokens {
    pub primary: [u8; 4],
    pub secondary: [u8; 4],
    pub background: [u8; 4],
    pub border: [u8; 4],
    pub muted: [u8; 4],
    pub radius: f32,
    pub padding: f32,
    pub text_scale: f32,
    pub icons: bool,
}

impl Default for Tokens {
    /// A neutral palette: white text, a transparent background, and no corners or padding.
    fn default() -> Self {
        Self {
            primary: [255, 255, 255, 255],
            secondary: [255, 200, 80, 255],
            background: [0, 0, 0, 0],
            border: [255, 255, 255, 40],
            muted: [180, 185, 195, 255],
            radius: 0.0,
            padding: 0.0,
            text_scale: 1.0,
            icons: false,
        }
    }
}

impl Tokens {
    /// The tokens as skin entries, under [`keys`].
    pub fn skin(&self) -> Skin {
        Skin::new()
            .with_colour(keys::PRIMARY, self.primary)
            .with_colour(keys::SECONDARY, self.secondary)
            .with_colour(keys::BACKGROUND, self.background)
            .with_colour(keys::BORDER, self.border)
            .with_colour(keys::MUTED, self.muted)
            .with_metric(keys::RADIUS, self.radius)
            .with_metric(keys::PADDING, self.padding)
            .with_metric(keys::TEXT_SCALE, self.text_scale)
            .with_metric(keys::ICONS, if self.icons { 1.0 } else { 0.0 })
    }
}

#[cfg(test)]
mod tests {
    use super::{Tokens, keys};

    #[test]
    fn the_tokens_are_read_back_by_their_keys() {
        let tokens = Tokens { background: [1, 2, 3, 4], radius: 6.0, padding: 2.5, ..Tokens::default() };
        let skin = tokens.skin();
        assert_eq!(skin.colour(keys::BACKGROUND), Some([1, 2, 3, 4]));
        assert_eq!(skin.colour(keys::PRIMARY), Some(tokens.primary));
        assert_eq!(skin.metric(keys::RADIUS), Some(6.0));
        assert_eq!(skin.metric(keys::PADDING), Some(2.5));
    }

    #[test]
    fn the_icons_flag_is_off_by_default_and_reads_back_as_a_metric() {
        assert_eq!(Tokens::default().skin().metric(keys::ICONS), Some(0.0));
        assert_eq!(Tokens { icons: true, ..Tokens::default() }.skin().metric(keys::ICONS), Some(1.0));
    }
}

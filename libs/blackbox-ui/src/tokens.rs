//! The theme tokens: the palette and the shape a theme declares for its components.
//!
//! A theme returns its [`Tokens`] from [`Theme::tokens`](crate::Theme::tokens). The registry writes them into the skin
//! of the active theme under [`keys`], so every component reads the active theme's colours, radius and padding without
//! knowing which theme it is. A mod theme declares its tokens the same way.

use crate::skin::Skin;

mod roles;

pub use roles::{ResolvedRoles, Roles};

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
    /// The colour roles (see [`Roles`](super::Roles)): linework, guide, meaning colours, scrim and overlay.
    pub const LINE: &str = "theme.line";
    pub const LINE_MINOR: &str = "theme.line_minor";
    pub const LINE_FAINT: &str = "theme.line_faint";
    pub const GUIDE: &str = "theme.guide";
    pub const SUCCESS: &str = "theme.success";
    pub const CAUTION: &str = "theme.caution";
    pub const DANGER: &str = "theme.danger";
    pub const INFO: &str = "theme.info";
    pub const SCRIM: &str = "theme.scrim";
    pub const OVERLAY: &str = "theme.overlay";
}

/// What a theme declares: five colours (`[r, g, b, a]`), two shape values in FEng units, and optional colour roles.
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
/// - `roles`: the colours for linework, a guide path, the meaning colours, a scrim and an overlay plate. A role the theme
///   leaves unset is derived from the five colours above, so a theme that declares none still has a value for each.
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
    pub roles: Roles,
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
            roles: Roles::default(),
        }
    }
}

impl Tokens {
    /// Every colour role, declared or derived from the five core colours.
    pub fn resolved_roles(&self) -> ResolvedRoles {
        self.roles.resolve(roles::Core {
            primary: self.primary,
            secondary: self.secondary,
            background: self.background,
            border: self.border,
            muted: self.muted,
        })
    }

    /// The colour of the skin key `key`: the skin's, else the neutral default's, else opaque white. A component that
    /// draws in a theme colour reads it with this, so a skin built before the key existed still gives a colour.
    pub fn colour_in(skin: &Skin, key: &str) -> [u8; 4] {
        skin.colour(key).or_else(|| Self::default().colour_of(key)).unwrap_or([255, 255, 255, 255])
    }

    /// The colour these tokens give the skin key `key`, or `None` for a key that is not a colour token.
    pub fn colour_of(&self, key: &str) -> Option<[u8; 4]> {
        let roles = self.resolved_roles();
        let colour = match key {
            keys::PRIMARY => self.primary,
            keys::SECONDARY => self.secondary,
            keys::BACKGROUND => self.background,
            keys::BORDER => self.border,
            keys::MUTED => self.muted,
            keys::LINE => roles.line,
            keys::LINE_MINOR => roles.line_minor,
            keys::LINE_FAINT => roles.line_faint,
            keys::GUIDE => roles.guide,
            keys::SUCCESS => roles.success,
            keys::CAUTION => roles.caution,
            keys::DANGER => roles.danger,
            keys::INFO => roles.info,
            keys::SCRIM => roles.scrim,
            keys::OVERLAY => roles.overlay,
            _ => return None,
        };
        Some(colour)
    }

    /// The tokens as skin entries, under [`keys`].
    pub fn skin(&self) -> Skin {
        let roles = self.resolved_roles();
        Skin::new()
            .with_colour(keys::LINE, roles.line)
            .with_colour(keys::LINE_MINOR, roles.line_minor)
            .with_colour(keys::LINE_FAINT, roles.line_faint)
            .with_colour(keys::GUIDE, roles.guide)
            .with_colour(keys::SUCCESS, roles.success)
            .with_colour(keys::CAUTION, roles.caution)
            .with_colour(keys::DANGER, roles.danger)
            .with_colour(keys::INFO, roles.info)
            .with_colour(keys::SCRIM, roles.scrim)
            .with_colour(keys::OVERLAY, roles.overlay)
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
    use super::{Roles, Tokens, keys};
    use crate::skin::Skin;

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

    #[test]
    fn every_role_is_in_the_skin_whether_the_theme_declares_it_or_not() {
        let plain = Tokens { primary: [9, 9, 9, 255], ..Tokens::default() }.skin();
        assert_eq!(plain.colour(keys::LINE), Some([9, 9, 9, 255]), "derived from the primary colour");
        for key in [keys::LINE_MINOR, keys::LINE_FAINT, keys::GUIDE, keys::SUCCESS, keys::CAUTION, keys::DANGER] {
            assert!(plain.colour(key).is_some(), "{key}");
        }
        for key in [keys::INFO, keys::SCRIM, keys::OVERLAY] {
            assert!(plain.colour(key).is_some(), "{key}");
        }
        let set = Tokens { roles: Roles { guide: Some([1, 2, 3, 4]), ..Roles::default() }, ..Tokens::default() };
        assert_eq!(set.skin().colour(keys::GUIDE), Some([1, 2, 3, 4]));
    }

    #[test]
    fn a_skin_without_a_key_falls_back_to_the_neutral_default() {
        let empty = Skin::new();
        assert_eq!(Tokens::colour_in(&empty, keys::GUIDE), Tokens::default().secondary);
        assert_eq!(Tokens::colour_in(&empty, "not.a.token"), [255, 255, 255, 255]);
        let themed = Skin::new().with_colour(keys::GUIDE, [5, 6, 7, 8]);
        assert_eq!(Tokens::colour_in(&themed, keys::GUIDE), [5, 6, 7, 8]);
    }

    #[test]
    fn the_overlay_plate_is_stronger_than_the_scrim() {
        let skin = Tokens::default().skin();
        let (scrim, overlay) = (skin.colour(keys::SCRIM).unwrap(), skin.colour(keys::OVERLAY).unwrap());
        assert!(overlay[3] > scrim[3], "the overlay plate is stronger than the scrim");
    }
}

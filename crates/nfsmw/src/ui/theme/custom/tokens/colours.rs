//! The Custom HUD's colours (`docs/specs/custom-hud/visual-identity.md` sections 2 to 4): the brief's palette, the
//! high-contrast variant and the colour-vision palettes, as `[r, g, b, a]`.
//!
//! Order of application: the palette replaces the meaning hues, then high contrast replaces the neutrals and brightens
//! the hues. With the default palette the high-contrast hues are the spec's table; with a colour-vision palette they are
//! that palette's hues brightened by [`BRIGHTEN`], so the colours the palette keeps apart stay apart.

use crate::settings::hud::HudPalette;

use super::CustomLook;

/// How far high contrast moves a colour-vision palette's hue towards white.
pub const BRIGHTEN: f32 = 0.25;

/// One set of the Custom colours. `outline` is the plate outline, drawn only with high contrast.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Colours {
    pub text: [u8; 4],
    pub text_secondary: [u8; 4],
    pub surface: [u8; 4],
    pub surface_raised: [u8; 4],
    pub rule: [u8; 4],
    pub nav: [u8; 4],
    pub success: [u8; 4],
    pub caution: [u8; 4],
    pub danger: [u8; 4],
    pub critical: [u8; 4],
    pub critical_text: [u8; 4],
    pub outline: Option<[u8; 4]>,
}

/// The meaning hues a palette replaces.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Hues {
    nav: [u8; 4],
    success: [u8; 4],
    caution: [u8; 4],
    danger: [u8; 4],
    critical: [u8; 4],
}

/// The brief's hues (`text`, `text_secondary` and `danger` are its hex values), and the default palette's.
const DEFAULT_HUES: Hues = Hues {
    nav: [0x7F, 0xD8, 0xEC, 255],
    success: [0x5C, 0xC2, 0x8F, 255],
    caution: [0xE8, 0xB2, 0x4A, 255],
    danger: [0xE0, 0x52, 0x4D, 255],
    critical: [0xFF, 0xB0, 0x00, 255],
};

/// Red-green: the Okabe and Ito set, a cool white for the route.
const DEUTERANOPIA_HUES: Hues = Hues {
    nav: [0xF2, 0xF4, 0xF7, 255],
    success: [0x56, 0xB4, 0xE9, 255],
    caution: [0xE6, 0x9F, 0x00, 255],
    danger: [0xD5, 0x5E, 0x00, 255],
    critical: [0xF0, 0xE4, 0x42, 255],
};

/// Blue-yellow: the Okabe and Ito set, a cool white for the route; danger and critical keep the default hues.
const TRITANOPIA_HUES: Hues = Hues {
    nav: [0xF2, 0xF4, 0xF7, 255],
    success: [0x00, 0x9E, 0x73, 255],
    caution: [0xCC, 0x79, 0xA7, 255],
    danger: [0xE0, 0x52, 0x4D, 255],
    critical: [0xFF, 0xB0, 0x00, 255],
};

/// The default palette's hues under high contrast.
const HIGH_CONTRAST_HUES: Hues = Hues {
    nav: [0x8F, 0xE6, 0xFF, 255],
    success: [0x66, 0xE0, 0xA3, 255],
    caution: [0xFF, 0xD1, 0x66, 255],
    danger: [0xFF, 0x6B, 0x66, 255],
    critical: [0xFF, 0xD0, 0x00, 255],
};

impl Colours {
    /// The brief's palette: light text on a translucent charcoal plate.
    pub const BRIEF: Colours = Colours {
        text: [0xE8, 0xED, 0xF2, 255],
        text_secondary: [0x9B, 0xA7, 0xB3, 255],
        surface: [0x12, 0x16, 0x1B, 153],
        surface_raised: [0x1C, 0x22, 0x2A, 191],
        rule: [0xE8, 0xED, 0xF2, 46],
        nav: DEFAULT_HUES.nav,
        success: DEFAULT_HUES.success,
        caution: DEFAULT_HUES.caution,
        danger: DEFAULT_HUES.danger,
        critical: DEFAULT_HUES.critical,
        critical_text: [0x12, 0x16, 0x1B, 255],
        outline: None,
    };

    /// The colours `look` chooses: the palette, then high contrast.
    pub fn for_look(look: CustomLook) -> Self {
        let colours = Self::BRIEF.with_palette(look.palette);
        if !look.high_contrast {
            return colours;
        }
        colours.high_contrast(look.palette)
    }

    /// These colours with the meaning hues of `palette`. The neutrals do not change.
    pub fn with_palette(self, palette: HudPalette) -> Self {
        self.with_hues(hues(palette))
    }

    /// These colours under high contrast, for a set made with `palette`: the neutrals become white on near-black, a
    /// plate gets an outline, and the hues brighten (the spec's table for the default palette).
    pub fn high_contrast(self, palette: HudPalette) -> Self {
        let hues = match palette {
            HudPalette::Default => HIGH_CONTRAST_HUES,
            HudPalette::Deuteranopia | HudPalette::Tritanopia => brightened(hues(palette)),
        };
        Self {
            text: [0xFF, 0xFF, 0xFF, 255],
            text_secondary: [0xD5, 0xDD, 0xE5, 255],
            surface: [0x00, 0x00, 0x00, 217],
            surface_raised: [0x1A, 0x1A, 0x1A, 235],
            rule: [0xFF, 0xFF, 0xFF, 128],
            critical_text: [0x00, 0x00, 0x00, 255],
            outline: Some([0xFF, 0xFF, 0xFF, 153]),
            ..self
        }
        .with_hues(hues)
    }

    fn with_hues(self, hues: Hues) -> Self {
        Self {
            nav: hues.nav,
            success: hues.success,
            caution: hues.caution,
            danger: hues.danger,
            critical: hues.critical,
            ..self
        }
    }
}

impl Default for Colours {
    fn default() -> Self {
        Self::BRIEF
    }
}

fn hues(palette: HudPalette) -> Hues {
    match palette {
        HudPalette::Default => DEFAULT_HUES,
        HudPalette::Deuteranopia => DEUTERANOPIA_HUES,
        HudPalette::Tritanopia => TRITANOPIA_HUES,
    }
}

/// Every hue of `hues` moved [`BRIGHTEN`] of the way to white.
fn brightened(hues: Hues) -> Hues {
    Hues {
        nav: brighten(hues.nav),
        success: brighten(hues.success),
        caution: brighten(hues.caution),
        danger: brighten(hues.danger),
        critical: brighten(hues.critical),
    }
}

/// `rgba` moved [`BRIGHTEN`] of the way to white; the alpha is kept.
pub fn brighten([r, g, b, a]: [u8; 4]) -> [u8; 4] {
    let lift = |c: u8| (f32::from(c) + (255.0 - f32::from(c)) * BRIGHTEN).round() as u8;
    [lift(r), lift(g), lift(b), a]
}

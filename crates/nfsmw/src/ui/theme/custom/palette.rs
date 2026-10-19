//! The palettes the token-driven HUD takes its look from. The settings pick one (`ui_theme`): the Bevy palette is the
//! colours of the speedometer and the panels, and the Remastered palette is the gold accents of the remastered HUD.

use blackbox_ui::Tokens;

use crate::settings::UiTheme;

/// The corner radius of a palette's panels and bars, in FEng units.
pub const DEFAULT_RADIUS: f32 = 14.0;

/// The Bevy palette: a black panel at 170/255 alpha (the FPS counter's background), white digits and title, a cyan
/// accent for the rev gauge, the nitrous and the speed breaker, grey labels, and a dim rule.
pub fn bevy() -> Tokens {
    Tokens {
        primary: [255, 255, 255, 255],
        secondary: [120, 220, 255, 255],
        background: [0, 0, 0, 170],
        border: [255, 255, 255, 40],
        muted: [150, 160, 172, 255],
        radius: DEFAULT_RADIUS,
        padding: 3.0,
        text_scale: 1.0,
    }
}

/// The Remastered palette: white digits, the gold accent of the remastered HUD on a dark panel, and a grey for labels.
pub fn remastered() -> Tokens {
    Tokens {
        primary: [255, 255, 255, 255],
        secondary: [255, 200, 80, 255],
        background: [8, 8, 12, 180],
        border: [255, 255, 255, 80],
        muted: [170, 170, 170, 255],
        radius: DEFAULT_RADIUS,
        padding: 3.0,
        text_scale: 1.0,
    }
}

/// The palette the settings choose.
pub fn for_ui_theme(theme: UiTheme) -> Tokens {
    match theme {
        UiTheme::Remastered => remastered(),
        UiTheme::Bevy => bevy(),
    }
}

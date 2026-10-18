//! The palettes the token-driven HUD takes its look from. The settings pick one (`ui_theme`): the Bevy palette is the
//! colours of the speedometer and the panels, and the Remastered palette is the gold accents of the remastered HUD.

use blackbox_ui::Tokens;

use crate::settings::UiTheme;

/// The Bevy palette: a black panel at 170/255 alpha (the FPS counter's background), white digits and title, grey labels
/// and credit, and a dim rule. The corners and the card padding are the Custom card's own.
pub fn bevy() -> Tokens {
    Tokens {
        primary: [255, 255, 255, 255],
        secondary: [255, 200, 80, 255],
        background: [0, 0, 0, 170],
        border: [255, 255, 255, 40],
        muted: [180, 185, 195, 255],
        radius: 6.0,
        padding: 6.0,
    }
}

/// The Remastered palette: white digits, the gold accent of the remastered HUD on a dark panel, and a grey for labels.
pub fn remastered() -> Tokens {
    Tokens {
        primary: [255, 255, 255, 255],
        secondary: [255, 196, 64, 255],
        background: [0, 0, 0, 170],
        border: [255, 255, 255, 34],
        muted: [170, 170, 170, 255],
        radius: 4.0,
        padding: 6.0,
    }
}

/// The palette the settings choose.
pub fn for_ui_theme(theme: UiTheme) -> Tokens {
    match theme {
        UiTheme::Remastered => remastered(),
        UiTheme::Bevy => bevy(),
    }
}

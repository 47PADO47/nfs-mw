//! The palettes the token-driven HUD takes its look from. The settings pick one (`ui_theme`): the Bevy palette is the
//! colours of the speedometer and the panels, and the Remastered palette is the gold accents of the remastered HUD.

use blackbox_ui::{Roles, Tokens};

use super::tokens::{Colours, SIGNAL};
use crate::settings::UiTheme;

/// The corner radius of a palette's panels and bars, in FEng units.
pub const DEFAULT_RADIUS: f32 = 14.0;

/// The colour roles of the Custom look's two palettes: the map's linework in the brief's text tones, the meaning colours
/// of the brief's palette, and the brief's plates. `guide` is the route's colour of the palette (`guide` is the only
/// role the two palettes differ in).
fn roles(guide: [u8; 4]) -> Roles {
    let brief = Colours::BRIEF;
    Roles {
        line: Some(brief.text),
        line_minor: Some(brief.text_secondary),
        line_faint: Some(brief.rule),
        guide: Some(guide),
        success: Some(brief.success),
        caution: Some(brief.caution),
        danger: Some(brief.danger),
        info: Some(INFO),
        scrim: Some(SCRIM),
        overlay: Some(OVERLAY),
    }
}

/// The cool blue of the palette's second signal (a car lot, the alternate flash of a light bar).
pub const INFO: [u8; 4] = [0x5A, 0x96, 0xFF, 255];
/// The scrim: the graphite of the brief's plate, dark and translucent, over the whole scene behind a modal map.
pub const SCRIM: [u8; 4] = [0x06, 0x09, 0x0D, 168];
/// The overlay: the same graphite, nearly opaque, as the plate the full map's picture lies on.
pub const OVERLAY: [u8; 4] = [0x10, 0x16, 0x1D, 232];

/// The Bevy palette (the one palette that shows the pictures of menu entries): a black panel at 170/255 alpha (the FPS counter's background), white digits and title, a cyan
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
        icons: true,
        roles: roles(SIGNAL),
    }
}

/// The gold of the Remastered palette's accent, which is also its guide path.
const GOLD: [u8; 4] = [255, 200, 80, 255];

/// The Remastered palette: white digits, the gold accent of the remastered HUD on a dark panel, and a grey for labels.
pub fn remastered() -> Tokens {
    Tokens {
        primary: [255, 255, 255, 255],
        secondary: GOLD,
        background: [8, 8, 12, 180],
        border: [255, 255, 255, 80],
        muted: [170, 170, 170, 255],
        radius: DEFAULT_RADIUS,
        padding: 3.0,
        text_scale: 1.0,
        icons: false,
        roles: roles(GOLD),
    }
}

/// The palette the settings choose.
pub fn for_ui_theme(theme: UiTheme) -> Tokens {
    match theme {
        UiTheme::Remastered => remastered(),
        UiTheme::Bevy => bevy(),
    }
}

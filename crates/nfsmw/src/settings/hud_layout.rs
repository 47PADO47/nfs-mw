//! The HUD's placement policy. Rendering is defined in `docs/specs/hud-viewport.md`.

use std::{fmt, str::FromStr};

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum HudLayout {
    /// The PC HUD, with the wide layout on wide canvases.
    #[default]
    Pc,
    /// Keep the authored 4:3 HUD centred.
    Classic,
    /// The wide HUD at the Xbox 360's smaller presentation scale.
    Xbox360,
}

impl FromStr for HudLayout {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.trim().to_ascii_lowercase().as_str() {
            "pc" => Ok(Self::Pc),
            "classic" => Ok(Self::Classic),
            "xbox360" => Ok(Self::Xbox360),
            _ => Err(format!("expected pc, classic or xbox360, got {s:?}")),
        }
    }
}

impl fmt::Display for HudLayout {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Pc => "pc",
            Self::Classic => "classic",
            Self::Xbox360 => "xbox360",
        })
    }
}

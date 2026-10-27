//! Colourful tire smoke: how the smoke is coloured (`smoke_color_mode`) and with what (`smoke_color`).
//!
//! `Off` keeps the smoke as the game draws it (white), whichever the colour. The colour is one of a small palette, by name,
//! or any colour as `#rrggbb` in the config file; the menu steps through the palette (and starts from the nearest name when
//! the file holds a custom colour). Both are read by the tire smoke of the game and by the tire smoke variants of mods
//! (`docs/tire-effects.md`).

use std::fmt;
use std::str::FromStr;

/// How the smoke is coloured.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum SmokeColorMode {
    /// The game's own smoke colour.
    #[default]
    Off,
    /// Every puff is the chosen colour.
    Solid,
    /// The hue cycles with time and along the trail, whatever the chosen colour.
    Rainbow,
    /// The smoke goes from its own colour to the chosen one as the car speeds up.
    Speed,
    /// The smoke goes from its own colour to the chosen one as the tyre slips more.
    Slip,
}

impl SmokeColorMode {
    /// Every mode, in the order the menu steps through them.
    pub const ALL: [SmokeColorMode; 5] = [Self::Off, Self::Solid, Self::Rainbow, Self::Speed, Self::Slip];

    /// The next mode, wrapping.
    pub fn next(self) -> Self {
        let at = Self::ALL.iter().position(|mode| *mode == self).unwrap_or(0);
        Self::ALL[(at + 1) % Self::ALL.len()]
    }

    /// What the menu shows.
    pub fn label(self) -> &'static str {
        match self {
            Self::Off => "Off",
            Self::Solid => "Solid Colour",
            Self::Rainbow => "Rainbow",
            Self::Speed => "By Speed",
            Self::Slip => "By Slip",
        }
    }
}

impl FromStr for SmokeColorMode {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, String> {
        match s {
            "off" => Ok(Self::Off),
            "solid" => Ok(Self::Solid),
            "rainbow" => Ok(Self::Rainbow),
            "speed" => Ok(Self::Speed),
            "slip" => Ok(Self::Slip),
            _ => Err(format!("expected off, solid, rainbow, speed or slip, got {s:?}")),
        }
    }
}

impl fmt::Display for SmokeColorMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Off => "off",
            Self::Solid => "solid",
            Self::Rainbow => "rainbow",
            Self::Speed => "speed",
            Self::Slip => "slip",
        })
    }
}

/// How the smoke is coloured, as the scenes take it: the mode and the colour it uses.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct SmokeTint {
    pub mode: SmokeColorMode,
    pub color: SmokeColor,
}

/// The palette: a name and a colour each. The first is the default.
pub const PALETTE: [(&str, [u8; 3]); 8] = [
    ("red", [235, 40, 40]),
    ("orange", [255, 140, 20]),
    ("yellow", [250, 220, 30]),
    ("green", [50, 210, 70]),
    ("cyan", [40, 210, 230]),
    ("blue", [50, 90, 255]),
    ("purple", [150, 70, 235]),
    ("pink", [255, 90, 190]),
];

/// The colour of the smoke.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SmokeColor(pub [u8; 3]);

impl Default for SmokeColor {
    fn default() -> Self {
        Self(PALETTE[0].1)
    }
}

impl SmokeColor {
    /// The palette name of this colour, if it is one.
    pub fn name(&self) -> Option<&'static str> {
        PALETTE.iter().find(|(_, rgb)| *rgb == self.0).map(|(name, _)| *name)
    }

    /// The next palette colour; a custom colour goes to the one after the palette colour nearest to it. Wraps.
    pub fn next(self) -> Self {
        let at = PALETTE.iter().position(|(_, rgb)| *rgb == self.0).unwrap_or_else(|| self.nearest());
        Self(PALETTE[(at + 1) % PALETTE.len()].1)
    }

    /// The index of the palette colour nearest to this one.
    fn nearest(&self) -> usize {
        let distance =
            |rgb: &[u8; 3]| -> i32 { (0..3).map(|i| (i32::from(rgb[i]) - i32::from(self.0[i])).pow(2)).sum() };
        (0..PALETTE.len()).min_by_key(|&at| distance(&PALETTE[at].1)).unwrap_or(0)
    }

    /// What the menu shows: the name of a palette colour, else the hex value.
    pub fn label(&self) -> String {
        match self.name() {
            Some(name) => {
                let mut chars = name.chars();
                chars.next().map(|first| first.to_ascii_uppercase().to_string() + chars.as_str()).unwrap_or_default()
            }
            None => self.to_string(),
        }
    }
}

impl FromStr for SmokeColor {
    type Err = String;

    /// A palette name (`orange`), or `#rrggbb` (the `#` may be left out), upper or lower case.
    fn from_str(s: &str) -> Result<Self, String> {
        let lower = s.trim().to_ascii_lowercase();
        if let Some((_, rgb)) = PALETTE.iter().find(|(name, _)| *name == lower) {
            return Ok(Self(*rgb));
        }
        let digits = lower.strip_prefix('#').unwrap_or(&lower);
        let bad = || format!("expected a colour name ({}) or #rrggbb, got {s:?}", palette_names());
        if digits.len() != 6 || !digits.bytes().all(|b| b.is_ascii_hexdigit()) {
            return Err(bad());
        }
        let byte = |at: usize| u8::from_str_radix(&digits[at..at + 2], 16).map_err(|_| bad());
        Ok(Self([byte(0)?, byte(2)?, byte(4)?]))
    }
}

impl fmt::Display for SmokeColor {
    /// The palette name, or `#rrggbb`.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.name() {
            Some(name) => f.write_str(name),
            None => write!(f, "#{:02x}{:02x}{:02x}", self.0[0], self.0[1], self.0[2]),
        }
    }
}

fn palette_names() -> String {
    PALETTE.iter().map(|(name, _)| *name).collect::<Vec<_>>().join(", ")
}

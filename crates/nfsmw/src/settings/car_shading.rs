//! Selectable original car material families and retained custom/low-cost shading.

use std::fmt;
use std::str::FromStr;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum CarShading {
    /// One directional light and ambient, as the world is lit: the cheap path for weak GPUs.
    Simple,
    /// The glossy car shader of docs/specs/car-assembly.md section 8.
    #[default]
    Glossy,
    /// Original PC material/light arithmetic used with shadow quality enabled.
    /// Original projected shadow generation remains incomplete.
    Original,
    /// Original PC shadowless material family, including its different reflection/highlight gain.
    OriginalUnshadowed,
}

impl FromStr for CarShading {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, String> {
        match s {
            "simple" => Ok(Self::Simple),
            "glossy" | "enhanced" => Ok(Self::Glossy),
            "original" => Ok(Self::Original),
            "original-unshadowed" => Ok(Self::OriginalUnshadowed),
            _ => Err(format!("expected enhanced, glossy, simple, original or original-unshadowed, got {s:?}")),
        }
    }
}

impl fmt::Display for CarShading {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Simple => "simple",
            Self::Glossy => "glossy",
            Self::Original => "original",
            Self::OriginalUnshadowed => "original-unshadowed",
        })
    }
}

impl CarShading {
    pub const ALL: [Self; 4] = [Self::Simple, Self::Glossy, Self::Original, Self::OriginalUnshadowed];
    /// The shading's English name, for the F8 graphics lab (which stays in English).
    pub fn label(self) -> &'static str {
        match self {
            Self::Simple => "Simple",
            Self::Glossy => "Enhanced",
            Self::Original => "Original PC",
            Self::OriginalUnshadowed => "Original PC (shadowless)",
        }
    }
    /// The shading's name in the current language, as the options show it.
    pub fn menu_label(self) -> String {
        let id = match self {
            Self::Simple => "options-car-shading-simple",
            Self::Glossy => "options-enhanced",
            Self::Original => "options-original-pc",
            Self::OriginalUnshadowed => "options-car-shading-shadowless",
        };
        crate::i18n::text(id, &crate::i18n::Args::new())
    }
    pub fn is_original(self) -> bool {
        matches!(self, Self::Original | Self::OriginalUnshadowed)
    }
}

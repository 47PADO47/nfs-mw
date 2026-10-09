//! How cars are shaded: the glossy three-light rig with sun highlight and environment reflection
//! (the default), or the single-light shading everything else in the scene uses. `simple` builds none of
//! the glossy resources (docs/low-end.md).

use std::fmt;
use std::str::FromStr;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum CarShading {
    /// One directional light and ambient, as the world is lit: the cheap path for weak GPUs.
    Simple,
    /// The glossy car shader of docs/specs/car-assembly.md section 8.
    #[default]
    Glossy,
}

impl FromStr for CarShading {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, String> {
        match s {
            "simple" => Ok(Self::Simple),
            "glossy" => Ok(Self::Glossy),
            _ => Err(format!("expected simple or glossy, got {s:?}")),
        }
    }
}

impl fmt::Display for CarShading {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Simple => "simple",
            Self::Glossy => "glossy",
        })
    }
}

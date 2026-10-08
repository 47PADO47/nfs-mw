//! The optional tire-smoke presentation quality; physics and the smoke switch are independent.

use std::fmt;
use std::str::FromStr;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum SmokeQuality {
    #[default]
    Standard,
    High,
}

impl FromStr for SmokeQuality {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, String> {
        match s {
            "standard" => Ok(Self::Standard),
            "high" => Ok(Self::High),
            _ => Err(format!("expected standard or high, got {s:?}")),
        }
    }
}

impl fmt::Display for SmokeQuality {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Standard => "standard",
            Self::High => "high",
        })
    }
}

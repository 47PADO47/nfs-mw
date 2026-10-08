//! The transmission setting: who changes gear (docs/specs/vehicle-manual-shifting.md, section 1).

use std::fmt;
use std::str::FromStr;

/// Whether the gearbox shifts by itself. The original keeps this as a player setting (0 automatic, 1 manual) and
/// starts automatic.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum Transmission {
    /// The box shifts by itself; the shift buttons are "sport shifts".
    #[default]
    Automatic,
    /// Only the player changes gear.
    Manual,
}

impl Transmission {
    /// The other value, for a toggle row.
    pub fn other(self) -> Self {
        match self {
            Self::Automatic => Self::Manual,
            Self::Manual => Self::Automatic,
        }
    }

    /// The gearbox shifts by itself (what `blackbox-vehicle` calls `automatic`).
    pub fn is_automatic(self) -> bool {
        self == Self::Automatic
    }
}

impl FromStr for Transmission {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.trim().to_ascii_lowercase().as_str() {
            "automatic" | "auto" | "0" => Ok(Self::Automatic),
            "manual" | "1" => Ok(Self::Manual),
            _ => Err(format!("expected automatic or manual, got {s:?}")),
        }
    }
}

impl fmt::Display for Transmission {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Automatic => "automatic",
            Self::Manual => "manual",
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_names_and_the_originals_numbers() {
        assert_eq!("manual".parse(), Ok(Transmission::Manual));
        assert_eq!(" Auto ".parse(), Ok(Transmission::Automatic));
        assert_eq!("1".parse(), Ok(Transmission::Manual));
        assert_eq!("0".parse(), Ok(Transmission::Automatic));
        assert!("sport".parse::<Transmission>().is_err());
    }

    #[test]
    fn displays_what_it_parses() {
        for t in [Transmission::Automatic, Transmission::Manual] {
            assert_eq!(t.to_string().parse(), Ok(t));
        }
        assert_eq!(Transmission::default(), Transmission::Automatic, "as in the original");
        assert_eq!(Transmission::Automatic.other(), Transmission::Manual);
    }
}

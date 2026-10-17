//! Scene-surface sampling quality, independent of post effects and lookup tables.
use std::{fmt, str::FromStr};

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub enum TextureFiltering {
    Trilinear,
    #[default]
    Anisotropic8,
    Anisotropic16,
}
impl TextureFiltering {
    pub const ALL: [Self; 3] = [Self::Trilinear, Self::Anisotropic8, Self::Anisotropic16];
    pub const fn anisotropy(self) -> u16 {
        match self {
            Self::Trilinear => 1,
            Self::Anisotropic8 => 8,
            Self::Anisotropic16 => 16,
        }
    }
    pub const fn label(self) -> &'static str {
        match self {
            Self::Trilinear => "Trilinear",
            Self::Anisotropic8 => "Anisotropic 8x",
            Self::Anisotropic16 => "Anisotropic 16x",
        }
    }
    pub const fn name(self) -> &'static str {
        match self {
            Self::Trilinear => "trilinear",
            Self::Anisotropic8 => "anisotropic-8x",
            Self::Anisotropic16 => "anisotropic-16x",
        }
    }
    pub fn supported(self, maximum: u16) -> Self {
        if self.anisotropy() <= maximum {
            return self;
        }
        if maximum >= 8 {
            return Self::Anisotropic8;
        }
        Self::Trilinear
    }
}
impl fmt::Display for TextureFiltering {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.name())
    }
}
impl FromStr for TextureFiltering {
    type Err = String;
    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value.trim().to_ascii_lowercase().as_str() {
            "trilinear" => Ok(Self::Trilinear),
            "8x" | "anisotropic-8x" => Ok(Self::Anisotropic8),
            "16x" | "anisotropic-16x" => Ok(Self::Anisotropic16),
            _ => Err("expected trilinear, anisotropic-8x or anisotropic-16x".into()),
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn persistence_round_trips_and_downlevels_explicitly() {
        for value in TextureFiltering::ALL {
            assert_eq!(value.to_string().parse(), Ok(value));
        }
        assert_eq!(TextureFiltering::Anisotropic16.supported(8), TextureFiltering::Anisotropic8);
        assert_eq!(TextureFiltering::Anisotropic8.supported(1), TextureFiltering::Trilinear);
        assert!("4x".parse::<TextureFiltering>().is_err());
    }
}

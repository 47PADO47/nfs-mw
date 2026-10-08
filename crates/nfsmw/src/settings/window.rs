//! Window preferences, shared by the config, CLI and console.

use std::fmt;
use std::str::FromStr;

/// The application window's mode (the renderer follows its physical client size).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum WindowMode {
    #[default]
    Windowed,
    Borderless,
    Exclusive,
}

impl FromStr for WindowMode {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, String> {
        match s.trim().to_ascii_lowercase().as_str() {
            "windowed" => Ok(Self::Windowed),
            "borderless" => Ok(Self::Borderless),
            "exclusive" => Ok(Self::Exclusive),
            _ => Err(format!("expected windowed, borderless or exclusive, got {s:?}")),
        }
    }
}

impl fmt::Display for WindowMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Windowed => "windowed",
            Self::Borderless => "borderless",
            Self::Exclusive => "exclusive",
        })
    }
}

/// A monitor in the window backend's enumeration; indices are zero-based.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum Monitor {
    #[default]
    Current,
    Primary,
    Index(usize),
}

impl FromStr for Monitor {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, String> {
        match s.trim().to_ascii_lowercase().as_str() {
            "current" => Ok(Self::Current),
            "primary" => Ok(Self::Primary),
            other => other
                .parse::<usize>()
                .map(Self::Index)
                .map_err(|_| format!("expected current, primary or a zero-based monitor index, got {s:?}")),
        }
    }
}

impl fmt::Display for Monitor {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Current => f.write_str("current"),
            Self::Primary => f.write_str("primary"),
            Self::Index(index) => write!(f, "{index}"),
        }
    }
}

/// Physical client pixels in windowed mode, a supported video mode in exclusive mode.
/// `Native` uses the default window size / the monitor's desktop video mode.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum Resolution {
    #[default]
    Native,
    Pixels {
        width: u32,
        height: u32,
    },
}

impl Resolution {
    pub fn pixels(width: u32, height: u32) -> Result<Self, String> {
        if !(160..=16384).contains(&width) || !(160..=16384).contains(&height) {
            return Err("the width and height must be between 160 and 16384".into());
        }
        Ok(Self::Pixels { width, height })
    }
}

impl FromStr for Resolution {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, String> {
        let text = s.trim();
        if text.eq_ignore_ascii_case("native") {
            return Ok(Self::Native);
        }
        let (w, h) = text.split_once(['x', 'X']).ok_or("expected native or WIDTHxHEIGHT")?;
        let number = |v: &str| v.trim().parse::<u32>().map_err(|_| "expected native or WIDTHxHEIGHT".to_owned());
        Self::pixels(number(w)?, number(h)?)
    }
}

impl fmt::Display for Resolution {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Native => f.write_str("native"),
            Self::Pixels { width, height } => write!(f, "{width}x{height}"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn values_round_trip_and_reject_bad_input() {
        for text in ["windowed", "borderless", "exclusive"] {
            assert_eq!(text.parse::<WindowMode>().unwrap().to_string(), text);
        }
        for text in ["current", "primary", "0", "3"] {
            assert_eq!(text.parse::<Monitor>().unwrap().to_string(), text);
        }
        for text in ["native", "1920x1080", "160x16384"] {
            assert_eq!(text.parse::<Resolution>().unwrap().to_string(), text);
        }
        assert_eq!("1920X1080".parse::<Resolution>().unwrap().to_string(), "1920x1080");
        assert!("fullscreen".parse::<WindowMode>().is_err());
        assert!("-1".parse::<Monitor>().is_err());
        for text in ["10x10", "16385x1080", "0x0", "1920", "1920x1080x60"] {
            assert!(text.parse::<Resolution>().is_err(), "{text}");
        }
    }
}

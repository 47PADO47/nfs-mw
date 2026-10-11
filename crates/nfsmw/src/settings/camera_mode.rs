//! The camera setting: which camera gameplay uses. Only cameras that report `selectable()` (see
//! [`crate::viewer::camera::Camera`]) are listed here; the free and orbit cameras have their own toggles.

use std::fmt;
use std::str::FromStr;

/// The cameras gameplay can use. Each variant must be a camera whose `selectable()` is true.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum CameraMode {
    /// Behind the car, following it.
    #[default]
    Chase,
    /// Behind the car, with the view widening as it speeds up.
    DynamicFov,
}

impl CameraMode {
    /// Every choosable camera, in the order the options row cycles through them.
    pub const ALL: [Self; 2] = [Self::Chase, Self::DynamicFov];
}

impl FromStr for CameraMode {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.trim().to_ascii_lowercase().as_str() {
            "chase" => Ok(Self::Chase),
            "dynamic-fov" | "dynamic_fov" => Ok(Self::DynamicFov),
            _ => Err(format!("expected chase or dynamic-fov, got {s:?}")),
        }
    }
}

impl fmt::Display for CameraMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Chase => "chase",
            Self::DynamicFov => "dynamic-fov",
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_and_displays_names() {
        assert_eq!("chase".parse(), Ok(CameraMode::Chase));
        assert_eq!(" Chase ".parse(), Ok(CameraMode::Chase));
        assert_eq!("dynamic_fov".parse(), Ok(CameraMode::DynamicFov));
        assert!("orbit".parse::<CameraMode>().is_err());
        for mode in CameraMode::ALL {
            assert_eq!(mode.to_string().parse(), Ok(mode));
        }
        assert_eq!(CameraMode::default(), CameraMode::Chase);
    }
}

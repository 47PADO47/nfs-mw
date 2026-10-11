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
    /// The original game's views, from its `camerainfo` data (docs/specs/camera-views.md).
    Bumper,
    Hood,
    Close,
    Far,
    SuperFar,
    Drift,
    Pursuit,
}

impl CameraMode {
    /// Every choosable camera, in the order the options row cycles through them.
    pub const ALL: [Self; 9] = [
        Self::Chase,
        Self::DynamicFov,
        Self::Bumper,
        Self::Hood,
        Self::Close,
        Self::Far,
        Self::SuperFar,
        Self::Drift,
        Self::Pursuit,
    ];

    /// The next choosable camera after this one, wrapping round.
    pub fn next(self) -> Self {
        let at = Self::ALL.iter().position(|mode| *mode == self).unwrap_or(0);
        Self::ALL[(at + 1) % Self::ALL.len()]
    }
}

impl FromStr for CameraMode {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.trim().to_ascii_lowercase().as_str() {
            "chase" => Ok(Self::Chase),
            "dynamic-fov" | "dynamic_fov" => Ok(Self::DynamicFov),
            "bumper" => Ok(Self::Bumper),
            "hood" => Ok(Self::Hood),
            "close" => Ok(Self::Close),
            "far" => Ok(Self::Far),
            "superfar" | "super-far" | "super_far" => Ok(Self::SuperFar),
            "drift" => Ok(Self::Drift),
            "pursuit" => Ok(Self::Pursuit),
            _ => Err(format!(
                "expected chase, dynamic-fov, bumper, hood, close, far, superfar, drift or pursuit, got {s:?}"
            )),
        }
    }
}

impl fmt::Display for CameraMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Chase => "chase",
            Self::DynamicFov => "dynamic-fov",
            Self::Bumper => "bumper",
            Self::Hood => "hood",
            Self::Close => "close",
            Self::Far => "far",
            Self::SuperFar => "superfar",
            Self::Drift => "drift",
            Self::Pursuit => "pursuit",
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
        assert_eq!("super-far".parse(), Ok(CameraMode::SuperFar));
        assert!("orbit".parse::<CameraMode>().is_err());
        for mode in CameraMode::ALL {
            assert_eq!(mode.to_string().parse(), Ok(mode));
        }
        assert_eq!(CameraMode::default(), CameraMode::Chase);
    }

    #[test]
    fn the_options_row_cycles_through_every_camera_and_wraps() {
        let mut mode = CameraMode::default();
        let mut seen = vec![mode];
        for _ in 1..CameraMode::ALL.len() {
            mode = mode.next();
            seen.push(mode);
        }
        assert_eq!(seen, CameraMode::ALL);
        assert_eq!(mode.next(), CameraMode::Chase);
    }
}

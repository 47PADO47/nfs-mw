//! The camera setting: which camera gameplay uses. Only cameras that report `selectable()` (see
//! [`crate::viewer::camera::Camera`]) are listed here; the free and orbit cameras have their own toggles.

use std::cell::RefCell;
use std::collections::HashMap;
use std::fmt;
use std::str::FromStr;

/// An extra camera view a mod registered, named by a hash of `<mod id>/<view id>` so that it is `Copy`, survives a change in
/// the order mods load, and parses from the config without the mod being loaded.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ViewKey(u32);

thread_local! {
    /// The `<mod id>/<view id>` of every key seen this run, so a key can be written back as its name.
    static NAMES: RefCell<HashMap<u32, String>> = RefCell::new(HashMap::new());
}

impl ViewKey {
    /// The key of the view `view` of mod `owner`. Remembers the name for [`ViewKey::path`].
    pub fn of(owner: &str, view: &str) -> Self {
        Self::from_path(&format!("{owner}/{view}"))
    }

    /// The key of a `<mod id>/<view id>` path. Remembers the path for [`ViewKey::path`].
    pub fn from_path(path: &str) -> Self {
        let key = Self(fnv1a(path.to_ascii_lowercase().as_bytes()));
        NAMES.with(|names| {
            names.borrow_mut().entry(key.0).or_insert_with(|| path.to_owned());
        });
        key
    }

    /// The `<mod id>/<view id>` path, when the key was made from one this run.
    pub fn path(self) -> Option<String> {
        NAMES.with(|names| names.borrow().get(&self.0).cloned())
    }
}

fn fnv1a(bytes: &[u8]) -> u32 {
    bytes.iter().fold(0x811C_9DC5, |hash, byte| (hash ^ u32::from(*byte)).wrapping_mul(0x0100_0193))
}

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
    /// A view a mod registered (`bb.camera` 1.1). Not in [`CameraMode::ALL`]: which views exist depends on the mods.
    Extra(ViewKey),
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
            other => match other.strip_prefix("extra:") {
                Some(path) if path.contains('/') => Ok(Self::Extra(ViewKey::from_path(path))),
                _ => Err(format!(
                    "expected chase, dynamic-fov, bumper, hood, close, far, superfar, drift, pursuit or extra:<mod>/<view>, got {s:?}"
                )),
            },
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
            // Every extra key was made from its path in this process; the fallback keeps the file parsable.
            Self::Extra(key) => return write!(f, "extra:{}", key.path().unwrap_or_else(|| format!("unknown/{:08x}", key.0))),
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
    fn extra_views_parse_by_path_and_display_back() {
        let mode: CameraMode = "extra:Nfsmw-2012-Content/chase".parse().unwrap();
        assert_eq!(mode, CameraMode::Extra(ViewKey::of("nfsmw-2012-content", "chase")));
        assert_eq!(mode.to_string(), "extra:nfsmw-2012-content/chase");
        assert!("extra:nopath".parse::<CameraMode>().is_err());
        assert_ne!(ViewKey::of("a", "b"), ViewKey::of("a", "c"));
        assert!(!CameraMode::ALL.iter().any(|mode| matches!(mode, CameraMode::Extra(_))));
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

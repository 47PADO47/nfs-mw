//! One layer of settings: every field may be unset.

use blackbox_render::Backend;

use crate::app::pacing::MaxFps;

/// The settings one source (command line, environment, config file) sets. Unset fields fall
/// through to the next layer.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Partial {
    pub backend: Option<Backend>,
    pub vsync: Option<bool>,
    pub max_fps: Option<MaxFps>,
}

impl Partial {
    /// This layer, with `lower` filling the fields it leaves unset.
    pub fn or(self, lower: Partial) -> Partial {
        Partial {
            backend: self.backend.or(lower.backend),
            vsync: self.vsync.or(lower.vsync),
            max_fps: self.max_fps.or(lower.max_fps),
        }
    }
}

/// Parse a boolean written as `1/0`, `true/false`, `on/off` or `yes/no`.
pub fn parse_bool(s: &str) -> Result<bool, String> {
    match s.trim().to_ascii_lowercase().as_str() {
        "1" | "true" | "on" | "yes" => Ok(true),
        "0" | "false" | "off" | "no" => Ok(false),
        _ => Err(format!("expected on or off, got {s:?}")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn upper_layer_wins_and_gaps_fall_through() {
        let cli = Partial { backend: Some(Backend::Gl), ..Partial::default() };
        let file = Partial { backend: Some(Backend::Vulkan), vsync: Some(false), max_fps: None };
        let merged = cli.or(file);
        assert_eq!(merged.backend, Some(Backend::Gl));
        assert_eq!(merged.vsync, Some(false));
        assert_eq!(merged.max_fps, None);
    }

    #[test]
    fn booleans() {
        assert_eq!(parse_bool("On"), Ok(true));
        assert_eq!(parse_bool("0"), Ok(false));
        assert!(parse_bool("maybe").is_err());
    }
}

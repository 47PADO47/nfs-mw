//! Settings from environment variables.

use std::str::FromStr;

use super::partial::{Partial, parse_bool};
use crate::app::pacing::MaxFps;
use crate::devtools::ShowMetrics;

pub const BACKEND: &str = "NFSMW_BACKEND";
pub const VSYNC: &str = "NFSMW_VSYNC";
pub const MAX_FPS: &str = "NFSMW_MAX_FPS";
pub const SHOW_METRICS: &str = "NFSMW_SHOW_METRICS";

/// Read the layer through `get`, so tests need not touch the process environment. A value that
/// does not parse is reported and ignored.
pub fn read(get: impl Fn(&str) -> Option<String>) -> Partial {
    Partial {
        backend: value(&get, BACKEND, |s| s.parse().map_err(|e| format!("{e}"))),
        vsync: value(&get, VSYNC, parse_bool),
        max_fps: value(&get, MAX_FPS, MaxFps::from_str),
        show_metrics: value(&get, SHOW_METRICS, ShowMetrics::from_str),
    }
}

fn value<T>(get: &impl Fn(&str) -> Option<String>, name: &str, parse: impl Fn(&str) -> Result<T, String>) -> Option<T> {
    let text = get(name)?;
    match parse(&text) {
        Ok(v) => Some(v),
        Err(e) => {
            log::warn!("ignoring {name}: {e}");
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use blackbox_render::Backend;

    use super::*;

    fn layer(pairs: &[(&str, &str)]) -> Partial {
        let map: HashMap<_, _> = pairs.iter().map(|(k, v)| (k.to_string(), v.to_string())).collect();
        read(|name| map.get(name).cloned())
    }

    #[test]
    fn reads_every_setting() {
        let p = layer(&[(BACKEND, "dx12"), (VSYNC, "off"), (MAX_FPS, "60"), (SHOW_METRICS, "advanced")]);
        assert_eq!(p.show_metrics, Some(ShowMetrics::Advanced));
        assert_eq!(p.backend, Some(Backend::Dx12));
        assert_eq!(p.vsync, Some(false));
        assert_eq!(p.max_fps, Some("60".parse::<MaxFps>().unwrap()));
    }

    #[test]
    fn unset_and_invalid_values_stay_unset() {
        assert_eq!(layer(&[]), Partial::default());
        assert_eq!(layer(&[(BACKEND, "dx11"), (VSYNC, "maybe"), (MAX_FPS, "fast")]), Partial::default());
    }
}

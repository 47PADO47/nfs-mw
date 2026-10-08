//! Settings from environment variables.

use std::str::FromStr;

use super::partial::{Partial, Percent, parse_bool};
use crate::app::pacing::MaxFps;
use crate::devtools::{ShowMetrics, ShowReadout};

pub const BACKEND: &str = "NFSMW_BACKEND";
pub const VSYNC: &str = "NFSMW_VSYNC";
pub const MAX_FPS: &str = "NFSMW_MAX_FPS";
pub const SHOW_METRICS: &str = "NFSMW_SHOW_METRICS";
pub const SHOW_READOUT: &str = "NFSMW_SHOW_READOUT";
pub const MASTER_VOLUME: &str = "NFSMW_MASTER_VOLUME";
pub const MUSIC_VOLUME: &str = "NFSMW_MUSIC_VOLUME";
pub const SFX_VOLUME: &str = "NFSMW_SFX_VOLUME";
pub const ENGINE_VOLUME: &str = "NFSMW_ENGINE_VOLUME";
pub const HUD: &str = "NFSMW_HUD";

/// Read the layer through `get`, so tests need not touch the process environment. A value that
/// does not parse is reported and ignored.
pub fn read(get: impl Fn(&str) -> Option<String>) -> Partial {
    Partial {
        backend: value(&get, BACKEND, |s| s.parse().map_err(|e| format!("{e}"))),
        vsync: value(&get, VSYNC, parse_bool),
        max_fps: value(&get, MAX_FPS, MaxFps::from_str),
        show_metrics: value(&get, SHOW_METRICS, ShowMetrics::from_str),
        show_readout: value(&get, SHOW_READOUT, ShowReadout::from_str),
        master_volume: value(&get, MASTER_VOLUME, Percent::from_str),
        music_volume: value(&get, MUSIC_VOLUME, Percent::from_str),
        sfx_volume: value(&get, SFX_VOLUME, Percent::from_str),
        engine_volume: value(&get, ENGINE_VOLUME, Percent::from_str),
        hud: value(&get, HUD, parse_bool),
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
    fn reads_the_readout_level() {
        assert_eq!(layer(&[(SHOW_READOUT, "full")]).show_readout, Some(ShowReadout::Full));
        assert_eq!(layer(&[(SHOW_READOUT, "loud")]).show_readout, None);
    }

    #[test]
    fn reads_the_volumes() {
        let p = layer(&[(MASTER_VOLUME, "50"), (MUSIC_VOLUME, "20%"), (SFX_VOLUME, "loud")]);
        assert_eq!((p.master_volume, p.music_volume), (Some(Percent(50)), Some(Percent(20))));
        assert_eq!((p.sfx_volume, p.engine_volume), (None, None));
    }

    #[test]
    fn unset_and_invalid_values_stay_unset() {
        assert_eq!(layer(&[]), Partial::default());
        assert_eq!(layer(&[(BACKEND, "dx11"), (VSYNC, "maybe"), (MAX_FPS, "fast")]), Partial::default());
    }
}

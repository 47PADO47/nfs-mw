//! Offline comparison cadence. Spec: `docs/specs/shared-physics-clock.md`.

pub const DEFAULT_PHYSICS_RATE_HZ: u32 = 60;

pub(super) fn resolve(rate: Option<u32>) -> u32 {
    rate.filter(|rate| matches!(rate, 60 | 144)).unwrap_or(DEFAULT_PHYSICS_RATE_HZ)
}

/// Only the original cadence and the measured comparison cadence are supported.
pub fn parse_physics_rate(text: &str) -> Result<u32, String> {
    match text.trim() {
        "60" => Ok(60),
        "144" => Ok(144),
        _ => Err(format!("expected physics update rate 60 or 144, got {text:?}")),
    }
}

pub fn label(rate: u32) -> &'static str {
    match rate {
        144 => "144 Hz (comparison)",
        _ => "60 Hz (original)",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::settings::{Partial, Settings, WorldTuning, env, file, write};

    #[test]
    fn physics_rate_is_independent_of_render_frame_limit_and_defaults_to_sixty() {
        let settings = Settings::from(Partial::default());
        assert_eq!(settings.physics_rate_hz, 60);
        assert_eq!(WorldTuning::default().physics_rate_hz, 60);
        let changed = Settings::from(Partial {
            physics_rate_hz: Some(144),
            max_fps: Some("unlocked".parse().unwrap()),
            ..Default::default()
        });
        assert_eq!(WorldTuning::from(&changed).physics_rate_hz, 144);
        assert_eq!(changed.max_fps, "unlocked".parse().unwrap());
        assert_eq!(Settings::from(Partial { physics_rate_hz: Some(120), ..Default::default() }).physics_rate_hz, 60);
    }

    #[test]
    fn physics_rate_sources_reject_other_rates_and_preserve_lower_layers() {
        for rate in ["60", "144"] {
            let expected = Some(rate.parse().unwrap());
            assert_eq!(file::parse(&format!("physics_rate_hz = {rate}"), "test").physics_rate_hz, expected);
            assert_eq!(env::read(|key| (key == env::PHYSICS_RATE_HZ).then(|| rate.into())).physics_rate_hz, expected);
        }
        for bad in ["0", "59", "120", "240", "60.0", "unlocked"] {
            assert!(parse_physics_rate(bad).is_err());
            let layer = file::parse(&format!("physics_rate_hz = '{bad}'"), "test");
            assert_eq!(
                layer.or(Partial { physics_rate_hz: Some(144), ..Default::default() }).physics_rate_hz,
                Some(144)
            );
        }
    }

    #[test]
    fn physics_rate_persists_in_config_and_alias_payloads() {
        let settings = Settings::from(Partial { physics_rate_hz: Some(144), ..Default::default() });
        let text = write::merge("unknown = 'kept'", &crate::saves::partial_of(&settings)).unwrap();
        assert!(text.contains("unknown = \"kept\""));
        assert_eq!(file::parse(&text, "round trip").physics_rate_hz, Some(144));
        let payload = crate::saves::settings_payload(&settings).unwrap();
        assert_eq!(crate::saves::settings_from_payload(&payload).unwrap().physics_rate_hz, 144);
        assert!(write::merge("", &Partial { physics_rate_hz: Some(120), ..Default::default() }).is_err());
    }

    #[test]
    fn cli_physics_rate_keeps_render_fps_independent_and_rejects_other_rates() {
        use crate::cli::{Cli, Command};
        use clap::Parser;

        let cli =
            Cli::try_parse_from(["nfsmw", "view-world", "--physics-rate-hz", "144", "--max-fps", "unlocked"]).unwrap();
        let Some(Command::ViewWorld { view, .. }) = cli.command else { panic!("wrong command") };
        let layer = view.settings_layer();
        assert_eq!(layer.physics_rate_hz, Some(144));
        assert_eq!(layer.max_fps, Some("unlocked".parse().unwrap()));
        for rate in ["0", "120", "240", "unlocked"] {
            assert!(Cli::try_parse_from(["nfsmw", "view-world", "--physics-rate-hz", rate]).is_err());
        }
    }
}

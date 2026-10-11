use super::*;
use crate::cli::Cli;
use clap::Parser;

#[test]
fn treatment_layers_and_config_round_trip_keep_independent_switches() {
    let file = file::parse("visual_treatment='high'", "test");
    let environment = env::read(|key| match key {
        "NFSMW_VISUAL_TREATMENT" => Some("low".into()),
        _ => None,
    });
    let s = Settings::from(environment.or(file));
    assert_eq!(s.visual_treatment, VisualTreatment::Low);
    let text = write::merge("future_option=2", &file).unwrap();
    assert_eq!(file::parse(&text, "test"), file);
    assert_eq!(text.parse::<toml::Table>().unwrap()["future_option"].as_integer(), Some(2));
    assert!(Cli::try_parse_from(["nfsmw", "play", "--visual-treatment", "high"]).is_ok());
    assert!(Cli::try_parse_from(["nfsmw", "play", "--visual-treatment", "ultra"]).is_err());
}

#[test]
fn unsupported_renderers_hide_both_settings_and_keep_stored_preferences() {
    let native = test_caps::native();
    let unsupported = blackbox_gfx::Capabilities { scene_treatment: false, ..native };
    let s = Settings::from(Partial { visual_treatment: Some(VisualTreatment::High), ..Default::default() });
    assert!(availability::check("visual_treatment", &s, &unsupported).is_err());
    let shown = s.effective(&unsupported);
    assert_eq!(shown.visual_treatment, VisualTreatment::Off);
    assert_eq!(s.visual_treatment, VisualTreatment::High);
}

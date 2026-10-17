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

#[test]
fn reference_treatment_preserves_mod_parameter_semantics_and_round_trips() {
    let setting = Partial { visual_treatment: Some(VisualTreatment::Reference), ..Default::default() };
    let settings = Settings::from(setting);
    let post = settings.graphics().post;
    let replacement = post.colour_treatment_override.expect("reference parameter override");
    assert_eq!(post.colour_treatment, 1.0);
    assert_eq!(replacement.tint, [0.68, 0.62, 0.03]);
    // The mod's Saturation key writes Desaturation directly, not its complement.
    assert_eq!(replacement.desaturation, 0.80);
    assert_eq!((replacement.colour_intensity, replacement.black_intensity), (0.75, 0.40));
    let text = write::merge("future_option=2", &setting).unwrap();
    assert_eq!(file::parse(&text, "test").visual_treatment, Some(VisualTreatment::Reference));
    assert!(Cli::try_parse_from(["nfsmw", "play", "--visual-treatment", "reference"]).is_ok());
    assert_eq!(settings.effective(&test_caps::native()).visual_treatment, VisualTreatment::Reference);
    let unsupported = blackbox_gfx::Capabilities { scene_treatment: false, ..test_caps::native() };
    assert_eq!(settings.effective(&unsupported).visual_treatment, VisualTreatment::Off);
    assert_eq!(settings.visual_treatment, VisualTreatment::Reference);
    for (mode, strength) in [(VisualTreatment::Off, 0.0), (VisualTreatment::Low, 0.5), (VisualTreatment::High, 1.0)] {
        let stock = Settings::from(Partial { visual_treatment: Some(mode), ..Default::default() }).graphics().post;
        assert_eq!(stock.colour_treatment, strength);
        assert!(stock.colour_treatment_override.is_none(), "stock modes retain authored parameters");
    }
}

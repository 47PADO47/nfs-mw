use super::*;

#[test]
fn filtering_persistence_layers_and_capability_fallback_preserve_the_request() {
    let config = file::parse("texture_filtering='16x'", "test");
    assert_eq!(config.texture_filtering, Some(TextureFiltering::Anisotropic16));
    let encoded = write::merge("future_setting=42", &config).unwrap();
    assert_eq!(file::parse(&encoded, "test"), config);
    let environment = env::read(|key| (key == "NFSMW_TEXTURE_FILTERING").then(|| "trilinear".into()));
    assert_eq!(Settings::from(environment.or(config)).texture_filtering, TextureFiltering::Trilinear);
    let stored = Settings::from(config);
    assert_eq!(stored.graphics().texture_filtering, TextureFiltering::Anisotropic16);
    assert_eq!(stored.appearance_layer().texture_filtering, config.texture_filtering);
    let weak = blackbox_gfx::Capabilities { max_texture_anisotropy: 1, ..test_caps::native() };
    assert_eq!(stored.effective(&weak).texture_filtering, TextureFiltering::Trilinear);
    assert_eq!(stored.texture_filtering, TextureFiltering::Anisotropic16);
}

#[test]
fn quality_presets_keep_16x_while_legacy_presets_preserve_manual_filtering() {
    let mut stored =
        Settings::from(Partial { texture_filtering: Some(TextureFiltering::Trilinear), ..Default::default() });
    for style in VisualStyle::ALL {
        stored.select_visual_style(style);
        for quality in GraphicsQuality::ALL {
            stored.select_graphics_quality(quality);
            assert_eq!(stored.texture_filtering, TextureFiltering::Anisotropic16);
        }
    }
    stored.texture_filtering = TextureFiltering::Trilinear;
    for preset in GraphicsPreset::ALL {
        stored.apply_preset(preset);
        assert_eq!(stored.texture_filtering, TextureFiltering::Trilinear);
    }
    assert_eq!(Settings::from(Partial::default()).texture_filtering, TextureFiltering::Anisotropic8);
}

#[test]
fn filtering_cli_and_menu_reject_invalid_values_and_offer_supported_choices() {
    use crate::cli::Cli;
    use clap::Parser;
    assert!(Cli::try_parse_from(["nfsmw", "play", "--texture-filtering", "16x"]).is_ok());
    assert!(Cli::try_parse_from(["nfsmw", "play", "--texture-filtering", "4x"]).is_err());
    let mut stored =
        Settings::from(Partial { texture_filtering: Some(TextureFiltering::Anisotropic16), ..Default::default() });
    let weak = blackbox_gfx::Capabilities { max_texture_anisotropy: 1, ..test_caps::native() };
    assert!(availability::check("texture_filtering", &stored, &weak).is_err());
    stored.texture_filtering = TextureFiltering::Trilinear;
    assert!(availability::check("texture_filtering", &stored, &weak).is_ok());
}

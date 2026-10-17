use super::*;

#[test]
fn car_shading_has_a_strict_round_trip_and_defaults_to_glossy() {
    assert_eq!(Settings::from(Partial::default()).car_shading, CarShading::Glossy);
    for (text, shading) in [
        ("simple", CarShading::Simple),
        ("glossy", CarShading::Glossy),
        ("original", CarShading::Original),
        ("original-unshadowed", CarShading::OriginalUnshadowed),
    ] {
        assert_eq!(text.parse::<CarShading>().unwrap(), shading);
        assert_eq!(shading.to_string(), text);
    }
    for text in ["low", "Glossy", " simple", ""] {
        assert!(text.parse::<CarShading>().is_err(), "{text:?}");
    }
}

#[test]
fn car_shading_layers_resolve_in_order_and_bad_values_fall_through() {
    let file = file::parse("car_shading = 'simple'", "test");
    let env = env::read(|name| (name == env::CAR_SHADING).then(|| "glossy".to_owned()));
    assert_eq!(Settings::from(file).car_shading, CarShading::Simple);
    assert_eq!(Settings::from(env.or(file)).car_shading, CarShading::Glossy);
    let cli = Partial { car_shading: Some(CarShading::Simple), ..Partial::default() };
    assert_eq!(Settings::from(cli.or(env).or(file)).car_shading, CarShading::Simple);
    let bad = env::read(|name| (name == env::CAR_SHADING).then(|| "matte".to_owned()));
    assert_eq!(Settings::from(bad.or(file)).car_shading, CarShading::Simple);
    for text in ["car_shading = 'matte'", "car_shading = true"] {
        assert_eq!(file::parse(text, "test").car_shading, None);
    }
}

#[test]
fn car_shading_is_written_and_read_back() {
    let changes = Partial { car_shading: Some(CarShading::Simple), ..Partial::default() };
    assert_eq!(file::parse(&write::merge("", &changes).unwrap(), "test"), changes);
}

#[test]
fn original_car_requests_survive_settings_persistence_and_quality_changes() {
    for shading in [CarShading::Original, CarShading::OriginalUnshadowed] {
        let changes = Partial { car_shading: Some(shading), ..Partial::default() };
        let text = write::merge("", &changes).unwrap();
        let mut settings = Settings::from(file::parse(&text, "test"));
        for quality in GraphicsQuality::ALL {
            settings.select_graphics_quality(quality);
            assert_eq!(settings.car_shading, shading);
        }
    }
    let mut settings = Settings::from(Partial::default());
    settings.select_visual_style(VisualStyle::OriginalPc);
    assert_eq!(settings.car_shading, CarShading::Original);
    settings.select_visual_style(VisualStyle::Enhanced);
    assert_eq!(settings.car_shading, CarShading::Glossy);
}

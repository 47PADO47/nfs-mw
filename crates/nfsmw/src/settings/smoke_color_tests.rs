use super::*;

#[test]
fn the_mode_has_a_strict_round_trip_and_off_is_the_default() {
    assert_eq!(Settings::from(Partial::default()).smoke_color_mode, SmokeColorMode::Off);
    for (text, mode) in [
        ("off", SmokeColorMode::Off),
        ("solid", SmokeColorMode::Solid),
        ("rainbow", SmokeColorMode::Rainbow),
        ("speed", SmokeColorMode::Speed),
        ("slip", SmokeColorMode::Slip),
    ] {
        assert_eq!(text.parse::<SmokeColorMode>().unwrap(), mode);
        assert_eq!(mode.to_string(), text);
    }
    for text in ["", "Solid", "colour", "on", " off", "rainbow2"] {
        assert!(text.parse::<SmokeColorMode>().is_err(), "{text:?}");
    }
}

#[test]
fn the_menu_steps_through_every_mode_and_wraps() {
    let mut mode = SmokeColorMode::Off;
    let mut seen = Vec::new();
    for _ in 0..SmokeColorMode::ALL.len() {
        seen.push(mode);
        mode = mode.next();
    }
    assert_eq!(seen, SmokeColorMode::ALL);
    assert_eq!(mode, SmokeColorMode::Off, "back at the start");
}

#[test]
fn a_colour_is_a_palette_name_or_hex_and_prints_back_the_same() {
    assert_eq!("orange".parse::<SmokeColor>().unwrap().0, [255, 140, 20]);
    assert_eq!("ORANGE".parse::<SmokeColor>().unwrap().0, [255, 140, 20]);
    assert_eq!("#ff8800".parse::<SmokeColor>().unwrap().0, [255, 136, 0]);
    assert_eq!("00FFAA".parse::<SmokeColor>().unwrap().0, [0, 255, 170]);
    for (name, rgb) in SMOKE_PALETTE {
        let colour = name.parse::<SmokeColor>().unwrap();
        assert_eq!((colour.0, colour.to_string().as_str()), (rgb, name));
    }
    assert_eq!(SmokeColor([255, 136, 0]).to_string(), "#ff8800", "a custom colour prints as hex");
    for text in ["", "#fff", "#ff88", "#gg0000", "#ff88001", "teal", "0xff8800"] {
        assert!(text.parse::<SmokeColor>().is_err(), "{text:?}");
    }
}

#[test]
fn the_palette_steps_and_a_custom_colour_goes_to_the_one_after_the_nearest() {
    let first = SmokeColor::default();
    assert_eq!(first.name(), Some("red"));
    assert_eq!(first.next().name(), Some("orange"));
    let mut colour = first;
    for _ in 0..SMOKE_PALETTE.len() {
        colour = colour.next();
    }
    assert_eq!(colour, first, "the palette wraps");
    // Nearly orange: the next palette colour after orange is yellow.
    assert_eq!(SmokeColor([250, 135, 25]).next().name(), Some("yellow"));
    assert_eq!(SmokeColor::default().label(), "Red");
    assert_eq!(SmokeColor([1, 2, 3]).label(), "#010203");
}

#[test]
fn the_layers_resolve_in_order_and_bad_values_fall_through() {
    let file = file::parse("smoke_color_mode = 'rainbow'\nsmoke_color = '#102030'", "test");
    assert_eq!(file.smoke_color_mode, Some(SmokeColorMode::Rainbow));
    assert_eq!(file.smoke_color, Some(SmokeColor([16, 32, 48])));
    let env = env::read(|name| match name {
        env::SMOKE_COLOR_MODE => Some("solid".to_owned()),
        env::SMOKE_COLOR => Some("blue".to_owned()),
        _ => None,
    });
    let settings = Settings::from(env.or(file));
    assert_eq!((settings.smoke_color_mode, settings.smoke_color.name()), (SmokeColorMode::Solid, Some("blue")));
    let invalid = env::read(|name| (name == env::SMOKE_COLOR).then(|| "teal".to_owned()));
    assert_eq!(Settings::from(invalid.or(file)).smoke_color, SmokeColor([16, 32, 48]));
    for text in ["smoke_color = 5", "smoke_color = '#12'", "smoke_color_mode = 'always'", "smoke_color_mode = true"] {
        let parsed = file::parse(text, "test");
        assert_eq!((parsed.smoke_color, parsed.smoke_color_mode), (None, None), "{text}");
    }
}

#[test]
fn the_choice_is_written_to_the_config_and_read_back() {
    let changes = Partial {
        smoke_color_mode: Some(SmokeColorMode::Speed),
        smoke_color: Some(SmokeColor([1, 2, 3])),
        ..Partial::default()
    };
    let text = write::merge("tire_smoke = true", &changes).unwrap();
    assert!(text.contains("smoke_color = \"#010203\"") && text.contains("smoke_color_mode = \"speed\""), "{text}");
    let saved = file::parse(&text, "test");
    assert_eq!((saved.smoke_color_mode, saved.smoke_color), (changes.smoke_color_mode, changes.smoke_color));
    assert_eq!(saved.tire_smoke, Some(true), "the other keys stay");
}

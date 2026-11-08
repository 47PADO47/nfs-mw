use super::*;

#[test]
fn defaults() {
    let s = Settings::from(Partial::default());
    assert_eq!((s.backend, s.vsync, s.max_fps), (GraphicsApi::Auto, true, MaxFps::default()));
    assert_eq!(s.transmission, Transmission::Automatic, "automatic, as in the original");
    assert_eq!(s.minimap, MinimapMode::Fixed, "fixed, the original's free roam mode");
    assert_eq!(s.wheel_options(), WheelOptions::default(), "no clutch pedal and no H-shifter unless asked for");
    assert_eq!((s.paddle_up, s.paddle_down), (None, None));
}

#[test]
fn layers_resolve_in_order() {
    let cli = Partial { vsync: Some(false), ..Partial::default() };
    let env = env::read(|n| (n == env::BACKEND).then(|| "gl".to_owned()));
    let file = file::parse("backend = 'vulkan'\nmax_fps = 90\n", "test");
    let s: Settings = cli.or(env).or(file).into();
    assert!(!s.vsync, "command line");
    assert_eq!(s.backend, GraphicsApi::Gl, "environment over file");
    assert_eq!(s.max_fps, "90".parse().unwrap(), "file over default");
}

#[test]
fn window_layers_resolve_independently_and_bad_values_fall_through() {
    let cli = Partial { window_mode: Some(WindowMode::Exclusive), ..Partial::default() };
    let env = env::read(|n| match n {
        env::WINDOW_MODE => Some("borderless".into()),
        env::MONITOR => Some("1".into()),
        env::RESOLUTION => Some("bad size".into()),
        _ => None,
    });
    let file = file::parse("window_mode = 'windowed'\nmonitor = 'primary'\nresolution = '1920x1080'", "test");
    let s: Settings = cli.or(env).or(file).into();
    assert_eq!(s.window_mode, WindowMode::Exclusive);
    assert_eq!(s.monitor, Monitor::Index(1));
    assert_eq!(s.resolution, Resolution::pixels(1920, 1080).unwrap());
    let bad = file::parse("window_mode = 'bad'\nmonitor = -1\nresolution = '0x0'", "test");
    assert_eq!(bad, Partial::default());
    assert_eq!(file::parse("monitor = 0", "test").monitor, Some(Monitor::Index(0)));
}

#[test]
fn traffic_is_on_by_default_with_one_cop_in_twenty() {
    let s = Settings::from(Partial::default());
    assert_eq!((s.traffic, s.cop_share), (DEFAULT_TRAFFIC, DEFAULT_COP_SHARE));
    assert!(s.traffic > 0);
    assert_eq!(DEFAULT_COP_SHARE, Percent(5), "5 percent is one in twenty");
}

#[test]
fn cop_health_bars_are_off_by_default_and_come_from_every_layer() {
    assert!(!Settings::from(Partial::default()).cop_health_bars);
    assert!(!WorldTuning::from(&Settings::from(Partial::default())).cop_health_bars);
    const { assert!(!DEFAULT_COP_HEALTH_BARS) };
    let cli = Partial { cop_health_bars: Some(false), ..Partial::default() };
    let env = env::read(|n| (n == env::COP_HEALTH_BARS).then(|| "on".to_owned()));
    let file = file::parse("cop_health_bars = true\n", "test");
    assert!(!Settings::from(cli.or(env).or(file)).cop_health_bars, "the command line wins");
    assert!(Settings::from(Partial::default().or(env).or(file)).cop_health_bars, "the environment over the file");
    assert!(Settings::from(Partial::default().or(file)).cop_health_bars);
    assert_eq!(file::parse("cop_health_bars = 3", "test").cop_health_bars, None);
    assert!(!WorldTuning::from(&Settings::from(cli.or(env).or(file))).cop_health_bars);
}

#[test]
fn rider_gear_is_on_by_default_and_comes_from_every_layer() {
    assert!(Settings::from(Partial::default()).rider_gear);
    let cli = Partial { rider_gear: Some(true), ..Partial::default() };
    let env = env::read(|n| (n == "NFSMW_RIDER_GEAR").then(|| "off".to_owned()));
    let file = file::parse("rider_gear = false\n", "test");
    assert!(Settings::from(cli.or(env).or(file)).rider_gear, "the command line wins");
    assert!(!Settings::from(Partial::default().or(env).or(file)).rider_gear, "the environment over the file");
    assert!(!Settings::from(Partial::default().or(file)).rider_gear);
    assert_eq!(file::parse("rider_gear = 3", "test").rider_gear, None);
}

#[test]
fn each_class_of_bike_has_its_riding_style_balanced_by_default_from_every_layer() {
    use super::{BikeClass, RidingStyle};
    let all = |s: Settings| [s.riding_style_road, s.riding_style_motard, s.riding_style_offroad];
    assert_eq!(all(Settings::from(Partial::default())), [RidingStyle::Balanced; 3]);
    let cli = Partial { riding_style_motard: Some(RidingStyle::OldSchool), ..Partial::default() };
    let env = env::read(|n| (n == "NFSMW_RIDING_STYLE_MOTARD").then(|| "shoulders-out".to_owned()));
    let file = file::parse(
        "riding_style_motard = \"balanced\"
riding_style_road = \"old-school\"
",
        "test",
    );
    let layered = Settings::from(cli.or(env).or(file));
    assert_eq!(layered.riding_style_motard, RidingStyle::OldSchool, "the command line wins");
    assert_eq!(
        layered.riding_style_road,
        RidingStyle::OldSchool,
        "a class the upper layers leave out comes from the file"
    );
    assert_eq!(layered.riding_style_offroad, RidingStyle::Balanced);
    assert_eq!(Settings::from(Partial::default().or(env).or(file)).riding_style_motard, RidingStyle::ShouldersOut);
    assert_eq!(
        file::parse("riding_style_road = \"leaning\"", "test").riding_style_road,
        None,
        "a name that is not a style is ignored"
    );
    for class in BikeClass::ALL {
        let env_name = class.env();
        let set = env::read(|n| (n == env_name).then(|| "old-school".to_owned()));
        assert_eq!(
            all(Settings::from(set)).iter().filter(|style| **style == RidingStyle::OldSchool).count(),
            1,
            "{env_name}"
        );
    }
    let written =
        write::merge("", &Partial { riding_style_offroad: Some(RidingStyle::ShouldersOut), ..Partial::default() })
            .unwrap();
    assert!(written.contains("riding_style_offroad = \"shoulders-out\""), "{written}");
    assert!(!written.contains("riding_style_road"), "{written}");
}

#[test]
fn the_older_single_riding_style_key_and_variable_set_all_three_classes_and_a_class_key_wins() {
    use super::RidingStyle;
    let all = |p: Partial| {
        let s = Settings::from(p);
        [s.riding_style_road, s.riding_style_motard, s.riding_style_offroad]
    };
    let file = file::parse(
        "riding_style = \"old-school\"
",
        "test",
    );
    assert_eq!(all(file), [RidingStyle::OldSchool; 3]);
    let env = env::read(|n| (n == "NFSMW_RIDING_STYLE").then(|| "shoulders-out".to_owned()));
    assert_eq!(all(env), [RidingStyle::ShouldersOut; 3]);
    let both = file::parse(
        "riding_style = \"old-school\"
riding_style_road = \"balanced\"
",
        "test",
    );
    assert_eq!(all(both), [RidingStyle::Balanced, RidingStyle::OldSchool, RidingStyle::OldSchool]);
    let named = env::read(|n| match n {
        "NFSMW_RIDING_STYLE" => Some("shoulders-out".to_owned()),
        "NFSMW_RIDING_STYLE_OFFROAD" => Some("balanced".to_owned()),
        _ => None,
    });
    assert_eq!(all(named), [RidingStyle::ShouldersOut, RidingStyle::ShouldersOut, RidingStyle::Balanced]);
    assert_eq!(all(env.or(file)), [RidingStyle::ShouldersOut; 3], "the environment over the file");
}

#[test]
fn the_fingers_are_two_by_default_and_come_from_every_layer_as_numbers_or_text() {
    use super::{BrakeFingers, ClutchFingers};
    let defaults = Settings::from(Partial::default());
    assert_eq!((defaults.brake_fingers, defaults.clutch_fingers), (BrakeFingers::Two, ClutchFingers::Two));
    let cli = Partial { brake_fingers: Some(BrakeFingers::One), ..Partial::default() };
    let env = env::read(|n| match n {
        "NFSMW_BRAKE_FINGERS" => Some("4".to_owned()),
        "NFSMW_CLUTCH_FINGERS" => Some("4".to_owned()),
        _ => None,
    });
    let file = file::parse(
        "brake_fingers = 4
clutch_fingers = \"2\"
",
        "test",
    );
    let layered = Settings::from(cli.or(env).or(file));
    assert_eq!((layered.brake_fingers, layered.clutch_fingers), (BrakeFingers::One, ClutchFingers::Four));
    let from_file = Settings::from(Partial::default().or(file));
    assert_eq!((from_file.brake_fingers, from_file.clutch_fingers), (BrakeFingers::Four, ClutchFingers::Two));
    assert_eq!(
        file::parse(
            "brake_fingers = 3
clutch_fingers = 1",
            "test"
        )
        .brake_fingers,
        None
    );
    assert_eq!(file::parse("clutch_fingers = 1", "test").clutch_fingers, None, "the clutch has no one-finger clip");
    let both = Partial {
        brake_fingers: Some(BrakeFingers::Four),
        clutch_fingers: Some(ClutchFingers::Four),
        ..Partial::default()
    };
    let written = write::merge("", &both).unwrap();
    assert!(written.contains("brake_fingers = 4") && written.contains("clutch_fingers = 4"), "{written}");
}

#[test]
fn traffic_lights_are_on_by_default_and_come_from_every_layer() {
    assert!(Settings::from(Partial::default()).traffic_lights);
    const { assert!(DEFAULT_TRAFFIC_LIGHTS) };
    let cli = Partial { traffic_lights: Some(false), ..Partial::default() };
    let env = env::read(|n| (n == env::TRAFFIC_LIGHTS).then(|| "on".to_owned()));
    let file = file::parse("traffic_lights = false\n", "test");
    assert!(!Settings::from(cli.or(env).or(file)).traffic_lights, "the command line wins");
    assert!(Settings::from(Partial::default().or(env).or(file)).traffic_lights, "the environment over the file");
    assert!(!Settings::from(Partial::default().or(file)).traffic_lights);
    assert_eq!(file::parse("traffic_lights = 3", "test").traffic_lights, None);
}

#[test]
fn the_traffic_settings_come_from_every_layer() {
    let cli = Partial { traffic: Some(0), ..Partial::default() };
    let env = env::read(|n| match n {
        env::TRAFFIC => Some("20".to_owned()),
        env::COP_SHARE => Some("10".to_owned()),
        _ => None,
    });
    let file = file::parse("traffic = 6\ncop_share = 25\n", "test");
    let s: Settings = cli.or(env).or(file).into();
    assert_eq!(
        (s.traffic, s.cop_share),
        (0, Percent(10)),
        "the command line turns traffic off, the environment sets the share"
    );
    let s: Settings = Partial::default().or(file).into();
    assert_eq!((s.traffic, s.cop_share), (6, Percent(25)));
    assert_eq!(env::read(|n| (n == env::TRAFFIC).then(|| "many".to_owned())).traffic, None);
    assert_eq!(file::parse("traffic = -3", "test").traffic, None, "a negative count is ignored");
}

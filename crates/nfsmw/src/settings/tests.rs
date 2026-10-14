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

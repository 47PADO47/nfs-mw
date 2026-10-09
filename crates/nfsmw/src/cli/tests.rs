use super::*;

#[test]
fn the_hud_demo_takes_optional_gauges() {
    let plain = parse_hud_demo("100,4000,8000,3").unwrap();
    assert!(!plain.has_nos && !plain.has_turbo);
    assert!((plain.speed - 100.0 / 3.6).abs() < 1e-4);
    let nos = parse_hud_demo("100,4000,8000,3,60").unwrap();
    assert!(nos.has_nos && !nos.has_turbo && (nos.nos - 0.6).abs() < 1e-6);
    let both = parse_hud_demo("100,4000,8000,3,60,-5").unwrap();
    assert!(both.has_turbo && both.boost_psi == -5.0);
    assert!(parse_hud_demo("100,4000,8000").is_err());
    assert!(parse_hud_demo("1,2,3,4,5,6,7").is_err());
}

#[test]
fn xy() {
    assert_eq!(parse_xy("1.5,-2").unwrap(), [1.5, -2.0]);
    assert!(parse_xy("3").is_err());
}

#[test]
fn screenshot_size_is_explicit_and_independent_of_window_preferences() {
    for size in ["1920x1080", "2560x1440", "3840x2160"] {
        let cli =
            Cli::try_parse_from(["nfsmw", "view-world", "--screenshot", "out.png", "--screenshot-size", size]).unwrap();
        let Some(Command::ViewWorld { view, .. }) = cli.command else { panic!("wrong command") };
        assert_eq!(view.screenshot_size, Some(parse_screenshot_size(size).unwrap()));
        assert_eq!(view.settings_layer().resolution, None);
    }
    assert!(Cli::try_parse_from(["nfsmw", "view-world", "--screenshot-size", "1920x1080"]).is_err());
    for invalid in ["native", "0x1080", "1920x0", "16385x1080", "1920"] {
        assert!(parse_screenshot_size(invalid).is_err(), "{invalid}");
    }
}

#[test]
fn cli_is_consistent() {
    use clap::CommandFactory;
    Cli::command().debug_assert();
}

#[test]
fn no_command_starts_the_game() {
    let bare = Cli::try_parse_from(["nfsmw"]).unwrap();
    assert!(bare.command.is_none());
    let cli = Cli::try_parse_from(["nfsmw", "--game-dir", "X"]).unwrap();
    assert!(cli.command.is_none() && cli.game_dir.is_some());
    assert!(Cli::try_parse_from(["nfsmw", "--game-dir", "X", "view-car"]).is_ok());
    assert!(Cli::try_parse_from(["nfsmw", "--no-sound"]).is_err(), "viewer options belong to a command");
}

#[test]
fn help_and_keys_are_commands() {
    use clap::error::ErrorKind;
    for flag in ["-h", "--help"] {
        let err = Cli::try_parse_from(["nfsmw", flag]).err().expect("help ends parsing");
        assert_eq!(err.kind(), ErrorKind::DisplayHelp);
        assert!(err.to_string().contains("Examples:"), "the help carries the examples");
    }
    assert!(matches!(Cli::try_parse_from(["nfsmw", "keys"]).unwrap().command, Some(Command::Keys)));
    assert!(matches!(Cli::try_parse_from(["nfsmw", "bindings"]).unwrap().command, Some(Command::Keys)));
}

#[test]
fn window_options_reach_the_cli_settings_layer() {
    let cli = Cli::try_parse_from([
        "nfsmw",
        "view-car",
        "--window-mode",
        "exclusive",
        "--monitor",
        "1",
        "--resolution",
        "1920x1080",
    ])
    .unwrap();
    let Some(Command::ViewCar { view, .. }) = cli.command else { panic!("wrong command") };
    let layer = view.settings_layer();
    assert_eq!(layer.window_mode, Some(WindowMode::Exclusive));
    assert_eq!(layer.monitor, Some(Monitor::Index(1)));
    assert_eq!(layer.resolution, Some(Resolution::pixels(1920, 1080).unwrap()));
    assert!(Cli::try_parse_from(["nfsmw", "view-car", "--resolution", "0x0"]).is_err());
}

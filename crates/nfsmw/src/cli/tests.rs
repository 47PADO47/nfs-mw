use super::hud_demo::parse_hud_demo;
use super::parse::{parse_screenshot_size, parse_xy};
use super::*;
use crate::settings::{Monitor, Resolution, WindowMode};

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
    let breaker = parse_hud_demo("140,5000,8000,3,0,0,70,1").unwrap();
    assert_eq!(breaker.speedbreaker, 0.7);
    assert!(breaker.speedbreaker_active);
    assert!(parse_hud_demo("1,2,3,4,5,6,7,8,9,10,11,12,13").is_err());
    assert!(plain.pursuit.is_none(), "no chase unless the demo names one");
    let chase = parse_hud_demo("140,5000,8000,3,0,0,0,0,137.5,40").unwrap();
    let pursuit = chase.pursuit.expect("a chase of 137.5 s");
    assert_eq!(pursuit.secs, 137.5);
    assert!((pursuit.evade - 0.4).abs() < 1e-6 && pursuit.cooldown, "40% of the evade done is the cool-down");
    let busting = parse_hud_demo("140,5000,8000,3,0,0,0,0,10,0,-60").unwrap().pursuit.expect("a chase");
    assert!((busting.busted + 0.6).abs() < 1e-6, "the last value is the signed meter in percent");
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
fn hidden_benchmark_accepts_an_explicit_4k_target() {
    let cli = Cli::try_parse_from([
        "nfsmw",
        "view-world",
        "--bench-seconds",
        "10",
        "--offscreen",
        "--screenshot-size",
        "3840x2160",
    ])
    .unwrap();
    let Some(Command::ViewWorld { view, .. }) = cli.command else { panic!("wrong command") };
    assert_eq!(view.screenshot_size, Some([3840, 2160]));
    assert_eq!(view.settings_layer().resolution, None);
}

#[test]
fn no_command_starts_the_game() {
    let bare = Cli::try_parse_from(["nfsmw"]).unwrap();
    assert!(bare.command.is_none());
    let cli = Cli::try_parse_from(["nfsmw", "--game-dir", "X"]).unwrap();
    assert!(cli.command.is_none() && cli.game_dir.is_some());
    assert!(Cli::try_parse_from(["nfsmw", "--game-dir", "X", "view-car"]).is_ok());
    assert!(Cli::try_parse_from(["nfsmw", "asset-viewer", "CAMARO", "--window-mode", "windowed"]).is_ok());
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
fn dump_anim_takes_a_bundle_and_an_optional_sample() {
    let plain = Cli::try_parse_from(["nfsmw", "dump-anim", "NIS/Scene_IntroNis05_BundleB.bun"]).unwrap();
    let Some(Command::DumpAnim { path, bank, sample }) = plain.command else { panic!("wrong command") };
    assert_eq!(path, "NIS/Scene_IntroNis05_BundleB.bun");
    assert!(bank.is_none() && sample.is_none());

    let sampled =
        Cli::try_parse_from(["nfsmw", "dump-anim", "NIS/x_BundleB.bun", "--bank", "ANM_Walk", "--sample", "5"])
            .unwrap();
    let Some(Command::DumpAnim { bank, sample, .. }) = sampled.command else { panic!("wrong command") };
    assert_eq!(bank.as_deref(), Some("ANM_Walk"));
    assert_eq!(sample, Some(5));

    assert!(
        Cli::try_parse_from(["nfsmw", "dump-anim", "NIS/x_BundleB.bun", "--sample", "5"]).is_err(),
        "a sample needs a bank"
    );
    assert!(Cli::try_parse_from(["nfsmw", "dump-anim", "NIS/x_BundleB.bun", "--bank", "A", "--sample", "0"]).is_err());
}

#[test]
fn view_anim_takes_a_bundle_and_optional_rig_clip_and_time() {
    let plain = Cli::try_parse_from(["nfsmw", "view-anim", "NIS/Scene_ArrestF02_BundleB.bun"]).unwrap();
    let Some(Command::ViewAnim { path, skeleton, clip, time, .. }) = plain.command else { panic!("wrong command") };
    assert_eq!(path, "NIS/Scene_ArrestF02_BundleB.bun");
    assert!(skeleton.is_none() && clip.is_none());
    assert_eq!(time, 0.0);

    let chosen = Cli::try_parse_from([
        "nfsmw",
        "view-anim",
        "NIS/Scene_ArrestF02_BundleB.bun",
        "--skeleton",
        "ArrestF02",
        "--clip",
        "ArrestF02_Cuffs_s",
        "--time",
        "1.5",
    ])
    .unwrap();
    let Some(Command::ViewAnim { skeleton, clip, time, .. }) = chosen.command else { panic!("wrong command") };
    assert_eq!(skeleton.as_deref(), Some("ArrestF02"));
    assert_eq!(clip.as_deref(), Some("ArrestF02_Cuffs_s"));
    assert_eq!(time, 1.5);

    assert!(Cli::try_parse_from(["nfsmw", "view-anim"]).is_err(), "a bundle is needed");
    assert!(Cli::try_parse_from(["nfsmw", "view-anim", "x_BundleB.bun", "--time", "soon"]).is_err());
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

#[test]
fn the_traffic_light_switches_reach_the_cli_settings_layer() {
    let layer = |flags: &[&str]| {
        let args = ["nfsmw", "view-car"].into_iter().chain(flags.iter().copied());
        let Some(Command::ViewCar { view, .. }) = Cli::try_parse_from(args).unwrap().command else {
            panic!("wrong command")
        };
        view.settings_layer().traffic_lights
    };
    assert_eq!(layer(&[]), None);
    assert_eq!(layer(&["--traffic-lights"]), Some(true));
    assert_eq!(layer(&["--no-traffic-lights"]), Some(false));
    assert!(Cli::try_parse_from(["nfsmw", "view-car", "--traffic-lights", "--no-traffic-lights"]).is_err());
}

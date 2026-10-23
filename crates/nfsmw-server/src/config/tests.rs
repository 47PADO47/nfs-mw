use std::path::PathBuf;

use log::LevelFilter;
use nfsmw_net::message::{CollisionMode, JoinInProgress, Mode};

use super::{ConfigError, ServerConfig};

/// The sample shipped beside the crate.
const SAMPLE: &str = include_str!("../../server.toml");

#[test]
fn the_sample_config_loads() {
    let config = ServerConfig::from_toml(SAMPLE).expect("the sample is valid");
    assert_eq!(config.settings.name.as_str(), "LAN night");
    assert_eq!(config.settings.mode, Mode::FreeRoam);
    assert_eq!(config.settings.max_players, 8);
    assert_eq!(config.settings.join_in_progress, JoinInProgress::FreeRoam);
    assert_eq!(config.settings.collision, CollisionMode::Full);
    assert_eq!(config.port, 9750);
    assert_eq!(config.snapshot_hz, 20);
    assert_eq!(config.replay_dir, None);
    assert_eq!(config.log_file, Some(PathBuf::from("nfsmw-server.log")));
    assert_eq!(config.log_level, LevelFilter::Info);
    assert_eq!(config.discovery_addr().map(|addr| addr.port()), Some(9751));
}

#[test]
fn an_empty_file_is_the_defaults() {
    assert_eq!(ServerConfig::from_toml("").expect("empty is fine"), ServerConfig::default());
}

#[test]
fn a_race_config_sets_mode_and_laps() {
    let config = ServerConfig::from_toml("mode = \"circuit\"\nlaps = 3\ntrack = 7\n").expect("valid");
    assert_eq!(config.settings.mode, Mode::Circuit);
    assert_eq!(config.settings.laps, 3);
    assert_eq!(config.settings.track, 7);
}

#[test]
fn a_password_locks_the_room() {
    let config = ServerConfig::from_toml("password = \"secret\"").expect("valid");
    assert_eq!(config.password.as_deref(), Some("secret"));
    assert!(config.settings.password_required, "a password turns the flag on");
    let open = ServerConfig::from_toml("password = \"\"").expect("an empty password is none");
    assert_eq!(open.password, None);
    assert!(!open.settings.password_required);
    let long = format!("password = \"{}\"", "x".repeat(65));
    let err = ServerConfig::from_toml(&long).expect_err("too long").to_string();
    assert!(err.contains("`password` is out of range"), "{err}");
}

#[test]
fn bad_files_name_the_key() {
    let cases: [(&str, &str); 9] = [
        ("colour = 1", "unknown key `colour`"),
        ("port = \"x\"", "`port` must be an integer"),
        ("port = 70000", "`port` is out of range"),
        ("snapshot_hz = 0", "`snapshot_hz` is out of range"),
        ("snapshot_budget = 99999", "`snapshot_budget` is out of range"),
        ("mode = \"drift\"", "key `mode`"),
        ("max_players = 40", "do not hold together"),
        ("log_level = \"loud\"", "`log_level` must be one of"),
        ("password_required = true", "do not hold together"),
    ];
    for (text, expected) in cases {
        let err = ServerConfig::from_toml(text).expect_err(text).to_string();
        assert!(err.contains(expected), "{text}: {err}");
    }
    assert!(matches!(ServerConfig::from_toml("port = "), Err(ConfigError::Syntax(_))));
}

#[test]
fn the_snapshot_interval_rounds_to_ticks() {
    assert_eq!(ServerConfig::snapshot_interval(20), 3);
    assert_eq!(ServerConfig::snapshot_interval(10), 6);
    assert_eq!(ServerConfig::snapshot_interval(60), 1);
    assert_eq!(ServerConfig::snapshot_interval(0), 60);
}

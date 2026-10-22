//! The LAN switches on the command line.

use std::net::SocketAddr;
use std::path::PathBuf;

use clap::Parser;

use super::net_args::{DEFAULT_PORT, parse_server_addr};
use super::*;

fn net(args: &[&str]) -> NetArgs {
    let mut line = vec!["nfsmw"];
    line.extend_from_slice(args);
    Cli::try_parse_from(line).unwrap().net
}

#[test]
fn host_takes_a_port_a_name_and_a_config_file() {
    let args = net(&["--host", "--port", "9800", "--name", "Rockport LAN", "--server-config", "lan.toml"]);
    assert!(args.host && args.starts_session());
    assert_eq!(args.port, Some(9800));
    assert_eq!(args.server_name.as_deref(), Some("Rockport LAN"));
    assert_eq!(args.server_config, Some(PathBuf::from("lan.toml")));
}

#[test]
fn the_switches_are_global_so_they_go_after_a_command() {
    let cli = Cli::try_parse_from(["nfsmw", "play", "--skip-boot", "--connect", "10.0.0.7:9760"]).unwrap();
    assert_eq!(cli.net.connect, Some(SocketAddr::from(([10, 0, 0, 7], 9760))));
    assert!(matches!(cli.command, Some(Command::Play { skip_boot: true, .. })));
}

#[test]
fn connect_without_a_port_takes_the_default() {
    assert_eq!(net(&["--connect", "192.168.1.5"]).connect, Some(SocketAddr::from(([192, 168, 1, 5], DEFAULT_PORT))));
    assert_eq!(parse_server_addr("localhost").map(|addr| addr.port()), Ok(DEFAULT_PORT));
    assert_eq!(parse_server_addr("localhost:9761").map(|addr| addr.port()), Ok(9761));
    assert!(parse_server_addr("").is_err());
    assert!(Cli::try_parse_from(["nfsmw", "--connect", "no such host name.invalid"]).is_err());
}

#[test]
fn dedicated_runs_for_some_seconds_and_names_its_config() {
    let args = net(&["--dedicated", "--server-config", "server.toml", "--seconds", "30"]);
    assert!(args.dedicated && !args.starts_session());
    assert_eq!((args.seconds, args.server_config), (Some(30), Some(PathBuf::from("server.toml"))));
    assert!(Cli::try_parse_from(["nfsmw", "--dedicated"]).unwrap().is_dedicated());
    assert!(!Cli::try_parse_from(["nfsmw"]).unwrap().is_dedicated());
}

#[test]
fn discover_asks_the_default_port_unless_told() {
    assert_eq!(net(&["--discover"]).discover_port(), DEFAULT_PORT);
    assert_eq!(net(&["--discover", "--port", "9800"]).discover_port(), 9800);
}

#[test]
fn the_modes_exclude_each_other() {
    for line in [
        ["nfsmw", "--host", "--connect", "1.2.3.4"],
        ["nfsmw", "--host", "--dedicated"],
        ["nfsmw", "--dedicated", "--discover"],
        ["nfsmw", "--connect", "1.2.3.4", "--discover"],
    ] {
        assert!(Cli::try_parse_from(line).is_err(), "{line:?}");
    }
}

#[test]
fn the_server_name_does_not_clash_with_a_command_s_name_argument() {
    let cli = Cli::try_parse_from(["nfsmw", "play-movie", "ealogo", "--name", "LAN"]).unwrap();
    assert_eq!(cli.net.server_name.as_deref(), Some("LAN"));
    assert!(matches!(cli.command, Some(Command::PlayMovie { name, .. }) if name == "ealogo"));
}

//! The LAN switches (`cli/net_args.rs`): `--discover` and `--dedicated` run here without a window; `--host` and
//! `--connect` start the game straight into free roam with a session.

use std::time::Duration;

use anyhow::{Context, Result};
use game_install::GameDir;

use crate::cli::NetArgs;
use crate::net::{NetPlugin, NetSession, NetStart, PlayerSetup, ServeOptions, discover};
use crate::scenes::world::DEFAULT_CAR;
use crate::settings::Settings;

/// How long `--discover` listens.
const DISCOVER_FOR: Duration = Duration::from_secs(2);

/// Runs `--discover` or `--dedicated`; `None` when neither was asked for (the game runs).
pub fn run_without_window(args: &NetArgs, game_dir: Option<&std::path::Path>) -> Option<Result<()>> {
    if args.discover {
        return Some(print_servers(args.discover_port()));
    }
    if !args.dedicated {
        return None;
    }
    Some(dedicated(args, game_dir))
}

/// The player of this program, before the settings name it.
pub fn player() -> PlayerSetup {
    PlayerSetup { name: "Player".into(), car: DEFAULT_CAR.into() }
}

/// What a server started from the command line runs with.
pub fn serve_options(args: &NetArgs) -> ServeOptions {
    ServeOptions {
        port: args.port,
        name: args.server_name.clone(),
        config: args.server_config.clone(),
        car: player().car,
        seed: None,
        settings: None,
        password: None,
    }
}

/// The session the game starts with: the player's name, the host's default port and the identity key file from the
/// `net_*` settings.
pub fn plugin(dir: &GameDir, args: &NetArgs, settings: &Settings) -> NetPlugin {
    let net = &settings.net;
    let start = match (args.host, args.connect) {
        (true, _) => NetStart::Host(ServeOptions { port: args.port.or(Some(net.port)), ..serve_options(args) }),
        (false, Some(addr)) => NetStart::Join(addr),
        (false, None) => NetStart::Idle,
    };
    let config_dir = Settings::config_path().and_then(|path| path.parent().map(std::path::Path::to_path_buf));
    let identity_file = config_dir.map(|dir| net.identity_path(&dir));
    let player = PlayerSetup { name: net.name.as_str().to_owned(), ..player() };
    NetPlugin { dir: Some(dir.clone()), player, start, identity_file }
}

fn print_servers(port: u16) -> Result<()> {
    println!("Looking for LAN servers on port {port} for {} s...", DISCOVER_FOR.as_secs());
    let found = discover::scan(port, DISCOVER_FOR).context("the LAN scan did not start")?;
    if found.is_empty() {
        println!("No server answered.");
        return Ok(());
    }
    for server in &found {
        println!("{}", discover::describe(server));
    }
    Ok(())
}

fn dedicated(args: &NetArgs, game_dir: Option<&std::path::Path>) -> Result<()> {
    let dir = nfsmw_data::game::open_install(game_dir)?;
    let mut session = NetSession::new(Some(dir), player());
    session.run_dedicated(&serve_options(args), args.seconds)
}

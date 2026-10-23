//! `nfsmw-server`: the multiplayer server over the toy world, for protocol tests, soak runs and CI. No window, no
//! audio, no game install. Usage in `cli.rs` and `docs/specs/multiplayer/server.md`.

mod cli;

use std::process::ExitCode;
use std::time::{SystemTime, UNIX_EPOCH};

use blackbox_net_core::secure::IdentityKey;
use nfsmw_net::message::Mode;
use nfsmw_server::{Server, ServerConfig, ServerSetup, TickHook, attach_bots, bind, logging};
use nfsmw_sim::toy::{RaceConfig, ToyConfig, ToyWorld};

fn main() -> ExitCode {
    let args = match cli::parse(std::env::args().skip(1)) {
        Ok(args) => args,
        Err(message) => {
            eprintln!("{message}\n\n{}", cli::USAGE);
            return ExitCode::from(2);
        }
    };
    if args.help {
        println!("{}", cli::USAGE);
        return ExitCode::SUCCESS;
    }
    match serve(&args) {
        Ok(()) => ExitCode::SUCCESS,
        Err(message) => {
            eprintln!("nfsmw-server: {message}");
            ExitCode::FAILURE
        }
    }
}

fn serve(args: &cli::Args) -> Result<(), String> {
    let mut config = match &args.config {
        Some(path) => ServerConfig::load(path).map_err(|err| err.to_string())?,
        None => ServerConfig::default(),
    };
    if let Some(port) = args.port {
        config.port = port;
    }
    logging::init(config.log_level, config.log_file.as_deref()).map_err(|err| format!("logging: {err}"))?;
    let identity = match &config.identity_file {
        Some(path) => nfsmw_server::identity::load_or_create(path).map_err(|err| err.to_string())?,
        None => IdentityKey::generate().map_err(|err| format!("no identity key: {err}"))?,
    };
    log::info!("identity {}", identity.key_id().fingerprint());
    let endpoint = bind(&config, identity).map_err(|err| format!("cannot bind {}: {err}", config.game_addr()))?;
    log::info!("listening on {}", endpoint.local_addr());
    let world = toy_world(&config, args);
    let setup = ServerSetup::toy(world.config().car_types);
    let mut server = Server::new(config, setup, endpoint, world, 0).map_err(|err| format!("settings: {err:?}"))?;
    let Some(mut bots) = attach_bots(&mut server, args.bots)? else {
        server.run(args.seconds, None);
        return Ok(());
    };
    let hook: &mut dyn TickHook<ToyWorld> = bots.as_mut();
    server.run(args.seconds, Some(hook));
    Ok(())
}

/// The toy world for the config's mode: a circuit for the lap modes, free roam otherwise.
fn toy_world(config: &ServerConfig, args: &cli::Args) -> ToyWorld {
    let defaults = ToyConfig::default();
    let laps = config.settings.laps.max(1);
    let race = match config.settings.mode {
        Mode::Circuit | Mode::Sprint | Mode::Knockout | Mode::Speedtrap => Some(RaceConfig::circuit(laps)),
        _ => None,
    };
    let toy = ToyConfig {
        traffic: args.toy_traffic.unwrap_or(defaults.traffic),
        police: args.toy_police.unwrap_or(defaults.police),
        race,
        ..defaults
    };
    let seed = args
        .seed
        .unwrap_or_else(|| SystemTime::now().duration_since(UNIX_EPOCH).map_or(1, |since| since.as_nanos() as u64));
    ToyWorld::new(seed, toy)
}

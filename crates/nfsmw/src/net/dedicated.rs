//! `nfsmw --dedicated`: the same `Server` loop as the listen server, over the real world drawn by nobody, on the main
//! thread until Ctrl-C (or `--seconds`). No window, no audio, no GPU: the world's renderer is `NullBackend`. It logs to
//! stderr and the config's `log_file` (`docs/specs/multiplayer/server.md` section 6).

use std::sync::atomic::Ordering;

use anyhow::{Context, Result};
use blackbox_net_core::secure::IdentityKey;
use game_install::GameDir;
use nfsmw_server::{Server, logging};

use super::build;
use super::options::ServeOptions;
use crate::scenes::world::net::host::WorldSimHost;

/// The server's config, the logging it asks for: call before anything logs (the game's own logging is not installed in
/// a dedicated run, `main.rs`).
pub fn init_logging(options: &ServeOptions) -> Result<nfsmw_server::ServerConfig> {
    let config = options.server_config()?;
    logging::init(config.log_level, config.log_file.as_deref()).context("the server's logging")?;
    Ok(config)
}

/// Runs a dedicated server over `dir`'s world with `config` until Ctrl-C, or for `seconds`.
pub fn run(
    dir: &GameDir,
    options: &ServeOptions,
    config: nfsmw_server::ServerConfig,
    seconds: Option<u64>,
) -> Result<()> {
    let seed = options.seed();
    log::info!("dedicated: opening the world (seed {seed:#x})");
    let host = WorldSimHost::open(dir, options.host_config(seed)).context("the server's world did not open")?;
    let identity = match &config.identity_file {
        Some(path) => nfsmw_server::identity::load_or_create(path)?,
        None => IdentityKey::generate().map_err(|err| anyhow::anyhow!("no identity key: {err}"))?,
    };
    log::info!("identity {}", identity.key_id().fingerprint());
    let endpoint =
        nfsmw_server::bind(&config, identity).with_context(|| format!("cannot bind {}", config.game_addr()))?;
    log::info!("listening on {}", endpoint.local_addr());
    let cars = nfsmw_data::car::list(dir);
    let setup = build::server_setup(&cars, seed);
    let mut server =
        Server::new(config, setup, endpoint, host, 0).map_err(|err| anyhow::anyhow!("server settings: {err:?}"))?;
    let stop = server.stop_flag();
    let handler = ctrlc::set_handler(move || {
        log::info!("Ctrl-C: shutting down");
        stop.store(true, Ordering::Relaxed);
    });
    if let Err(error) = handler {
        log::warn!("no Ctrl-C handler ({error}); stop the server with --seconds or by closing the terminal");
    }
    server.run(seconds, None);
    Ok(())
}

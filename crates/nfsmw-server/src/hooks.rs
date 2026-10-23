//! The bot clients in the binary's loop (`--bots N`): a [`BotFleet`] on the server's own in-memory link (the listen
//! host's path, no socket and no handshake), stepped before every tick with the loop's clock.

use blackbox_net_core::endpoint::{ClientEndpoint, EndpointConfig};
use blackbox_net_core::secure::IdentityKey;
use nfsmw_sim::toy::ToyWorld;

use crate::bot::{Bot, BotConfig, BotFleet};
use crate::{Server, TickHook};

/// A hook the binary runs before every tick, or `None`.
pub type Hook = Option<Box<dyn TickHook<ToyWorld>>>;

/// The bots, stepped before each tick.
struct BotHook {
    fleet: BotFleet<ClientEndpoint>,
}

impl TickHook<ToyWorld> for BotHook {
    fn before_tick(&mut self, _server: &mut Server<ToyWorld>, now_ms: u64) {
        self.fleet.step(now_ms);
    }
}

/// Attaches `count` bot clients to `server`, each with a fresh identity key; `None` for 0.
pub fn attach_bots(server: &mut Server<ToyWorld>, count: usize) -> Result<Hook, String> {
    if count == 0 {
        return Ok(None);
    }
    let mut bots = Vec::with_capacity(count);
    for index in 0..count {
        let key = IdentityKey::generate().map_err(|err| format!("bot {index}: no identity: {err}"))?.key_id();
        let endpoint =
            server.connect_local(key, EndpointConfig::default()).map_err(|err| format!("bot {index}: {err}"))?;
        bots.push(Bot::over(endpoint, BotConfig::new(index as u32, 0xB07)));
    }
    log::info!("{count} bots attached");
    Ok(Some(Box::new(BotHook { fleet: BotFleet::from_bots(bots) })))
}

//! The game's multiplayer and the Discord presence (`crate::discord`): each frame the LAN game the player is seated in
//! goes to the presence as its party, and the servers friends invite the player to are joined.

use std::net::{IpAddr, Ipv4Addr, UdpSocket};
use std::time::{Duration, Instant};

use bevy_ecs::change_detection::DetectChangesMut;
use bevy_ecs::prelude::*;

use super::plugin::session;
use crate::discord::{DiscordJoins, PresenceParty, PresenceStatus};
use crate::frontend::lan::LanBackend;
use crate::settings::Settings;

/// How long this PC's LAN address is trusted before it is looked up again (a laptop changes networks).
const ADDRESS_EVERY: Duration = Duration::from_secs(10);

/// The address of this PC on the network its default route uses, or `None` without one. A UDP socket "connected" to
/// an address that no one owns (a documentation address) sends nothing, and still learns which interface would be used.
fn lan_address() -> Option<IpAddr> {
    let socket = UdpSocket::bind((Ipv4Addr::UNSPECIFIED, 0)).ok()?;
    socket.connect((Ipv4Addr::new(192, 0, 2, 1), 9)).ok()?;
    let address = socket.local_addr().ok()?.ip();
    (!address.is_unspecified()).then_some(address)
}

/// Tells the presence which LAN game the player is in, if any. A game this PC hosts is offered at the `net_public_address`
/// setting, else at this PC's LAN address.
pub(super) fn publish_party(
    mut backend: NonSendMut<LanBackend>,
    settings: Res<Settings>,
    status: Option<ResMut<PresenceStatus>>,
    party: Option<ResMut<PresenceParty>>,
    mut lan: Local<Option<(Instant, Option<IpAddr>)>>,
) {
    let (Some(mut status), Some(mut party)) = (status, party) else { return };
    let Some(session) = session(&mut backend) else { return };
    let address = match *lan {
        Some((at, address)) if at.elapsed() < ADDRESS_EVERY => address,
        _ => {
            let address = lan_address();
            *lan = Some((Instant::now(), address));
            address
        }
    };
    let game = session.lan_game(settings.net.public_ip().or(address));
    let wanted = match game {
        Some(_) => PresenceStatus::Lan,
        None => PresenceStatus::Playing,
    };
    status.set_if_neq(wanted);
    party.set_if_neq(PresenceParty(game));
}

/// Joins the servers friends invited the player to (their Discord's "Join"), when the join setting is on.
pub(super) fn accept_joins(
    joins: Option<Res<DiscordJoins>>,
    settings: Res<Settings>,
    mut backend: NonSendMut<LanBackend>,
) {
    let Some(joins) = joins else { return };
    let servers = joins.take();
    if servers.is_empty() {
        return;
    }
    if !(settings.discord && settings.discord_join) {
        log::info!("net: a Discord invite was ignored: Discord joining is off");
        return;
    }
    let Some(session) = session(&mut backend) else { return };
    for server in servers {
        session.join_from_discord(server);
    }
}

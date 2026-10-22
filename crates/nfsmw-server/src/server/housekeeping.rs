//! Once-a-second work: the LAN discovery reply and the status line.

use blackbox_net_core::discovery::{FLAG_DEDICATED, FLAG_IN_PROGRESS, FLAG_JOIN_IN_PROGRESS, Reply};
use nfsmw_net::message::{JoinInProgress, SessionPhase};
use nfsmw_net::version::{PROTO_MAJOR, PROTO_MINOR};

use super::Server;
use crate::control::WorldControl;

/// Ticks between two discovery reply updates.
const DISCOVERY_TICKS: u32 = 60;

impl<W: WorldControl> Server<W> {
    /// The discovery reply every second, the status line every `status_seconds`.
    pub(super) fn housekeeping(&mut self) {
        let tick = self.world.tick();
        if tick.is_multiple_of(DISCOVERY_TICKS) {
            self.refresh_discovery();
        }
        let period = u64::from(self.config.status_seconds) * u64::from(nfsmw_sim::TICK_HZ);
        if period > 0 && u64::from(tick) % period == 0 && tick > 0 {
            self.log_status(self.config.status_seconds);
        }
    }

    /// Answers LAN discovery with the session as it is now, or not at all when discovery is off.
    pub(super) fn refresh_discovery(&mut self) {
        if !self.config.discovery {
            return;
        }
        let settings = self.lobby.settings();
        let in_progress = self.lobby.wire_phase() != SessionPhase::Lobby;
        let flags = [
            (in_progress, FLAG_IN_PROGRESS),
            (settings.join_in_progress != JoinInProgress::Closed, FLAG_JOIN_IN_PROGRESS),
            (true, FLAG_DEDICATED),
        ]
        .iter()
        .filter(|(on, _)| *on)
        .fold(0, |flags, (_, bit)| flags | bit);
        let reply = Reply {
            proto_major: PROTO_MAJOR,
            nonce: 0,
            proto_minor: u8::try_from(PROTO_MINOR).unwrap_or(u8::MAX),
            key_hash: self.endpoint.key_id().short(),
            game_port: self.endpoint.local_addr().port(),
            players: self.lobby.racers().min(usize::from(u8::MAX)) as u8,
            max_players: settings.max_players,
            flags,
            mode: settings.mode.code(),
            data_id: self.setup.build.data_id.short(),
            name: settings.name.as_str().to_owned(),
        };
        self.endpoint.set_discovery_reply(Some(reply));
    }

    /// Logs one status line: players, tick cost, snapshot bandwidth per client over the last `seconds`, traffic.
    fn log_status(&mut self, seconds: u32) {
        let mut max_rate = 0u64;
        let mut total = 0u64;
        let mut clients = 0u64;
        for peer in self.peers.values_mut() {
            let rate = peer.window_bytes / u64::from(seconds.max(1));
            peer.window_bytes = 0;
            if peer.slot().is_none() {
                continue;
            }
            max_rate = max_rate.max(rate);
            total += rate;
            clients += 1;
        }
        let (bytes_in, bytes_out) =
            self.stats.endpoint.as_ref().map_or((0, 0), |stats| (stats.bytes_in, stats.bytes_out));
        log::info!(
            "status: tick {}, phase {:?}, {} racers, {} spectators, tick {} us mean / {} us max, snapshots {} B/s mean / \
             {} B/s max per client, net in {} B out {} B, {} invalid, {} strikes",
            self.world.tick(),
            self.lobby.wire_phase(),
            self.lobby.racers(),
            self.lobby.spectators(),
            self.stats.mean_tick_us(),
            self.stats.tick_time_max.as_micros(),
            total / clients.max(1),
            max_rate,
            bytes_in,
            bytes_out,
            self.stats.invalid_messages,
            self.stats.strikes,
        );
    }
}

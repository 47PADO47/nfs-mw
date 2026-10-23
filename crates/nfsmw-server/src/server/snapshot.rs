//! Writing each client's snapshot (protocol.md 6.3 as built, [`nfsmw_net::frames::write_snapshot`]): at the player rate, or the
//! spectator rate, staggered by slot so the clients of one server do not all fall on the same tick. The body is a
//! delta against the client's newest delivered view ([`blackbox_net_core::replicate::ClientState`]); the priority
//! accumulator fills the per-snapshot share of the client's byte budget.

use blackbox_net_core::bits::BitWriter;
use blackbox_net_core::session::PeerId;
use nfsmw_net::lobby::LobbyConfig;
use nfsmw_net::message::Role;
use nfsmw_net::schema::{CorrectionCause, OwnBlock};
use nfsmw_net::spectate::{RateClass, snapshot_hz};
use nfsmw_sim::PlayerId;

use super::MAX_SNAPSHOT_PAYLOAD;
use super::Server;
use crate::config::ServerConfig;
use crate::control::WorldControl;
use nfsmw_net::frames::{
    FRAME_SNAPSHOT, GLOBAL_BLOCK_BITS, GlobalBlock, SNAPSHOT_HEADER_BYTES, SnapshotHeader, write_snapshot,
};
use nfsmw_net::schema::OWN_BLOCK_BITS;

impl<W: WorldControl> Server<W> {
    /// Builds the tick's entity frame and sends every snapshot due.
    pub(super) fn send_snapshots(&mut self) {
        let tick = self.world.tick().saturating_sub(1);
        if self.slots.is_empty() && self.replay.is_none() {
            return;
        }
        self.frames.build_frame(&self.setup.schema, tick);
        let seated: Vec<(u8, PeerId)> = self.slots.iter().map(|(slot, peer)| (*slot, *peer)).collect();
        for (slot, peer) in seated {
            self.snapshot_to(slot, peer, tick);
        }
        let interval = ServerConfig::snapshot_interval(self.config.snapshot_hz);
        if let Some(replay) = self.replay.as_mut() {
            replay.snapshot(&mut self.frames, &self.setup.schema, tick, interval);
        }
    }

    /// The snapshot of `tick` to `slot`'s connection, if one is due.
    fn snapshot_to(&mut self, slot: u8, peer: PeerId, tick: u32) {
        let role = self.lobby.member(slot).map_or(Role::Spectator, |member| member.role);
        let hz = snapshot_hz(RateClass::of(role), self.config.snapshot_hz);
        let interval = ServerConfig::snapshot_interval(hz);
        if !(tick + u32::from(slot)).is_multiple_of(interval) {
            return;
        }
        let car = self.frames.car_view(slot).copied();
        let own_state = self.world.own_state(PlayerId(slot)).unwrap_or_default();
        let mut global = GlobalBlock {
            phase: self.lobby.wire_phase(),
            time_of_day: f32::from(LobbyConfig::default().time_of_day),
            ..GlobalBlock::default()
        };
        self.world.fill_global(PlayerId(slot), &mut global);
        let mut candidates = Vec::with_capacity(self.frames.framed_len());
        self.frames.candidates(car.map(|view| view.position), &mut candidates);
        let Some(state) = self.peers.get_mut(&peer) else {
            return;
        };
        if state.closing() {
            return;
        }
        let own = car.map(|view| OwnBlock::new(&view, own_state, state.cause, state.inputs.report()));
        let header = SnapshotHeader { part: 0, parts: 1, ack_input: state.last_input.map_or(0, |input| input as u16) };
        let blocks = SNAPSHOT_HEADER_BYTES + (GLOBAL_BLOCK_BITS + own.map_or(0, |_| OWN_BLOCK_BITS)) / 8;
        let share = (self.config.snapshot_budget / u32::from(hz.max(1))) as usize;
        let room = MAX_SNAPSHOT_PAYLOAD.saturating_sub(blocks);
        let budget_bits = share.min(MAX_SNAPSHOT_PAYLOAD).saturating_sub(blocks) * 8;
        let mut body = vec![0u8; room];
        let mut w = BitWriter::new(&mut body);
        let schema = self.setup.schema.schema();
        let frames = &mut self.frames;
        let written = state.replication.write_snapshot(
            schema,
            &frames.history,
            &mut frames.cache,
            tick,
            &candidates,
            budget_bits,
            &mut w,
        );
        if written.is_err() || w.align().is_err() {
            log::debug!("slot {slot}: snapshot of tick {tick} not written; the next one is full");
            self.stats.snapshot_errors += 1;
            state.resync();
            return;
        }
        let len = w.bytes_written();
        body.truncate(len);
        let mut payload = Vec::with_capacity(blocks + len);
        if let Err(err) = write_snapshot(&self.setup.schema, header, own.as_ref(), Some(&global), &body, &mut payload) {
            log::warn!("slot {slot}: no snapshot: {err}");
            state.resync();
            return;
        }
        let len = payload.len();
        Self::sent_snapshot(state, tick);
        state.cause = CorrectionCause::None;
        state.snapshots += 1;
        state.snapshot_bytes += len as u64;
        state.window_bytes += len as u64;
        self.stats.snapshots_sent += 1;
        self.stats.snapshot_bytes += len as u64;
        self.endpoint.send_unreliable(peer, FRAME_SNAPSHOT, payload, Some(tick));
    }
}

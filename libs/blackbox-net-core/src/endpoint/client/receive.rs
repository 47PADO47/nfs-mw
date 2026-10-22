//! The client's receive side: only datagrams from the server address are looked at. Handshake packets go to the
//! handshake; data packets to the connection, and the first one that opens confirms the keys.

use std::time::Duration;

use super::super::event::{ClientEvent, DisconnectReason};
use super::super::link::PeerLink;
use super::super::transport::Transport;
use super::LatestFrame;
use super::worker::{ClientWorker, LatestSlot};
use crate::MAX_DATAGRAM;
use crate::conn::{Connection, Sealer, parse_clear_header};
use crate::handoff::RingSender;
use crate::secure::{ClientStatus, Established, PacketKind, classify};

impl<T: Transport> ClientWorker<T> {
    /// Takes datagrams, waiting up to `wait` for the first.
    pub(super) fn receive(&mut self, buf: &mut [u8], wait: Duration) -> Result<bool, DisconnectReason> {
        let mut wait = wait;
        let mut work = false;
        for _ in 0..self.config.max_datagrams_per_pass.max(1) {
            let received = self.wire.net.recv_from(buf, wait);
            let (from, len) = match received {
                Ok(Some(got)) => got,
                Ok(None) => return Ok(work),
                Err(_) => {
                    self.counters.recv_errors += 1;
                    return Ok(work);
                }
            };
            wait = Duration::ZERO;
            work = true;
            self.counters.datagrams_in += 1;
            self.counters.bytes_in += len as u64;
            if from != self.server || len > MAX_DATAGRAM {
                self.counters.garbage += 1;
                continue;
            }
            let Some(datagram) = buf.get_mut(..len) else {
                continue;
            };
            let now = self.clock.now_ms();
            match classify(datagram) {
                PacketKind::Handshake(_) => self.on_handshake(datagram)?,
                PacketKind::Data => self.on_data(now, datagram)?,
                PacketKind::Invalid => self.counters.garbage += 1,
            }
        }
        Ok(work)
    }

    /// A handshake datagram. When the keys exist the sealed connection is built; it opens on the first sealed packet.
    fn on_handshake(&mut self, datagram: &[u8]) -> Result<(), DisconnectReason> {
        let Some(handshake) = self.handshake.as_mut() else {
            return Ok(());
        };
        if let ClientStatus::Failed(err) = handshake.handle(self.server, datagram) {
            return Err(DisconnectReason::Handshake(err));
        }
        let Some(Established { info, sealer }) = handshake.take_established() else {
            return Ok(());
        };
        let sealer: Box<dyn Sealer + Send> = Box::new(sealer);
        let conn = Connection::with_sealer(self.config.conn(info.local_conn_id, info.remote_conn_id), sealer);
        self.link = Some(PeerLink::new(conn, self.server, info.local_conn_id, None, false));
        self.server_key = Some(info.remote_id());
        Ok(())
    }

    /// A data packet. Frames of a packet that opened are authentic, so the first one also confirms the connection
    /// (reported before the frames it carries).
    fn on_data(&mut self, now_ms: u64, datagram: &mut [u8]) -> Result<(), DisconnectReason> {
        let Some(link) = self.link.as_mut() else {
            self.counters.unknown_connection += 1;
            return Ok(());
        };
        let Ok(clear) = parse_clear_header(datagram) else {
            self.counters.garbage += 1;
            return Ok(());
        };
        if clear.conn_id != link.local_id() {
            self.counters.unknown_connection += 1;
            return Ok(());
        }
        let mut announce = match self.connected {
            true => None,
            false => self.server_key,
        };
        let opened_by_frame = announce.is_some();
        let events = &mut self.events;
        let latest = &mut self.latest;
        let read = link.read(now_ms, datagram, |frame_type, payload| {
            if let Some(server_key) = announce.take() {
                events.push(ClientEvent::Connected { server_key });
            }
            deliver_frame(events, latest, frame_type, payload);
        });
        let announced = opened_by_frame && announce.is_none();
        if read.is_err() {
            self.counters.rejected += 1;
        }
        if announced || (read.is_ok() && !self.connected) {
            self.confirm(announced);
        }
        self.drain()
    }

    /// Opens the connection after its first authentic packet; `announced` when the event already went out.
    fn confirm(&mut self, announced: bool) {
        if announced {
            self.server_key = None;
        }
        self.open();
    }
}

/// Hands one unreliable frame to the game: the latest-value cell for its frame type, the event queue otherwise.
fn deliver_frame(events: &mut RingSender<ClientEvent>, latest: &mut LatestSlot, frame_type: u8, payload: &[u8]) {
    if let Some((latest_type, writer)) = latest.as_mut()
        && *latest_type == frame_type
    {
        let seq = writer.published() + 1;
        let slot: &mut LatestFrame = writer.buffer_mut();
        slot.seq = seq;
        slot.frame_type = frame_type;
        slot.bytes.clear();
        slot.bytes.extend_from_slice(payload);
        writer.publish_in_place();
        return;
    }
    events.push(ClientEvent::Unreliable { frame_type, bytes: payload.to_vec() });
}

//! One connection as the net thread drives it, the same on both sides: the sealed [`Connection`], the peer's address
//! and inbound budget, the unreliable frames waiting for the next flush, and the tags of the packets that carried them.

use std::collections::VecDeque;
use std::net::SocketAddr;

use super::path::PathProbe;
use super::stats::PeerStats;
use crate::conn::{
    ConnError, ConnEvent, Connection, FRAME_DISCONNECT, FRAME_PADDING, FRAME_RELIABLE, FRAME_SPACE, HEADER_LEN,
    PATH_TOKEN_LEN, Sealer,
};
use crate::ratelimit::PeerBudget;
use crate::session::PeerId;

/// A connection whose sealer is chosen at run time: Noise on the network, plain on the in-memory link.
pub(crate) type DynConnection = Connection<Box<dyn Sealer + Send>>;

/// Unreliable frames in one packet at most.
const MAX_FRAMES_PER_PACKET: usize = 64;
/// Packets of due reliable fragments sent in one flush at most.
const MAX_RELIABLE_PACKETS: usize = 64;
/// Packet-to-tag records kept; the oldest go first.
const MAX_TAGS: usize = 1024;
/// Copies of the DISCONNECT packet sent when closing, since the frame itself is not retransmitted.
const CLOSE_REPEATS: usize = 3;
/// While a new server-side connection has not heard from its client, an acknowledgement-only packet goes out this
/// often, so a client whose first confirmation was lost still confirms its keys well inside its retry window.
const CONFIRM_EVERY_MS: u64 = 100;

/// What a connection reports besides messages and frames.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum LinkEvent {
    /// A tagged frame's packet was acknowledged.
    Delivered(u32),
    /// A tagged frame's packet was written off.
    Lost(u32),
    /// The other side sent DISCONNECT with this reason.
    Closed(u8),
}

/// An unreliable frame waiting for the next flush.
#[derive(Debug)]
struct Pending {
    frame_type: u8,
    bytes: Vec<u8>,
    tag: Option<u32>,
}

/// One driven connection.
pub(crate) struct PeerLink {
    conn: DynConnection,
    addr: SocketAddr,
    /// The address the handshake ran with: what the pre-authentication gate counted this connection against, which a
    /// move to another address does not change.
    admitted: SocketAddr,
    local_id: u16,
    budget: Option<PeerBudget>,
    unreliable: Vec<Pending>,
    tags: VecDeque<(u32, u32)>,
    ack_wanted: bool,
    confirm_next_ms: Option<u64>,
    path: PathProbe,
}

impl PeerLink {
    /// A link to `addr` over `conn`, answering to `local_id`. `budget` limits inbound datagrams (none on the in-memory
    /// link); `confirm` makes it send acknowledgement-only packets until the peer is heard from.
    pub(crate) fn new(
        conn: DynConnection,
        addr: SocketAddr,
        local_id: u16,
        budget: Option<PeerBudget>,
        confirm: bool,
    ) -> Self {
        Self {
            conn,
            addr,
            admitted: addr,
            local_id,
            budget,
            unreliable: Vec::new(),
            tags: VecDeque::new(),
            ack_wanted: false,
            confirm_next_ms: confirm.then_some(0),
            path: PathProbe::default(),
        }
    }

    /// The peer's address.
    pub(crate) fn addr(&self) -> SocketAddr {
        self.addr
    }

    /// The address the handshake ran with, for the gate's per-address count when the connection ends.
    pub(crate) fn admitted_addr(&self) -> SocketAddr {
        self.admitted
    }

    /// The connection id this side answers to.
    pub(crate) fn local_id(&self) -> u16 {
        self.local_id
    }

    /// Replaces the inbound budget.
    pub(crate) fn set_budget(&mut self, budget: PeerBudget) {
        self.budget = Some(budget);
    }

    /// Starts a new budget tick.
    pub(crate) fn start_tick(&mut self) {
        if let Some(budget) = self.budget.as_mut() {
            budget.start_tick();
        }
    }

    /// Whether a datagram of `len` bytes is within the inbound budget; counts it when it is.
    pub(crate) fn admit(&mut self, now_ms: u64, len: usize) -> bool {
        self.budget.as_mut().is_none_or(|budget| budget.accept(now_ms, len))
    }

    /// Whether the peer has been silent for the idle timeout.
    pub(crate) fn is_timed_out(&self, now_ms: u64) -> bool {
        self.conn.is_timed_out(now_ms)
    }

    /// Queues a reliable message.
    pub(crate) fn send_reliable(&mut self, channel: u8, bytes: &[u8]) -> Result<(), ConnError> {
        self.conn.send_message(channel, bytes)
    }

    /// Queues an unreliable frame for the next flush. `false` (nothing queued) for a frame type the connection owns
    /// or a payload that cannot fit in one packet.
    pub(crate) fn queue_unreliable(&mut self, frame_type: u8, bytes: Vec<u8>, tag: Option<u32>) -> bool {
        let reserved = frame_type == FRAME_PADDING || (FRAME_RELIABLE..=FRAME_DISCONNECT + 2).contains(&frame_type);
        if reserved || frame_size(bytes.len()) > FRAME_SPACE {
            return false;
        }
        self.unreliable.push(Pending { frame_type, bytes, tag });
        true
    }

    /// Whether a packet from an address the peer is not at may be opened now (path validation's cheap limit).
    pub(crate) fn admit_foreign(&mut self, now_ms: u64) -> bool {
        self.path.admit_foreign(now_ms)
    }

    /// An authenticated packet came from `from`, not the peer's address: asks `from` to prove itself, and moves the
    /// peer there if the packet itself carried the answer to an earlier challenge. Returns whether the peer moved.
    pub(crate) fn path_heard_from(&mut self, now_ms: u64, from: SocketAddr) -> bool {
        if let Some(token) = self.conn.take_path_response()
            && self.path.confirm(now_ms, from, &token)
        {
            self.addr = from;
            return true;
        }
        self.path.offer(now_ms, from, new_token);
        false
    }

    /// The challenge due for a candidate address, written into `out`: the address and the packet's length.
    pub(crate) fn path_challenge(&mut self, now_ms: u64, out: &mut [u8]) -> Option<(SocketAddr, usize)> {
        let (addr, token) = self.path.probe_due(now_ms)?;
        let sent = self.conn.write_path_challenge(now_ms, &token, out).ok()?;
        Some((addr, sent.len))
    }

    /// Takes in one datagram (opened in place). Unreliable frames go to `on_frame`; messages and events wait for
    /// [`PeerLink::take_message`] and [`PeerLink::drain_events`].
    pub(crate) fn read<'a, F: FnMut(u8, &'a [u8])>(
        &mut self,
        now_ms: u64,
        datagram: &'a mut [u8],
        on_frame: F,
    ) -> Result<(), ConnError> {
        let len = datagram.len();
        self.conn.read_packet(now_ms, datagram, on_frame)?;
        self.confirm_next_ms = None;
        // A packet with frames wants an acknowledgement soon; an acknowledgement-only packet does not, or two idle
        // ends would bounce acknowledgements forever.
        if len > HEADER_LEN + self.conn.sealer().overhead() {
            self.ack_wanted = true;
        }
        Ok(())
    }

    /// The next complete reliable message and its channel.
    pub(crate) fn take_message(&mut self) -> Option<(u8, Vec<u8>)> {
        let mut bytes = Vec::new();
        let channel = self.conn.recv_message(&mut bytes)?;
        Some((channel, bytes))
    }

    /// Reports every pending connection event: tagged deliveries and losses, and a DISCONNECT from the other side.
    pub(crate) fn drain_events<F: FnMut(LinkEvent)>(&mut self, mut on_event: F) {
        while let Some(event) = self.conn.poll_event() {
            let (number, delivered) = match event {
                ConnEvent::Delivered(number) => (number, true),
                ConnEvent::Lost(number) => (number, false),
                ConnEvent::Disconnected(reason) => {
                    on_event(LinkEvent::Closed(reason));
                    continue;
                }
            };
            self.tags.retain(|&(packet, tag)| {
                if packet != number {
                    return true;
                }
                on_event(match delivered {
                    true => LinkEvent::Delivered(tag),
                    false => LinkEvent::Lost(tag),
                });
                false
            });
        }
    }

    /// Builds and sends what is due: the queued unreliable frames (packed several to a packet), the due reliable
    /// fragments, and an acknowledgement-only packet when one is wanted, a keep-alive is due, or the peer still has to
    /// confirm its keys. `out` holds one datagram. Returns whether anything was sent; `bad` counts frames dropped.
    pub(crate) fn flush<S: FnMut(&[u8])>(&mut self, now_ms: u64, out: &mut [u8], bad: &mut u64, mut send: S) -> bool {
        let mut sent_any = self.flush_unreliable(now_ms, out, bad, &mut send);
        for _ in 0..MAX_RELIABLE_PACKETS {
            if !self.conn.has_due_reliable(now_ms) {
                break;
            }
            let Ok(sent) = self.conn.write_packet(now_ms, &[], out) else {
                break;
            };
            send(&out[..sent.len]);
            sent_any = true;
        }
        let confirm = self.confirm_due(now_ms);
        let wanted = self.ack_wanted || confirm || self.conn.wants_keepalive(now_ms);
        if !sent_any
            && wanted
            && let Ok(sent) = self.conn.write_packet(now_ms, &[], out)
        {
            send(&out[..sent.len]);
            sent_any = true;
        }
        if sent_any {
            self.ack_wanted = false;
        }
        sent_any
    }

    /// Sends a DISCONNECT with `reason` a few times; the caller forgets the link afterwards.
    pub(crate) fn close_now<S: FnMut(&[u8])>(&mut self, now_ms: u64, reason: u8, out: &mut [u8], mut send: S) {
        for _ in 0..CLOSE_REPEATS {
            self.conn.close(reason);
            let Ok(sent) = self.conn.write_packet(now_ms, &[], out) else {
                return;
            };
            send(&out[..sent.len]);
        }
    }

    /// This connection's numbers.
    pub(crate) fn stats(&self, peer: PeerId) -> PeerStats {
        PeerStats {
            peer,
            rtt_ms: self.conn.rtt_ms(),
            loss: self.conn.loss(),
            bytes_sent: self.conn.bytes_sent(),
            bytes_received: self.conn.bytes_received(),
            pending_reliable: self.conn.pending_messages(),
        }
    }

    /// Whether an acknowledgement-only packet for key confirmation is due; moves the timer on when it is.
    fn confirm_due(&mut self, now_ms: u64) -> bool {
        let Some(next) = self.confirm_next_ms else {
            return false;
        };
        if now_ms < next {
            return false;
        }
        self.confirm_next_ms = Some(now_ms + CONFIRM_EVERY_MS);
        true
    }

    /// Packs the queued unreliable frames into as few packets as they fit and sends them.
    fn flush_unreliable<S: FnMut(&[u8])>(&mut self, now_ms: u64, out: &mut [u8], bad: &mut u64, send: &mut S) -> bool {
        let mut pending = std::mem::take(&mut self.unreliable);
        let mut sent_any = false;
        let mut start = 0;
        while start < pending.len() {
            let mut frames: [(u8, &[u8]); MAX_FRAMES_PER_PACKET] = [(0, &[]); MAX_FRAMES_PER_PACKET];
            let mut count = 0;
            let mut space = 0;
            for item in pending.iter().skip(start).take(MAX_FRAMES_PER_PACKET) {
                let size = frame_size(item.bytes.len());
                if space + size > FRAME_SPACE {
                    break;
                }
                space += size;
                frames[count] = (item.frame_type, item.bytes.as_slice());
                count += 1;
            }
            if count == 0 {
                *bad += 1;
                start += 1;
                continue;
            }
            let written = self.conn.write_packet(now_ms, &frames[..count], out);
            let end = start + count;
            start = end;
            let Ok(sent) = written else {
                *bad += count as u64;
                continue;
            };
            send(&out[..sent.len]);
            sent_any = true;
            for item in pending.iter().take(end).skip(end - count) {
                let Some(tag) = item.tag else {
                    continue;
                };
                if self.tags.len() >= MAX_TAGS {
                    self.tags.pop_front();
                }
                self.tags.push_back((sent.number, tag));
            }
        }
        pending.clear();
        self.unreliable = pending;
        sent_any
    }
}

/// A fresh path token, `None` if the system has no randomness.
fn new_token() -> Option<[u8; PATH_TOKEN_LEN]> {
    let mut token = [0u8; PATH_TOKEN_LEN];
    getrandom::fill(&mut token).ok()?;
    Some(token)
}

/// Bytes of a frame with a `len`-byte payload: type, the LEB128 length, the payload.
fn frame_size(len: usize) -> usize {
    let varint = match len {
        0..0x80 => 1,
        0x80..0x4000 => 2,
        _ => 3,
    };
    1 + varint + len
}

impl std::fmt::Debug for PeerLink {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PeerLink")
            .field("addr", &self.addr)
            .field("local_id", &self.local_id)
            .field("queued", &self.unreliable.len())
            .finish_non_exhaustive()
    }
}

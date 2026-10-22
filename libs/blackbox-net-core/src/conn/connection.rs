//! The connection: packet numbers and acknowledgements, the reliable channels, and the events and messages the caller reads
//! back. Nothing here reads a clock or touches a socket; every time comes in as `now_ms`.

use std::collections::VecDeque;

use super::acks::{RecvWindow, SequenceBuffer, seq_newer};
use super::channel::{FLAG_FIRST, FLAG_LAST, RecvChannel, SendChannel};
use super::header::{FLAG_ACK_VALID, Header, frame_size, parse_header, read_varint, write_frame, write_header};
use super::rtt::Rtt;
use super::seal::{Plain, Sealer, parse_clear_header};
use super::{
    CHANNELS, ConnConfig, ConnError, ConnEvent, FRAME_DISCONNECT, FRAME_PADDING, FRAME_RELIABLE, FRAME_SPACE,
    HEADER_LEN, Sent,
};
use crate::MAX_DATAGRAM;

/// Packet records kept for matching acknowledgements: the same 1024 numbers the receive window covers.
const SENT_WINDOW: usize = 1024;
/// Reliable frames one packet may carry.
const MAX_PACKET_FRAGS: usize = 16;
/// A packet still unacknowledged this long after it was sent is lost.
const LOSS_AGE_MS: u64 = 1000;
/// A packet this many numbers older than the newest acknowledged one is lost.
const LOSS_GAP: u32 = 3;
/// Events queued before the queue grows.
const EVENT_CAPACITY: usize = 1024;

/// What one sent packet carried: its send time and the reliable fragments in it, as (channel, message id).
#[derive(Debug, Clone, Copy)]
struct SentPacket {
    sent_ms: u64,
    count: usize,
    frags: [(u8, u16); MAX_PACKET_FRAGS],
}

/// One end of a connection. Feed it datagrams with [`Connection::read_packet`] and build the ones to send with
/// [`Connection::write_packet`]; queue application messages with [`Connection::send_message`] and collect the complete
/// incoming ones with [`Connection::recv_message`]. `S` seals and opens the datagrams: [`Plain`] for the in-memory link,
/// a Noise sealer for the network.
#[derive(Debug)]
pub struct Connection<S: Sealer = Plain> {
    config: ConnConfig,
    sealer: S,
    next_number: u32,
    window: RecvWindow,
    sent: SequenceBuffer<SentPacket>,
    highest_acked: Option<u32>,
    rtt: Rtt,
    send: [SendChannel; CHANNELS],
    recv: [RecvChannel; CHANNELS],
    events: VecDeque<ConnEvent>,
    disconnect: Option<u8>,
    created_ms: Option<u64>,
    last_send_ms: Option<u64>,
    last_recv_ms: Option<u64>,
    bytes_sent: u64,
    bytes_received: u64,
}

impl Connection<Plain> {
    /// A fresh unsealed connection (the in-memory link) with packet number 0 next and no history.
    pub fn new(config: ConnConfig) -> Self {
        Self::with_sealer(config, Plain)
    }
}

impl<S: Sealer> Connection<S> {
    /// A fresh connection whose datagrams `sealer` seals and opens.
    pub fn with_sealer(config: ConnConfig, sealer: S) -> Self {
        Self {
            config,
            sealer,
            next_number: 0,
            window: RecvWindow::default(),
            sent: SequenceBuffer::with_capacity(SENT_WINDOW),
            highest_acked: None,
            rtt: Rtt::default(),
            send: Default::default(),
            recv: Default::default(),
            events: VecDeque::with_capacity(EVENT_CAPACITY),
            disconnect: None,
            created_ms: None,
            last_send_ms: None,
            last_recv_ms: None,
            bytes_sent: 0,
            bytes_received: 0,
        }
    }

    /// Queues `msg` on `channel` (0 to 3) for reliable, ordered delivery. Fails with [`ConnError::BadChannel`],
    /// [`ConnError::MessageTooLarge`] or [`ConnError::QueueFull`] and queues nothing.
    pub fn send_message(&mut self, channel: u8, msg: &[u8]) -> Result<(), ConnError> {
        let Some(queue) = self.send.get_mut(usize::from(channel)) else {
            return Err(ConnError::BadChannel);
        };
        queue.push(msg)
    }

    /// Copies the next complete incoming message into `out` (cleared first) and returns its channel. Channels are
    /// searched from 0 to 3; within a channel messages come in the order the peer sent them. `None` when nothing is
    /// complete (`out` is still cleared).
    pub fn recv_message(&mut self, out: &mut Vec<u8>) -> Option<u8> {
        out.clear();
        for (channel, queue) in self.recv.iter_mut().enumerate() {
            let Some(msg) = queue.pop_ready() else {
                continue;
            };
            out.extend_from_slice(&msg);
            return u8::try_from(channel).ok();
        }
        None
    }

    /// Builds the next packet into `out` and returns its number and length. The packet carries the acknowledgement block,
    /// then `app_frames` in order (`FrameTooLarge` if they exceed [`FRAME_SPACE`], with no packet number used), then
    /// a pending disconnect, then the reliable fragments that are due, oldest first on channel 0 to 3, while they fit and
    /// the packet has fewer than 16. Frame types the connection owns (`0x10` to `0x13`) are not checked here and must not
    /// be passed by the caller. Always writes a packet, even one with only acknowledgements. `out` must hold a whole
    /// datagram ([`MAX_DATAGRAM`] bytes). Packets that are not acknowledged within 1 s, or 3 behind the newest
    /// acknowledged number, are written off as [`ConnEvent::Lost`] during the call. The packet is sealed last; a seal
    /// failure ([`ConnError::SealFailed`]) uses no packet number, though due fragments count as sent and are resent later.
    pub fn write_packet(&mut self, now_ms: u64, app_frames: &[(u8, &[u8])], out: &mut [u8]) -> Result<Sent, ConnError> {
        if out.len() < MAX_DATAGRAM {
            return Err(ConnError::BufferTooSmall);
        }
        let mut app_len = 0usize;
        for &(_, payload) in app_frames {
            app_len += frame_size(payload.len());
            if app_len > FRAME_SPACE {
                return Err(ConnError::FrameTooLarge);
            }
        }
        if self.created_ms.is_none() {
            self.created_ms = Some(now_ms);
        }
        self.scan_losses(now_ms);

        let limit = HEADER_LEN + FRAME_SPACE;
        let mut at = HEADER_LEN;
        for &(ftype, payload) in app_frames {
            at = write_frame(out, at, ftype, &[payload]).ok_or(ConnError::BufferTooSmall)?;
        }
        if let Some(reason) = self.disconnect {
            let reason_byte = [reason];
            if at + frame_size(1) <= limit {
                at = write_frame(out, at, FRAME_DISCONNECT, &[&reason_byte[..]]).ok_or(ConnError::BufferTooSmall)?;
                self.disconnect = None;
            }
        }

        let rto = self.rtt.resend_after_ms();
        let mut frags = [(0u8, 0u16); MAX_PACKET_FRAGS];
        let mut count = 0usize;
        'channels: for channel in 0..CHANNELS {
            let channel_byte = u8::try_from(channel).unwrap_or(0);
            while let Some(queue) = self.send.get(channel) {
                let Some((pos, index)) = queue.next_due(now_ms, rto) else {
                    break;
                };
                let Some(part) = queue.fragment(pos, index) else {
                    break;
                };
                let Some(slot) = frags.get_mut(count) else {
                    break 'channels;
                };
                let prefix: &[u8] = match part.prefix.as_ref() {
                    Some(bytes) => bytes.as_slice(),
                    None => &[],
                };
                if at + frame_size(3 + prefix.len() + part.data.len()) > limit {
                    break 'channels;
                }
                let id = part.id.to_le_bytes();
                let head = [(channel_byte << 6) | part.flags, id[0], id[1]];
                at = write_frame(out, at, FRAME_RELIABLE, &[&head[..], prefix, part.data])
                    .ok_or(ConnError::BufferTooSmall)?;
                *slot = (channel_byte, part.id);
                count += 1;
                if let Some(queue) = self.send.get_mut(channel) {
                    queue.mark_sent(pos, index, now_ms);
                }
            }
        }

        let number = self.next_number;
        let header = Header {
            flags: FLAG_ACK_VALID * u8::from(self.window.any()),
            conn_id: self.config.remote_id,
            number,
            ack: self.window.ack(),
            ack_bits: self.window.ack_bits(),
        };
        write_header(out, &header)?;
        if at + self.sealer.overhead() > MAX_DATAGRAM {
            return Err(ConnError::SealFailed);
        }
        let len = self.sealer.seal(now_ms, out, at)?;

        let record = SentPacket { sent_ms: now_ms, count, frags };
        if let Some((old, _)) = self.sent.insert(number, record) {
            self.events.push_back(ConnEvent::Lost(old));
            self.rtt.record_outcome(true);
        }
        self.next_number = self.next_number.wrapping_add(1);
        self.bytes_sent = self.bytes_sent.saturating_add(len as u64);
        self.last_send_ms = Some(now_ms);
        Ok(Sent { number, len })
    }

    /// Takes in one datagram, opened in place in `datagram`. The clear header is checked (connection id, and the window
    /// is asked whether the number is new) before the sealer opens it; the number is recorded only once the tag
    /// verified. Then the acknowledgements are applied and each frame is handled in order: reliable frames go to the
    /// channels, a disconnect becomes [`ConnEvent::Disconnected`], the reserved types `0x12` and `0x13` are skipped, and
    /// every other frame is passed to `on_frame` with its payload. Returns the packet number. On
    /// [`ConnError::BadFrame`] the frames before the bad one stay handled.
    pub fn read_packet<'a, F: FnMut(u8, &'a [u8])>(
        &mut self,
        now_ms: u64,
        datagram: &'a mut [u8],
        mut on_frame: F,
    ) -> Result<u32, ConnError> {
        if datagram.len() < HEADER_LEN + self.sealer.overhead() {
            return Err(ConnError::Truncated);
        }
        let clear = parse_clear_header(datagram)?;
        if clear.conn_id != self.config.local_id {
            return Err(ConnError::WrongConnection);
        }
        self.window.check(clear.number)?;
        let received = datagram.len();
        let plain_len = self.sealer.open(now_ms, datagram)?;
        let datagram: &'a [u8] = datagram;
        let Some(datagram) = datagram.get(..plain_len) else {
            return Err(ConnError::Truncated);
        };
        let header = parse_header(datagram)?;
        self.window.record(header.number)?;
        if self.created_ms.is_none() {
            self.created_ms = Some(now_ms);
        }
        self.last_recv_ms = Some(now_ms);
        self.bytes_received = self.bytes_received.saturating_add(received as u64);
        if header.flags & FLAG_ACK_VALID != 0 {
            self.apply_acks(now_ms, header.ack, header.ack_bits);
        }
        self.parse_frames(datagram, &mut on_frame)?;
        Ok(header.number)
    }

    /// The next event (acknowledged, lost or disconnected), oldest first.
    pub fn poll_event(&mut self) -> Option<ConnEvent> {
        self.events.pop_front()
    }

    /// Asks for a disconnect with `reason`: one DISCONNECT frame goes out in the next packet that has room for it.
    pub fn close(&mut self, reason: u8) {
        self.disconnect = Some(reason);
    }

    /// Whether the peer has been silent for `idle_timeout_ms`. The clock starts at the first call that took a time
    /// (a write or a read); before that the connection is never timed out.
    pub fn is_timed_out(&self, now_ms: u64) -> bool {
        let Some(base) = self.last_recv_ms.or(self.created_ms) else {
            return false;
        };
        now_ms.saturating_sub(base) >= self.config.idle_timeout_ms
    }

    /// Whether `keepalive_ms` have passed since the last packet was sent, so an acknowledgement-only packet is due.
    pub fn wants_keepalive(&self, now_ms: u64) -> bool {
        let Some(base) = self.last_send_ms.or(self.created_ms) else {
            return false;
        };
        now_ms.saturating_sub(base) >= self.config.keepalive_ms
    }

    /// Whether any reliable fragment would be sent by the next [`Connection::write_packet`].
    pub fn has_due_reliable(&self, now_ms: u64) -> bool {
        let rto = self.rtt.resend_after_ms();
        self.send.iter().any(|queue| queue.has_due(now_ms, rto))
    }

    /// Messages queued for sending that are not yet fully acknowledged, over all channels.
    pub fn pending_messages(&self) -> usize {
        self.send.iter().map(SendChannel::pending).sum()
    }

    /// Smoothed round-trip time in milliseconds.
    pub fn rtt_ms(&self) -> f64 {
        self.rtt.srtt_ms()
    }

    /// Round-trip time variance in milliseconds.
    pub fn rtt_var_ms(&self) -> f64 {
        self.rtt.rttvar_ms()
    }

    /// Loss average between 0 and 1 (lost outcomes count 1, delivered ones 0).
    pub fn loss(&self) -> f64 {
        self.rtt.loss()
    }

    /// Bytes of packets written so far.
    pub fn bytes_sent(&self) -> u64 {
        self.bytes_sent
    }

    /// The sealer, for its counters and key state.
    pub fn sealer(&self) -> &S {
        &self.sealer
    }

    /// Bytes of accepted datagrams read so far.
    pub fn bytes_received(&self) -> u64 {
        self.bytes_received
    }

    /// Handles the frames of an accepted datagram after its header.
    fn parse_frames<'a, F: FnMut(u8, &'a [u8])>(
        &mut self,
        datagram: &'a [u8],
        on_frame: &mut F,
    ) -> Result<(), ConnError> {
        let mut at = HEADER_LEN;
        loop {
            let Some(&ftype) = datagram.get(at) else {
                return Ok(());
            };
            if ftype == FRAME_PADDING {
                at += 1;
                continue;
            }
            let Some(rest) = datagram.get(at + 1..) else {
                return Ok(());
            };
            let (len, used) = read_varint(rest)?;
            let start = at + 1 + used;
            let Some(payload) = datagram.get(start..start + len) else {
                return Err(ConnError::BadFrame);
            };
            at = start + len;
            match ftype {
                FRAME_RELIABLE => self.receive_reliable(payload)?,
                FRAME_DISCONNECT => {
                    let Some(&reason) = payload.first() else {
                        return Err(ConnError::BadFrame);
                    };
                    self.events.push_back(ConnEvent::Disconnected(reason));
                }
                0x12 | 0x13 => {}
                _ => on_frame(ftype, payload),
            }
        }
    }

    /// Takes one reliable frame payload: channel and flags, message id, then data.
    fn receive_reliable(&mut self, payload: &[u8]) -> Result<(), ConnError> {
        let Some(&head) = payload.first() else {
            return Err(ConnError::BadFrame);
        };
        if (head & 0x0F) != 0 {
            return Err(ConnError::BadFrame);
        }
        let Some(&[lo, hi]) = payload.get(1..3) else {
            return Err(ConnError::BadFrame);
        };
        let Some(data) = payload.get(3..) else {
            return Err(ConnError::BadFrame);
        };
        let Some(channel) = self.recv.get_mut(usize::from(head >> 6)) else {
            return Err(ConnError::BadChannel);
        };
        let first = (head & FLAG_FIRST) != 0;
        let last = (head & FLAG_LAST) != 0;
        channel.receive(u16::from_le_bytes([lo, hi]), first, last, data)
    }

    /// Applies the acknowledgement block: each newly acknowledged packet is reported and its fragments are marked
    /// delivered; the newest of them gives the RTT sample. Then the losses are scanned.
    fn apply_acks(&mut self, now_ms: u64, ack: u32, ack_bits: u32) {
        // An acknowledgement of a number not yet sent is a peer bug or forgery; it must not write off packets in flight.
        if self.next_number == 0 || !seq_newer(self.next_number, ack) {
            return;
        }
        let older = (0..32u32).filter(move |&i| (ack_bits >> i) & 1 == 1).map(|i| ack.wrapping_sub(i + 1));
        let mut newest_sent: Option<u64> = None;
        for number in std::iter::once(ack).chain(older) {
            let Some(sent_ms) = self.ack_one(number) else {
                continue;
            };
            if newest_sent.is_none() {
                newest_sent = Some(sent_ms);
            }
        }
        if let Some(sent_ms) = newest_sent {
            self.rtt.sample(now_ms.saturating_sub(sent_ms) as f64);
        }
        if self.highest_acked.is_none_or(|highest| seq_newer(ack, highest)) {
            self.highest_acked = Some(ack);
        }
        self.scan_losses(now_ms);
    }

    /// Acknowledges one packet if it is still outstanding. Returns its send time when it was.
    fn ack_one(&mut self, number: u32) -> Option<u64> {
        let record = self.sent.remove(number)?;
        self.events.push_back(ConnEvent::Delivered(number));
        self.rtt.record_outcome(false);
        for &(channel, id) in record.frags.iter().take(record.count) {
            if let Some(queue) = self.send.get_mut(usize::from(channel)) {
                queue.mark_delivered(id);
            }
        }
        Some(record.sent_ms)
    }

    /// Writes off outstanding packets that are 3 numbers behind the newest acknowledged one or older than 1 s. Each is
    /// reported once and forgotten; its fragments stay queued for resending.
    fn scan_losses(&mut self, now_ms: u64) {
        let highest = self.highest_acked;
        let rtt = &mut self.rtt;
        let events = &mut self.events;
        self.sent.drain_matching(
            |seq, record| {
                let behind = highest.is_some_and(|h| seq_newer(h, seq) && h.wrapping_sub(seq) >= LOSS_GAP);
                behind || now_ms.saturating_sub(record.sent_ms) > LOSS_AGE_MS
            },
            |seq, _| {
                events.push_back(ConnEvent::Lost(seq));
                rtt.record_outcome(true);
            },
        );
    }
}

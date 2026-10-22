//! The reliable channels. The send side keeps each message until every fragment is acknowledged, with per-fragment state
//! and resend timers. The receive side holds fragments inside a window of [`WINDOW`] ids, assembles messages once their
//! fragments are all present, and hands them out in order.

use std::collections::VecDeque;

use super::fragment::{declared_total, fragment_count, fragment_span};
use super::{ConnError, MAX_FRAGMENT, MAX_FRAGMENTS, MAX_IN_FLIGHT};

/// Fragment ids the receive window covers: [`MAX_IN_FLIGHT`] messages of up to [`MAX_FRAGMENTS`] fragments each.
const WINDOW: usize = MAX_IN_FLIGHT * MAX_FRAGMENTS;

/// Reliable frame flag: the fragment starts a message.
pub const FLAG_FIRST: u8 = 0x20;
/// Reliable frame flag: the fragment ends a message.
pub const FLAG_LAST: u8 = 0x10;

/// Send-side state of one fragment.
#[derive(Debug, Clone, Copy, Default)]
struct FragState {
    delivered: bool,
    last_sent_ms: Option<u64>,
}

/// One message waiting for acknowledgements. Its bytes are stored once; fragments are cut from them on each send.
#[derive(Debug)]
struct Pending {
    first_id: u16,
    count: usize,
    data: Vec<u8>,
    frags: [FragState; MAX_FRAGMENTS],
}

impl Pending {
    /// Whether message id `id` is one of this message's fragments.
    fn holds(&self, id: u16) -> bool {
        usize::from(id.wrapping_sub(self.first_id)) < self.count
    }

    /// Whether every fragment is acknowledged.
    fn is_complete(&self) -> bool {
        self.frags.iter().take(self.count).all(|frag| frag.delivered)
    }

    /// The bytes of fragment `index`, ready to be written into a reliable frame.
    fn part(&self, index: usize) -> FragPart<'_> {
        let (start, end) = fragment_span(self.data.len(), index);
        let data = self.data.get(start..end).unwrap_or(&[]);
        let prefix = (index == 0 && self.count > 1).then(|| (self.data.len() as u16).to_le_bytes());
        let flags = (FLAG_FIRST * u8::from(index == 0)) | (FLAG_LAST * u8::from(index + 1 == self.count));
        FragPart { id: self.first_id.wrapping_add(index as u16), flags, prefix, data }
    }
}

/// The bytes of one fragment as they go into a reliable frame: the header bytes come from `id` and `flags`, then the
/// optional prefix, then `data`.
#[derive(Debug, Clone, Copy)]
pub struct FragPart<'a> {
    /// The fragment's message id.
    pub id: u16,
    /// [`FLAG_FIRST`] and [`FLAG_LAST`].
    pub flags: u8,
    /// The total message length; present only on the first fragment of a multi-fragment message.
    pub prefix: Option<[u8; 2]>,
    /// Message bytes.
    pub data: &'a [u8],
}

/// The send side of one channel: a queue of messages in id order.
#[derive(Debug, Default)]
pub struct SendChannel {
    next_id: u16,
    queue: VecDeque<Pending>,
}

impl SendChannel {
    /// Queues one message and gives it the next fragment ids. Refuses a message that cannot be fragmented
    /// ([`ConnError::MessageTooLarge`]) and a channel that already holds [`MAX_IN_FLIGHT`] messages
    /// ([`ConnError::QueueFull`]).
    pub fn push(&mut self, msg: &[u8]) -> Result<(), ConnError> {
        let count = fragment_count(msg.len()).ok_or(ConnError::MessageTooLarge)?;
        if self.queue.len() >= MAX_IN_FLIGHT {
            return Err(ConnError::QueueFull);
        }
        self.queue.push_back(Pending {
            first_id: self.next_id,
            count,
            data: msg.to_vec(),
            frags: [FragState::default(); MAX_FRAGMENTS],
        });
        self.next_id = self.next_id.wrapping_add(count as u16);
        Ok(())
    }

    /// Messages not yet fully acknowledged.
    pub fn pending(&self) -> usize {
        self.queue.len()
    }

    /// Queue position and fragment index of the oldest fragment that is due: never sent, or last sent at least
    /// `rto_ms` ago and not yet acknowledged.
    pub fn next_due(&self, now_ms: u64, rto_ms: f64) -> Option<(usize, usize)> {
        self.queue.iter().enumerate().find_map(|(pos, msg)| {
            let index = msg.frags.iter().take(msg.count).position(|frag| is_due(frag, now_ms, rto_ms))?;
            Some((pos, index))
        })
    }

    /// Whether any fragment is due.
    pub fn has_due(&self, now_ms: u64, rto_ms: f64) -> bool {
        self.next_due(now_ms, rto_ms).is_some()
    }

    /// The fragment at queue position `pos`, index `index`, ready to be framed.
    pub fn fragment(&self, pos: usize, index: usize) -> Option<FragPart<'_>> {
        let msg = self.queue.get(pos)?;
        if index >= msg.count {
            return None;
        }
        Some(msg.part(index))
    }

    /// Records that fragment `index` of the message at `pos` was sent at `now_ms`.
    pub fn mark_sent(&mut self, pos: usize, index: usize, now_ms: u64) {
        if let Some(frag) = self.queue.get_mut(pos).and_then(|msg| msg.frags.get_mut(index)) {
            frag.last_sent_ms = Some(now_ms);
        }
    }

    /// Marks the fragment with message id `id` as acknowledged, then drops the complete messages at the front of the
    /// queue (ids stay consecutive because only the front goes).
    pub fn mark_delivered(&mut self, id: u16) {
        let Some(msg) = self.queue.iter_mut().find(|msg| msg.holds(id)) else {
            return;
        };
        let index = usize::from(id.wrapping_sub(msg.first_id));
        if let Some(frag) = msg.frags.get_mut(index) {
            frag.delivered = true;
        }
        while self.queue.front().is_some_and(Pending::is_complete) {
            self.queue.pop_front();
        }
    }
}

/// Whether a fragment should be sent now.
fn is_due(frag: &FragState, now_ms: u64, rto_ms: f64) -> bool {
    if frag.delivered {
        return false;
    }
    frag.last_sent_ms.is_none_or(|sent| now_ms.saturating_sub(sent) as f64 >= rto_ms)
}

/// One received fragment waiting for its message.
#[derive(Debug, Default)]
struct Slot {
    id: u16,
    filled: bool,
    first: bool,
    last: bool,
    data: Vec<u8>,
}

/// The receive side of one channel: fragments inside the window, and the messages assembled from them.
#[derive(Debug, Default)]
pub struct RecvChannel {
    /// Id of the next fragment to deliver; the start of the next message.
    expected: u16,
    /// Ring of [`WINDOW`] slots indexed by `id % WINDOW`; allocated on first use.
    slots: Vec<Slot>,
    ready: VecDeque<Vec<u8>>,
}

impl RecvChannel {
    /// Takes one fragment. Fragments below the window (already delivered) and above it are ignored, and a fragment seen
    /// before is ignored. A malformed fragment returns [`ConnError::BadFrame`] before any buffer is touched.
    pub fn receive(&mut self, id: u16, first: bool, last: bool, data: &[u8]) -> Result<(), ConnError> {
        if data.len() > MAX_FRAGMENT {
            return Err(ConnError::BadFrame);
        }
        if first && !last {
            declared_total(data)?;
        }
        let offset = usize::from(id.wrapping_sub(self.expected));
        if offset >= WINDOW {
            return Ok(());
        }
        if self.slots.is_empty() {
            self.slots.resize_with(WINDOW, Slot::default);
        }
        let Some(slot) = self.slots.get_mut(usize::from(id) % WINDOW) else {
            return Ok(());
        };
        if slot.filled && slot.id == id {
            return Ok(());
        }
        slot.id = id;
        slot.filled = true;
        slot.first = first;
        slot.last = last;
        slot.data.clear();
        slot.data.extend_from_slice(data);
        self.assemble_ready()
    }

    /// The next complete message, in delivery order.
    pub fn pop_ready(&mut self) -> Option<Vec<u8>> {
        self.ready.pop_front()
    }

    /// Moves every message that can now be assembled from the front of the window to the ready queue.
    fn assemble_ready(&mut self) -> Result<(), ConnError> {
        while let Some((message, count)) = assemble(&self.slots, self.expected)? {
            self.ready.push_back(message);
            for k in 0..count {
                let id = self.expected.wrapping_add(k as u16);
                if let Some(slot) = self.slots.get_mut(usize::from(id) % WINDOW) {
                    slot.filled = false;
                    slot.data.clear();
                }
            }
            self.expected = self.expected.wrapping_add(count as u16);
        }
        Ok(())
    }
}

/// The slot holding fragment `id`, if it is filled with that id.
fn slot_at(slots: &[Slot], id: u16) -> Option<&Slot> {
    let slot = slots.get(usize::from(id) % WINDOW)?;
    (slot.filled && slot.id == id).then_some(slot)
}

/// Assembles the message that starts at fragment `expected`, if all its fragments are present. Returns the bytes and the
/// number of fragments. The flags and the total length must agree with the fragments, or the frame is refused.
fn assemble(slots: &[Slot], expected: u16) -> Result<Option<(Vec<u8>, usize)>, ConnError> {
    let Some(head) = slot_at(slots, expected) else {
        return Ok(None);
    };
    if !head.first {
        return Err(ConnError::BadFrame);
    }
    let (count, total) = match head.last {
        true => (1, head.data.len()),
        false => {
            let total = declared_total(&head.data)?;
            let count = fragment_count(total).ok_or(ConnError::BadFrame)?;
            (count, total)
        }
    };
    let mut message = Vec::with_capacity(total);
    for k in 0..count {
        let id = expected.wrapping_add(k as u16);
        let Some(part) = slot_at(slots, id) else {
            return Ok(None);
        };
        let is_last = k + 1 == count;
        if part.last != is_last || (k > 0 && part.first) {
            return Err(ConnError::BadFrame);
        }
        let skip = match k {
            0 if count > 1 => 2,
            _ => 0,
        };
        message.extend_from_slice(part.data.get(skip..).unwrap_or(&[]));
    }
    if message.len() != total {
        return Err(ConnError::BadFrame);
    }
    Ok(Some((message, count)))
}

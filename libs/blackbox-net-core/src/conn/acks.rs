//! Packet bookkeeping: a fixed ring keyed by packet number, and the receive window that builds the acknowledgement block.

use super::ConnError;

/// Packet numbers the receive window remembers (and the most behind the newest it accepts).
pub const WINDOW_BITS: u32 = 1024;

/// Wrap-aware "`a` is newer than `b`": true when `a` is ahead of `b` by less than 2^31.
pub fn seq_newer(a: u32, b: u32) -> bool {
    a != b && a.wrapping_sub(b) < 0x8000_0000
}

/// A ring of `Option<T>` keyed by `u32`, with a fixed capacity. Each slot remembers its key, so a lookup never returns
/// a value stored under another number that maps to the same slot. Keys may wrap at 2^32 when the capacity divides it.
#[derive(Debug)]
pub struct SequenceBuffer<T> {
    slots: Vec<Option<(u32, T)>>,
}

impl<T> SequenceBuffer<T> {
    /// A buffer of `capacity` slots (at least one).
    pub fn with_capacity(capacity: usize) -> Self {
        let mut slots = Vec::with_capacity(capacity.max(1));
        slots.resize_with(capacity.max(1), || None);
        Self { slots }
    }

    /// Stores `value` under `seq`, replacing whatever the slot held. Returns the replaced entry, whatever its key.
    pub fn insert(&mut self, seq: u32, value: T) -> Option<(u32, T)> {
        let index = seq as usize % self.slots.len();
        let slot = self.slots.get_mut(index)?;
        slot.replace((seq, value))
    }

    /// Removes and returns the value stored under `seq`, if the slot still holds that key.
    pub fn remove(&mut self, seq: u32) -> Option<T> {
        let index = seq as usize % self.slots.len();
        let slot = self.slots.get_mut(index)?;
        let stored = matches!(slot, Some((key, _)) if *key == seq);
        if !stored {
            return None;
        }
        slot.take().map(|(_, value)| value)
    }

    /// Visits every stored entry, removing and passing on those for which `pred` is true. Allocates nothing.
    pub fn drain_matching(&mut self, mut pred: impl FnMut(u32, &T) -> bool, mut sink: impl FnMut(u32, T)) {
        for slot in &mut self.slots {
            let take = match slot.as_ref() {
                Some((seq, value)) => pred(*seq, value),
                None => false,
            };
            let taken = take.then(|| slot.take()).flatten();
            if let Some((seq, value)) = taken {
                sink(seq, value);
            }
        }
    }
}

/// The newest packet numbers received, as a bitset: bit `i` of the window is "packet `newest - i` was received".
#[derive(Debug, Default, Clone, Copy)]
pub struct RecvWindow {
    newest: u32,
    any: bool,
    bits: [u64; 16],
}

impl RecvWindow {
    /// Whether any packet has been received yet.
    pub fn any(&self) -> bool {
        self.any
    }

    /// The newest packet number received (0 before any).
    pub fn ack(&self) -> u32 {
        self.newest
    }

    /// The acknowledgement bits: bit `i` set means packet `ack - 1 - i` was received.
    pub fn ack_bits(&self) -> u32 {
        let low = self.bits.first().copied().unwrap_or(0);
        let high = self.bits.get(1).copied().unwrap_or(0);
        let shifted = (low >> 1) | (high << 63);
        (shifted & 0xFFFF_FFFF) as u32
    }

    /// Marks `seq` as received. Refuses a number already marked ([`ConnError::Duplicate`]) and one more than 1024 behind
    /// the newest ([`ConnError::TooOld`]); a newer number moves the window forward.
    pub fn record(&mut self, seq: u32) -> Result<(), ConnError> {
        if !self.any {
            self.any = true;
            self.newest = seq;
            self.bits = [0; 16];
            self.set_newest_bit();
            return Ok(());
        }
        if seq_newer(seq, self.newest) {
            self.shift(seq.wrapping_sub(self.newest));
            self.newest = seq;
            self.set_newest_bit();
            return Ok(());
        }
        let age = self.newest.wrapping_sub(seq);
        if age >= WINDOW_BITS {
            return Err(ConnError::TooOld);
        }
        let word = (age / 64) as usize;
        let bit = 1u64 << (age % 64);
        let Some(slot) = self.bits.get_mut(word) else {
            return Err(ConnError::TooOld);
        };
        if *slot & bit != 0 {
            return Err(ConnError::Duplicate);
        }
        *slot |= bit;
        Ok(())
    }

    fn set_newest_bit(&mut self) {
        if let Some(word) = self.bits.first_mut() {
            *word |= 1;
        }
    }

    /// Moves every mark `by` places towards older-to-newer order (bit `i` becomes bit `i + by`); marks that fall off
    /// the top are dropped.
    fn shift(&mut self, by: u32) {
        if by >= WINDOW_BITS {
            self.bits = [0; 16];
            return;
        }
        let words = (by / 64) as usize;
        let bits = by % 64;
        let old = self.bits;
        let mut next = [0u64; 16];
        for (i, out) in next.iter_mut().enumerate().skip(words) {
            let src = i - words;
            let direct = old.get(src).copied().unwrap_or(0) << bits;
            let carry = match src.checked_sub(1) {
                Some(prev) if bits > 0 => old.get(prev).copied().unwrap_or(0) >> (64 - bits),
                _ => 0,
            };
            *out = direct | carry;
        }
        self.bits = next;
    }
}

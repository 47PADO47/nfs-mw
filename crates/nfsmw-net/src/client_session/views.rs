//! The client's replicated views: one [`Frame`] per decoded snapshot tick for the last 64 ticks, and the decoder that
//! builds a snapshot's view from its baseline and every part's entries.
//!
//! `blackbox_net_core::replicate::ClientViews` decodes a single body; a snapshot in parts needs the parts merged, so
//! this ring applies part 0 to the baseline, part 1 to that result, and so on. Parts hold disjoint entities, so an
//! entity a later part does not name keeps the value an earlier part or the baseline gave it.

use std::mem;

use blackbox_net_core::bits::{BitError, BitReader, IdGapDecoder};
use blackbox_net_core::replicate::{
    Frame, MAX_FIELDS, OP_DELTA, OP_FULL, OP_REMOVE, ReplicateError, Schema, decode_delta, decode_full,
};
use nfsmw_sim::Tick;

/// Views kept: a snapshot may name any of the last this many ticks as its baseline.
pub const VIEW_SLOTS: usize = 64;

#[derive(Debug, Default)]
struct Slot {
    valid: bool,
    frame: Frame,
}

/// The decoded views, by tick.
#[derive(Debug)]
pub struct Views {
    slots: Vec<Slot>,
    scratch: [Frame; 2],
    /// The baseline of a full snapshot.
    empty: Frame,
}

impl Default for Views {
    fn default() -> Self {
        Self::new()
    }
}

impl Views {
    pub fn new() -> Self {
        Self {
            slots: (0..VIEW_SLOTS).map(|_| Slot::default()).collect(),
            scratch: Default::default(),
            empty: Frame::new(),
        }
    }

    /// The view of `tick`, if kept.
    pub fn view(&self, tick: Tick) -> Option<&Frame> {
        let slot = self.slots.get(tick as usize % VIEW_SLOTS)?;
        match slot.valid && slot.frame.tick() == tick {
            true => Some(&slot.frame),
            false => None,
        }
    }

    /// Forgets every view (a reconnect): the next snapshot must be full.
    pub fn reset(&mut self) {
        for slot in &mut self.slots {
            slot.valid = false;
        }
    }

    /// Decodes the snapshot of `tick` from its part bodies (in part order) and keeps its view. Every body must name
    /// `tick` and the same baseline distance. A missing baseline is [`ReplicateError::MissingBaseline`]; any error
    /// leaves the kept views as they were.
    pub fn decode(&mut self, schema: &Schema, tick: Tick, bodies: &[&[u8]]) -> Result<&Frame, ReplicateError> {
        let [current, spare] = &mut self.scratch;
        let mut distance = None;
        for body in bodies {
            let mut r = BitReader::new(body);
            let body_tick = u32::try_from(r.read_bits(32)?).map_err(|_| ReplicateError::BadValue)?;
            let body_distance = r.read_varint()?;
            if body_tick != tick || distance.is_some_and(|known| known != body_distance) {
                return Err(ReplicateError::BadValue);
            }
            let count = usize::try_from(r.read_varint()?).map_err(|_| ReplicateError::BadValue)?;
            if count > r.remaining_bits() {
                return Err(ReplicateError::Bits(BitError::Underflow));
            }
            spare.clear(tick);
            match distance {
                // The first part applies to the baseline; every later one to the parts before it.
                None => merge(schema, &mut r, count, baseline(&self.slots, &self.empty, tick, body_distance)?, spare)?,
                Some(_) => merge(schema, &mut r, count, current, spare)?,
            }
            check_padding(&mut r)?;
            mem::swap(current, spare);
            distance = Some(body_distance);
        }
        if distance.is_none() {
            return Err(ReplicateError::BadValue);
        }
        let slot = &mut self.slots[tick as usize % VIEW_SLOTS];
        mem::swap(&mut slot.frame, current);
        slot.valid = true;
        Ok(&slot.frame)
    }
}

/// The baseline view a snapshot of `tick` names with `distance` (0 = an empty view).
fn baseline<'a>(slots: &'a [Slot], empty: &'a Frame, tick: Tick, distance: u64) -> Result<&'a Frame, ReplicateError> {
    if distance == 0 {
        return Ok(empty);
    }
    let back = u32::try_from(distance).map_err(|_| ReplicateError::MissingBaseline)?;
    let base_tick = tick.checked_sub(back).ok_or(ReplicateError::MissingBaseline)?;
    let slot = slots.get(base_tick as usize % VIEW_SLOTS).ok_or(ReplicateError::MissingBaseline)?;
    if !slot.valid || slot.frame.tick() != base_tick {
        return Err(ReplicateError::MissingBaseline);
    }
    Ok(&slot.frame)
}

/// Reads `count` entries into `out`, merged with `base`: entities the entries do not name are copied.
fn merge(
    schema: &Schema,
    r: &mut BitReader<'_>,
    count: usize,
    base: &Frame,
    out: &mut Frame,
) -> Result<(), ReplicateError> {
    let mut ids = IdGapDecoder::new();
    let mut base_iter = base.iter().peekable();
    let mut values = [0u32; MAX_FIELDS];
    for _ in 0..count {
        let id = ids.read(r)?;
        while let Some(&(base_id, kind, old)) = base_iter.peek() {
            if base_id >= id {
                break;
            }
            out.push(schema, base_id, kind, old)?;
            base_iter.next();
        }
        let old = match base_iter.peek() {
            Some(&(base_id, kind, old)) if base_id == id => {
                base_iter.next();
                Some((kind, old))
            }
            _ => None,
        };
        match r.read_bits(2)? {
            OP_DELTA => {
                let (kind, old) = old.ok_or(ReplicateError::UnknownEntity)?;
                let len = decode_delta(schema, kind, old, r, &mut values)?;
                out.push(schema, id, kind, &values[..len])?;
            }
            OP_FULL => {
                let (kind, len) = decode_full(schema, r, &mut values)?;
                out.push(schema, id, kind, &values[..len])?;
            }
            OP_REMOVE => {
                old.ok_or(ReplicateError::UnknownEntity)?;
            }
            _ => return Err(ReplicateError::BadOp),
        }
    }
    for (base_id, kind, old) in base_iter {
        out.push(schema, base_id, kind, old)?;
    }
    Ok(())
}

/// The bits after the last entry must be the zero padding of the last byte.
fn check_padding(r: &mut BitReader<'_>) -> Result<(), ReplicateError> {
    let left = r.remaining_bits();
    if left >= 8 {
        return Err(ReplicateError::BadValue);
    }
    if left > 0 && r.read_bits(left as u32)? != 0 {
        return Err(ReplicateError::BadValue);
    }
    Ok(())
}

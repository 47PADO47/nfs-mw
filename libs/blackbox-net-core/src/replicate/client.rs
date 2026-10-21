//! The client's half of replication: the views the server has sent, one per tick, and the decoder that builds each new
//! view from the baseline view named in the snapshot.

use std::iter::Peekable;
use std::mem;

use super::{
    Frame, KindId, MAX_FIELDS, OP_DELTA, OP_FULL, OP_REMOVE, ReplicateError, Schema, decode_delta, decode_full,
};
use crate::bits::{BitError, BitReader, IdGapDecoder};

/// Views kept: a snapshot may name any of the last this many ticks as its baseline.
const VIEW_SLOTS: usize = 64;

/// One view slot: a frame and whether it holds a decoded view.
#[derive(Debug, Default)]
struct ViewSlot {
    valid: bool,
    frame: Frame,
}

/// The views of the last 64 ticks this client decoded. Each view is the set of entities the client holds at that tick.
#[derive(Debug)]
pub struct ClientViews {
    slots: Vec<ViewSlot>,
    scratch: Frame,
    latest: Option<u32>,
}

impl Default for ClientViews {
    fn default() -> Self {
        Self::new()
    }
}

impl ClientViews {
    /// No views yet.
    pub fn new() -> Self {
        Self { slots: (0..VIEW_SLOTS).map(|_| ViewSlot::default()).collect(), scratch: Frame::new(), latest: None }
    }

    /// Decodes one snapshot from `r` and keeps its view. Returns the snapshot's tick.
    ///
    /// The baseline is the view named by the distance field (0 = an empty view). When it is not kept the result is
    /// [`ReplicateError::MissingBaseline`] and nothing changes. A malformed snapshot is an error too, and leaves the
    /// stored views as they were. This never panics on any input.
    pub fn read_snapshot(&mut self, schema: &Schema, r: &mut BitReader<'_>) -> Result<u32, ReplicateError> {
        let tick = u32::try_from(r.read_bits(32)?).map_err(|_| ReplicateError::BadValue)?;
        let distance = r.read_varint()?;
        let count = usize::try_from(r.read_varint()?).map_err(|_| ReplicateError::BadValue)?;
        let baseline = baseline_tick(tick, distance)?;
        let empty = Frame::new();
        let base: &Frame = match baseline {
            Some(base_tick) => view_in(&self.slots, base_tick).ok_or(ReplicateError::MissingBaseline)?,
            None => &empty,
        };
        // Each entry is at least an id gap (4 bits) and an operation (2 bits).
        if count > r.remaining_bits() {
            return Err(ReplicateError::Bits(BitError::Underflow));
        }
        let out = &mut self.scratch;
        out.clear(tick);
        decode_entries(schema, r, base, out, count)?;
        // The new view goes into a scratch frame and is swapped in, so a baseline sharing the slot stays readable until
        // the end.
        let slot = &mut self.slots[slot_of(tick)];
        mem::swap(&mut slot.frame, &mut self.scratch);
        slot.valid = true;
        self.latest = Some(match self.latest {
            Some(newest) if newest >= tick => newest,
            _ => tick,
        });
        Ok(tick)
    }

    /// The view of `tick`, if it is still kept.
    pub fn view(&self, tick: u32) -> Option<&Frame> {
        view_in(&self.slots, tick)
    }

    /// The newest view decoded, if it is still kept.
    pub fn latest(&self) -> Option<&Frame> {
        self.view(self.latest?)
    }

    /// Forgets every view. The next snapshot must then have no baseline.
    pub fn reset(&mut self) {
        for slot in &mut self.slots {
            slot.valid = false;
        }
        self.latest = None;
    }
}

/// The view slot of `tick`.
fn slot_of(tick: u32) -> usize {
    tick as usize % VIEW_SLOTS
}

/// The view of `tick` in `slots`, if it is the one kept there.
fn view_in(slots: &[ViewSlot], tick: u32) -> Option<&Frame> {
    let slot = slots.get(slot_of(tick))?;
    if !slot.valid || slot.frame.tick() != tick {
        return None;
    }
    Some(&slot.frame)
}

/// The baseline tick for a snapshot of `tick` whose distance field is `distance`. Distance 0 means no baseline.
fn baseline_tick(tick: u32, distance: u64) -> Result<Option<u32>, ReplicateError> {
    if distance == 0 {
        return Ok(None);
    }
    let back = u32::try_from(distance).map_err(|_| ReplicateError::MissingBaseline)?;
    tick.checked_sub(back).map(Some).ok_or(ReplicateError::MissingBaseline)
}

/// Reads `count` entries into `out`, merging them with the baseline `base`: baseline entities the snapshot does not
/// name are copied, named ones are replaced or removed.
fn decode_entries(
    schema: &Schema,
    r: &mut BitReader<'_>,
    base: &Frame,
    out: &mut Frame,
    count: usize,
) -> Result<(), ReplicateError> {
    let mut ids = IdGapDecoder::new();
    let mut base_iter = base.iter().peekable();
    for _ in 0..count {
        let id = ids.read(r)?;
        copy_base(&mut base_iter, out, schema, Some(id))?;
        let op = r.read_bits(2)?;
        apply_entry(schema, r, &mut base_iter, out, id, op)?;
    }
    copy_base(&mut base_iter, out, schema, None)
}

/// Applies one entry for entity `id` with operation `op`. The baseline entry for `id` (if any) is consumed.
fn apply_entry<'a, I>(
    schema: &Schema,
    r: &mut BitReader<'_>,
    base: &mut Peekable<I>,
    out: &mut Frame,
    id: u16,
    op: u64,
) -> Result<(), ReplicateError>
where
    I: Iterator<Item = (u16, KindId, &'a [u32])>,
{
    let mut values = [0u32; MAX_FIELDS];
    match op {
        OP_DELTA => {
            let (kind, old) = take_base(base, id).ok_or(ReplicateError::UnknownEntity)?;
            let count = decode_delta(schema, kind, old, r, &mut values)?;
            out.push(schema, id, kind, &values[..count])
        }
        OP_FULL => {
            let (kind, count) = decode_full(schema, r, &mut values)?;
            skip_base(base, id);
            out.push(schema, id, kind, &values[..count])
        }
        OP_REMOVE => {
            take_base(base, id).ok_or(ReplicateError::UnknownEntity)?;
            Ok(())
        }
        _ => Err(ReplicateError::BadOp),
    }
}

/// Copies every baseline entry with an id below `before` (every entry when `before` is `None`) into `out`.
fn copy_base<'a, I>(
    base: &mut Peekable<I>,
    out: &mut Frame,
    schema: &Schema,
    before: Option<u16>,
) -> Result<(), ReplicateError>
where
    I: Iterator<Item = (u16, KindId, &'a [u32])>,
{
    while let Some(&(id, kind, values)) = base.peek() {
        if before.is_some_and(|limit| id >= limit) {
            break;
        }
        out.push(schema, id, kind, values)?;
        base.next();
    }
    Ok(())
}

/// Takes the baseline entry for `id`: its kind and values. Fails (`None`) when the next baseline entry is another one.
fn take_base<'a, I>(base: &mut Peekable<I>, id: u16) -> Option<(KindId, &'a [u32])>
where
    I: Iterator<Item = (u16, KindId, &'a [u32])>,
{
    match base.peek() {
        Some(&(base_id, kind, values)) if base_id == id => {
            base.next();
            Some((kind, values))
        }
        _ => None,
    }
}

/// Drops the baseline entry for `id`, if the next one is it.
fn skip_base<'a, I>(base: &mut Peekable<I>, id: u16)
where
    I: Iterator<Item = (u16, KindId, &'a [u32])>,
{
    if base.peek().is_some_and(|&(base_id, _, _)| base_id == id) {
        base.next();
    }
}

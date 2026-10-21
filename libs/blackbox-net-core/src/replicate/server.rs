//! The server's half of replication: for each client, the views it has been sent and the snapshot written for each tick
//! (ADR 0009 section 2). The view of a tick is `(id, source tick)` per entity the client holds, sorted by id. A snapshot
//! is written against the newest delivered view that is still kept.
//!
//! Snapshot layout: tick (32 bits), baseline distance (varint, 0 = none), entry count (varint), then the entries in id
//! order: id gap ([`IdGapEncoder`]), operation (2 bits), body (a cached span; nothing for a removal).

use std::mem;

use super::{EncodeCache, History, OP_DELTA, OP_FULL, OP_REMOVE, ReplicateError, Schema, Span};
use crate::bits::{BitWriter, IdGapEncoder};

/// Sent views kept: a snapshot may be based on any of the last this many ticks.
const VIEW_SLOTS: usize = 64;
/// Bits the header takes before the entries, counted conservatively: tick 32, distance 16, count 16.
const HEADER_BITS: usize = 64;
/// Bits an entry takes beyond its body, counted conservatively: the id gap 16 and the operation 2.
const ENTRY_BITS: usize = 18;

/// What one [`ClientState::write_snapshot`] wrote.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct SnapshotStats {
    /// The tick of the view the snapshot was written against, or `None` when it has no baseline.
    pub baseline: Option<u32>,
    /// Full records written, forced ones included.
    pub full: u32,
    /// Deltas written.
    pub delta: u32,
    /// Removals written.
    pub removed: u32,
    /// Changed candidates left out because they did not fit the budget.
    pub skipped: u32,
    /// Bits written, header included.
    pub bits: usize,
}

/// One sent view slot: the tick it records and the `(id, source tick)` pairs it holds, sorted by id.
#[derive(Debug, Default)]
struct SentView {
    tick: u32,
    valid: bool,
    held: Vec<(u16, u32)>,
}

/// A changed candidate that waits for budget.
#[derive(Debug, Clone, Copy)]
struct Pending {
    id: u16,
    span: Span,
    priority: f32,
    /// The source tick the client keeps when this entity is not sent; `None` when the client does not hold it yet.
    kept_source: Option<u32>,
}

/// The scratch of one snapshot. Its lists keep their capacity from call to call, so the steady state allocates nothing.
#[derive(Debug, Default)]
struct Plan {
    priority: Vec<f32>,
    pending: Vec<Pending>,
    chosen: Vec<(u16, Span)>,
    held: Vec<(u16, u32)>,
    used: usize,
    skipped: u32,
}

impl Plan {
    /// Empties the lists and reserves room for the most entries this snapshot can hold.
    fn reset(&mut self, candidates: usize, base: usize) {
        self.pending.clear();
        self.chosen.clear();
        self.held.clear();
        self.used = HEADER_BITS;
        self.skipped = 0;
        self.pending.reserve(candidates);
        self.chosen.reserve(candidates + base);
        self.held.reserve(candidates);
    }

    /// Adds each candidate's weight to its priority accumulator. Negative and NaN weights count as 0.
    fn accrue(&mut self, candidates: &[(u16, f32)]) {
        for &(id, weight) in candidates {
            let index = usize::from(id);
            if index >= self.priority.len() {
                self.priority.resize(index + 1, 0.0);
            }
            self.priority[index] += weight.max(0.0);
        }
    }

    /// Clears the priority of `id`: it was just sent or is up to date.
    fn clear_priority(&mut self, id: u16) {
        if let Some(priority) = self.priority.get_mut(usize::from(id)) {
            *priority = 0.0;
        }
    }

    /// Sends `id` regardless of budget with `span`: a forced full record.
    fn force(&mut self, id: u16, span: Span, tick: u32) {
        self.used += span.bits + ENTRY_BITS;
        self.chosen.push((id, span));
        self.held.push((id, tick));
        self.clear_priority(id);
    }

    /// Removes `id` from the client's view. Always sent.
    fn remove(&mut self, id: u16) {
        self.used += ENTRY_BITS;
        self.chosen.push((id, Span { op: OP_REMOVE, start: 0, bits: 0 }));
    }

    /// `id` is unchanged since its source: the client holds it up to date, so the view moves to `tick` for free.
    fn unchanged(&mut self, id: u16, tick: u32) {
        self.held.push((id, tick));
        self.clear_priority(id);
    }

    /// A changed candidate that competes for budget. `kept_source` is the tick the client keeps if it is not sent.
    fn offer(&mut self, id: u16, span: Span, kept_source: Option<u32>) {
        let priority = self.priority.get(usize::from(id)).copied().unwrap_or(0.0);
        self.pending.push(Pending { id, span, priority, kept_source });
    }

    /// Takes the pending entries in priority order (highest first, ties by id) while they fit `budget`. Each entry that
    /// does not fit is skipped and the scan goes on, so a smaller one can still be taken.
    fn select(&mut self, budget: usize, tick: u32) {
        self.pending.sort_unstable_by(|a, b| b.priority.total_cmp(&a.priority).then(a.id.cmp(&b.id)));
        for entry in &self.pending {
            let cost = entry.span.bits + ENTRY_BITS;
            match self.used + cost <= budget {
                true => {
                    self.used += cost;
                    self.chosen.push((entry.id, entry.span));
                    self.held.push((entry.id, tick));
                    if let Some(priority) = self.priority.get_mut(usize::from(entry.id)) {
                        *priority = 0.0;
                    }
                }
                false => {
                    self.skipped += 1;
                    if let Some(source) = entry.kept_source {
                        self.held.push((entry.id, source));
                    }
                }
            }
        }
    }
}

/// The replication state of one client, kept by the server: the views it has been sent and which of them it acknowledged.
#[derive(Debug)]
pub struct ClientState {
    delivered: Option<u32>,
    views: Vec<SentView>,
    plan: Plan,
}

impl Default for ClientState {
    fn default() -> Self {
        Self::new()
    }
}

impl ClientState {
    /// A client with no view and no delivered snapshot, so its first snapshot has no baseline.
    pub fn new() -> Self {
        Self { delivered: None, views: (0..VIEW_SLOTS).map(|_| SentView::default()).collect(), plan: Plan::default() }
    }

    /// Forgets every sent view and the delivered tick, so the next snapshot has no baseline and is written in full. The
    /// game calls this when the client reports that it lost its baseline.
    pub fn reset(&mut self) {
        self.delivered = None;
        for view in &mut self.views {
            view.valid = false;
        }
    }

    /// Records that the snapshot of `tick` was delivered. The newest delivered tick is the baseline.
    pub fn on_delivered(&mut self, tick: u32) {
        self.delivered = Some(match self.delivered {
            Some(newest) if newest >= tick => newest,
            _ => tick,
        });
    }

    /// The view recorded for `tick`: `(id, source tick)` per entity the client holds, sorted by id. `None` when the view
    /// has been overwritten or never existed.
    pub fn held(&self, tick: u32) -> Option<&[(u16, u32)]> {
        let view = self.views.get(slot_of(tick))?;
        if !view.valid || view.tick != tick {
            return None;
        }
        Some(view.held.as_slice())
    }

    /// Writes the snapshot of `tick` into `w` and records the view it gives the client.
    ///
    /// `candidates` are the entities relevant to this client, sorted by id, each with a priority weight (finite, and
    /// usually 1 or less). Each one must be in the frame of `tick`. `budget_bits` caps the size of the optional entries;
    /// the header and the mandatory entries (removals and forced full records) are written whatever their size. The
    /// cache must hold `tick`, and `history` must hold its frame.
    ///
    /// Errors: [`ReplicateError::BadOrder`] when the cache holds another tick or the candidates are not sorted and
    /// unique; [`ReplicateError::MissingBaseline`] when `history` does not hold `tick`; the writer's own errors when `w`
    /// is too small for the mandatory entries. On an error the recorded view does not change.
    #[allow(clippy::too_many_arguments)]
    pub fn write_snapshot(
        &mut self,
        schema: &Schema,
        history: &History,
        cache: &mut EncodeCache,
        tick: u32,
        candidates: &[(u16, f32)],
        budget_bits: usize,
        w: &mut BitWriter<'_>,
    ) -> Result<SnapshotStats, ReplicateError> {
        if cache.tick() != tick {
            return Err(ReplicateError::BadOrder);
        }
        if candidates.windows(2).any(|pair| pair[0].0 >= pair[1].0) {
            return Err(ReplicateError::BadOrder);
        }
        let current = history.frame(tick).ok_or(ReplicateError::MissingBaseline)?;
        let baseline = self.baseline_for(tick);
        let base: &[(u16, u32)] = match baseline.and_then(|base_tick| self.views.get(slot_of(base_tick))) {
            Some(view) => view.held.as_slice(),
            None => &[],
        };
        let start = w.bits_written();
        let budget = budget_bits.min(w.remaining_bits());
        let plan = &mut self.plan;
        plan.reset(candidates.len(), base.len());
        plan.accrue(candidates);
        let (mut i, mut j) = (0usize, 0usize);
        while i < base.len() || j < candidates.len() {
            match (base.get(i), candidates.get(j)) {
                (Some(&(id, source)), Some(&(candidate, _))) if id == candidate => {
                    match history.frame(source) {
                        None => {
                            let span = cache.full(schema, current, id)?;
                            plan.force(id, span, tick);
                        }
                        Some(base_frame) => {
                            let span = cache.delta(schema, base_frame, current, id)?;
                            match span.op == OP_DELTA && span.bits == 0 {
                                true => plan.unchanged(id, tick),
                                false => plan.offer(id, span, Some(source)),
                            }
                        }
                    }
                    i += 1;
                    j += 1;
                }
                (Some(&(id, _)), next) if next.is_none_or(|&(candidate, _)| id < candidate) => {
                    plan.remove(id);
                    i += 1;
                }
                (_, Some(&(candidate, _))) => {
                    let span = cache.full(schema, current, candidate)?;
                    plan.offer(candidate, span, None);
                    j += 1;
                }
                _ => break,
            }
        }
        plan.select(budget, tick);
        plan.chosen.sort_unstable_by_key(|entry| entry.0);
        plan.held.sort_unstable_by_key(|entry| entry.0);
        let mut stats = SnapshotStats { baseline, skipped: plan.skipped, ..SnapshotStats::default() };
        write_entries(&plan.chosen, cache, tick, baseline, w, &mut stats)?;
        stats.bits = w.bits_written() - start;
        let view = &mut self.views[slot_of(tick)];
        mem::swap(&mut view.held, &mut plan.held);
        view.tick = tick;
        view.valid = true;
        Ok(stats)
    }

    /// The baseline for a snapshot of `tick`: the newest delivered tick, if its view is still kept and is older than
    /// `tick`.
    fn baseline_for(&self, tick: u32) -> Option<u32> {
        let delivered = self.delivered?;
        if delivered >= tick {
            return None;
        }
        let view = self.views.get(slot_of(delivered))?;
        if !view.valid || view.tick != delivered {
            return None;
        }
        Some(delivered)
    }
}

/// The view slot of `tick`.
fn slot_of(tick: u32) -> usize {
    tick as usize % VIEW_SLOTS
}

/// Writes the header and the chosen entries, in the order they are given (ascending ids).
fn write_entries(
    chosen: &[(u16, Span)],
    cache: &EncodeCache,
    tick: u32,
    baseline: Option<u32>,
    w: &mut BitWriter<'_>,
    stats: &mut SnapshotStats,
) -> Result<(), ReplicateError> {
    w.write_bits(u64::from(tick), 32)?;
    let distance = baseline.map_or(0, |base| u64::from(tick.saturating_sub(base)));
    w.write_varint(distance)?;
    w.write_varint(chosen.len() as u64)?;
    let mut ids = IdGapEncoder::new();
    for &(id, span) in chosen {
        ids.write(w, id)?;
        w.write_bits(span.op, 2)?;
        match span.op {
            OP_REMOVE => stats.removed += 1,
            OP_FULL => {
                stats.full += 1;
                cache.copy(span, w)?;
            }
            _ => {
                stats.delta += 1;
                cache.copy(span, w)?;
            }
        }
    }
    Ok(())
}

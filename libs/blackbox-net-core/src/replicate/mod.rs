//! Entity replication over a registered schema (ADR 0009 sections 1, 2 and 4), built on [`crate::bits`].
//!
//! CONTRACT (to implement). Wave A5 builds the codec layer (schema, frames, codec, cache, spatial hash); wave A6 builds
//! the per-client server state, the client views and the end-to-end tests on top of it.
//!
//! **Model.** The game quantises each entity into `u32` field values (with [`crate::bits::Quant`] or its own rules) and
//! writes them once per tick into a [`History`]-to-be. The generic layer only compares and packs integers.
//!
//! ## A5: codec layer
//!
//! - `error.rs`: `pub enum ReplicateError { Bits(BitError), BadSchema, TooManyKinds, BadOrder, BadValue, UnknownKind,
//!   UnknownEntity, MissingBaseline, BadOp }` with `Display`, `std::error::Error` and `From<BitError>`.
//! - `schema.rs`:
//!   - `pub struct FieldDesc { pub bits: u8, pub diff: bool }`: a field of `bits` (1..=32) bits; `diff` marks a field whose
//!     delta is coded as a difference (positions).
//!   - `pub struct KindId(pub u8)` (Copy, Eq, Ord, Hash, Debug).
//!   - `pub struct Schema` with `new()`, `add_kind(&mut self, fields: &[FieldDesc]) -> Result<KindId, ReplicateError>`
//!     (at most [`MAX_KINDS`] kinds, else `TooManyKinds`; 1..=[`MAX_FIELDS`] fields with bits in 1..=32, else
//!     `BadSchema`), `fields(kind) -> Option<&[FieldDesc]>`, `kind_count()`, `full_bits(kind) -> Option<usize>`
//!     (6 + the sum of the field bits) and `schema_id() -> u64`: 64-bit FNV-1a (offset `0xcbf29ce484222325`, prime
//!     `0x100000001b3`) over the bytes `b"BBNS"`, the kind count, then per kind its field count and per field `bits`
//!     and `diff as u8`.
//! - `frame.rs`: `pub struct Frame`: one tick of entities sorted by id, stored flat (a `Vec<Entry { id: u16, kind: KindId,
//!   start: u32 }>` and a `Vec<u32>` of values) so clearing keeps capacity. `tick()`, `len()`, `is_empty()`,
//!   `get(id) -> Option<(KindId, &[u32])>` (binary search), `iter() -> impl Iterator<Item = (u16, KindId, &[u32])>`,
//!   `clear(tick)`, `push(&mut self, schema, id, kind, values) -> Result<(), ReplicateError>` (id not above the last one is
//!   `BadOrder`; unknown kind `UnknownKind`; wrong value count `BadSchema`; a value that does not fit its bits
//!   `BadValue`). The client's view of the world is also a `Frame`.
//! - `history.rs`: `pub struct History` a ring of `capacity` frames (64 in use). `new(capacity)`, `begin(&mut self,
//!   tick) -> &mut Frame` (reuses the oldest slot, clears it to `tick`; ticks must increase, an older or equal tick
//!   replaces nothing and returns the slot anyway after clearing — document it), `frame(tick) -> Option<&Frame>`,
//!   `latest_tick() -> Option<u32>`.
//! - `codec.rs` (entry bodies; the operation and id are written by the caller):
//!   - `pub const OP_DELTA: u64 = 0b01; OP_FULL: u64 = 0b10; OP_REMOVE: u64 = 0b11;` (2 bits; `0b00` is `BadOp`).
//!   - `encode_full(schema, kind, values, w)`: kind in 6 bits, then every field raw, MSB first.
//!   - `encode_delta(schema, kind, old, new, w) -> Result<bool, ReplicateError>`: if `old == new` writes nothing and
//!     returns `false`. Else a mask of `field_count` bits (field 0 first, 1 = changed), then each changed field: a `diff`
//!     field as `write_zigzag`-style zigzag of `new - old` (as `i64`) with `write_varint_n(.., 4)`; a raw field as its
//!     `bits` bits. Returns `true`.
//!   - `decode_full(schema, r, out: &mut [u32; MAX_FIELDS]) -> Result<(KindId, usize), ReplicateError>` (count of
//!     values written to `out`).
//!   - `decode_delta(schema, kind, old: &[u32], r, out: &mut [u32; MAX_FIELDS]) -> Result<usize, ReplicateError>`:
//!     a decoded value outside `0..2^bits` is `BadValue`.
//!   - `delta_bits` is not needed: the cache measures what was written.
//! - `cache.rs`: `pub struct EncodeCache`: the bodies of one tick. `new()`, `reset(&mut self, tick)` (clears the spans and
//!   the bit buffer, keeps capacity), `tick()`, `full(&mut self, schema, frame: &Frame, id) -> Result<Span,
//!   ReplicateError>` and `delta(&mut self, schema, base: &Frame, frame: &Frame, id) -> Result<Span, ReplicateError>`:
//!   each returns the cached span or encodes into the internal bit buffer first (a `Vec<u8>` grown on demand plus a bit
//!   length; write with a `BitWriter` over the spare tail, growing the `Vec` by doubling when `Overflow`). Keys:
//!   `(id, u32::MAX)` for full, `(id, base.tick())` for delta, in a `HashMap` that is cleared, not dropped. `pub struct
//!   Span { pub op: u64, pub start: usize, pub bits: usize }`: a delta of an unchanged entity is `bits == 0`; a delta
//!   becomes a full record (op `OP_FULL`) when the full record is not larger or the kinds differ or `id` is missing from
//!   `base`. `copy(&self, span, w: &mut BitWriter) -> Result<(), BitError>` writes the span's bits (in 64-bit chunks with
//!   `write_bits`). Counters `hits()` and `misses()`.
//! - `spatial.rs`: `pub struct SpatialHash` over the horizontal plane: `new(cell_size: f32)`, `clear()` (keeps every
//!   cell's capacity), `insert(id: u16, x: f32, z: f32)`, `query(&self, x, z, radius, out: &mut Vec<u16>)`: clears `out`,
//!   pushes ids within `radius` (exact distance check), sorted ascending without duplicates.
//! - Tests: schema limits and a fixed `schema_id` for a fixed schema (hard-code the value once computed); frame order and
//!   value checks; random full and delta round trips (diff and raw fields, values at 0 and the top of the range, deltas
//!   crossing the whole range); unchanged gives no bits; the cache returns identical bits on a hit and counts hits; a delta
//!   larger than the full record is sent full; spatial query against a brute-force scan on 2,000 random points.
//!
//! ## A6: per-client state, client views, end to end
//!
//! - `server.rs`: `pub struct ClientState` (one per client): `new()`, `on_delivered(&mut self, tick)` (the newest
//!   delivered snapshot tick is the baseline), and `write_snapshot(&mut self, schema, history, cache, tick, candidates:
//!   &[(u16, f32)], budget_bits, w) -> Result<SnapshotStats, ReplicateError>`. `candidates` are the entities relevant to
//!   this client, sorted by id, with a priority weight. Snapshot layout: `tick` 32 bits, baseline distance varint (0 =
//!   none), entry count varint, then entries in id order: id gap (`IdGapEncoder`), op 2 bits, body (cached span).
//!   Steps: pick the baseline (the newest delivered tick whose frame is still in `history` and whose sent view is still
//!   kept); every entity of the baseline view that is not a candidate is a remove; a candidate whose source frame left the
//!   history is a forced full; every other changed or new candidate competes by its priority accumulator (`+= weight` per
//!   snapshot, reset to 0 when sent) for `budget_bits`; unchanged entities move their source tick to `tick` for free. The
//!   view recorded for `tick` is `(id, source_tick)` per entity the client then holds. Buffers reused; nothing allocated
//!   per call after warm-up except when the entity id range grows.
//! - `client.rs`: `pub struct ClientViews` (a ring of 64 `Frame`s): `read_snapshot(&mut self, schema, r) -> Result<u32,
//!   ReplicateError>` applies entries onto the baseline view (`MissingBaseline` if it is not kept) into a new view for
//!   the tick; `view(tick)`, `latest()`.
//! - Integration tests, the allocation test and the `net_bench` example: see the A6 task.

/// Most entity kinds a schema can register (they are written in 6 bits).
pub const MAX_KINDS: usize = 64;
/// Most fields in one kind (the field mask is at most 32 bits).
pub const MAX_FIELDS: usize = 32;

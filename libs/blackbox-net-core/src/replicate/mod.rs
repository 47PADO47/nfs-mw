//! Entity replication over a registered schema (ADR 0009 sections 1, 2 and 4), built on [`crate::bits`].
//!
//! The game quantises each entity into `u32` field values (with [`crate::bits::Quant`] or its own rules) and stores one
//! [`Frame`] per tick in a [`History`]. The generic layer only compares and packs integers. This module holds the codec
//! layer: the schema, the frames, the entry bodies, the per-tick encode cache and the spatial hash for relevancy.
//!
//! - [`ReplicateError`]: every failure on malformed input, never a panic.
//! - [`Schema`], [`KindId`] and [`FieldDesc`]: the kinds the game registers. A field has `bits` (1..=32) and `diff`,
//!   which codes its change as a difference (for positions) instead of raw bits. [`Schema::schema_id`] fingerprints the
//!   layout for the handshake.
//! - [`Frame`]: one tick of entities sorted by id, stored flat so clearing keeps its capacity. The client's view is a
//!   `Frame` too.
//! - [`History`]: a ring of frames, one slot per tick modulo the capacity.
//! - [`encode_full`], [`encode_delta`], [`decode_full`] and [`decode_delta`]: the entry bodies. A full record is the kind
//!   id in 6 bits, then every field raw, most significant bit first. A delta is a changed-field mask (field 0 first,
//!   1 = changed), then each changed field: a `diff` field as the zigzag of `new - old` in a varint with 4 data bits per
//!   group, any other field as its raw bits. An unchanged entity writes nothing. The operation codes are [`OP_DELTA`],
//!   [`OP_FULL`] and [`OP_REMOVE`] (2 bits; `0b00` is [`ReplicateError::BadOp`]). The caller writes the entry id and
//!   the operation code around the body.
//! - [`EncodeCache`] and [`Span`]: the bodies of one tick, each encoded once and copied to every client that needs
//!   them. A delta that is not smaller than the full record is sent full.
//! - [`SpatialHash`]: points on the horizontal plane, with a radius query that returns sorted ids.
//!
//! The codec is allocation free after warm-up: [`Frame::clear`], [`EncodeCache::reset`] and
//! [`SpatialHash::clear`] keep their capacity, and the decoders write into caller arrays.
//!
//! ## To build next (A6)
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

mod cache;
mod codec;
mod error;
mod frame;
mod history;
mod schema;
mod spatial;

#[cfg(test)]
mod tests;

pub use cache::{EncodeCache, Span};
pub use codec::{OP_DELTA, OP_FULL, OP_REMOVE, decode_delta, decode_full, encode_delta, encode_full};
pub use error::ReplicateError;
pub use frame::Frame;
pub use history::History;
pub use schema::{FieldDesc, KindId, Schema};
pub use spatial::SpatialHash;

/// Most entity kinds a schema can register (they are written in 6 bits).
pub const MAX_KINDS: usize = 64;
/// Most fields in one kind (the field mask is at most 32 bits).
pub const MAX_FIELDS: usize = 32;

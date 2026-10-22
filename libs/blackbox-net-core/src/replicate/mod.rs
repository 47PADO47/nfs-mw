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
//! ## Per-client state: server and client
//!
//! - [`ClientState`] (server side, one per client) records the views the client has been sent: for each tick, the
//!   `(id, source tick)` pairs of the entities it holds. A view entry names the server tick whose frame gives the
//!   entity's values, so nothing is copied per client. [`ClientState::on_delivered`] takes the newest acknowledged
//!   snapshot tick as the baseline. [`ClientState::write_snapshot`] then decides, for each candidate entity (sorted by
//!   id, with a priority weight): an unchanged entity moves its view to the tick for free; a removal is mandatory, as
//!   is a full record when the source frame has left the history; a changed entity competes for the byte budget by a
//!   priority accumulator that grows by the weight each snapshot and resets when the entity is sent, so every entity
//!   is sent eventually. [`ClientState::reset`] forgets the views after the client reports a lost baseline, so the next
//!   snapshot is full.
//! - [`ClientViews`] (client side) keeps the last 64 decoded views. `read_snapshot` takes the baseline view that the
//!   snapshot names, copies the entities the snapshot does not touch, applies the entries and keeps the result as the
//!   view of the snapshot's tick.
//!
//! Snapshot layout: tick (32 bits), baseline distance (varint, 0 = none), entry count (varint), then the entries in id
//! order: id gap ([`crate::bits::IdGapEncoder`]), operation (2 bits), body (a cached span, nothing for a removal).
//! The per-tick path allocates nothing after warm-up: every list is reused, and an accumulator grows only when an id
//! beyond its length appears.

mod cache;
mod client;
mod codec;
mod error;
mod frame;
mod history;
mod schema;
mod server;
mod spatial;

#[cfg(test)]
mod tests;

pub use cache::{EncodeCache, Span};
pub use client::ClientViews;
pub use codec::{OP_DELTA, OP_FULL, OP_REMOVE, decode_delta, decode_full, encode_delta, encode_full};
pub use error::ReplicateError;
pub use frame::Frame;
pub use history::History;
pub use schema::{FieldDesc, KindId, Schema};
pub use server::{ClientState, SnapshotStats};
pub use spatial::SpatialHash;

/// Most entity kinds a schema can register (they are written in 6 bits).
pub const MAX_KINDS: usize = 64;
/// Most fields in one kind (the field mask is at most 32 bits).
pub const MAX_FIELDS: usize = 32;

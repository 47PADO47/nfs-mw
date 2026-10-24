//! The reliable message catalogue (protocol.md section 7) and its codec.
//!
//! **Encoding.** A message is `type` (u16 little-endian), `format` (u8), then the body in postcard's wire format,
//! written by hand so that every decode is bounded and the crate gains no dependency: `u8` one raw byte, wider unsigned
//! integers LEB128 varints, signed integers zigzag varints, `bool` one byte 0 or 1, `f32` 4 bytes little-endian,
//! arrays element by element, `Option` a tag byte then the value, strings, byte strings and lists a varint count then
//! the elements, enums a varint tag. Fields follow the declaration order of the body struct (`macros.rs`).
//!
//! **Decoding rules** (`security-gameplay.md` section 1.1): [`decode`] checks the whole message against
//! [`MAX_MESSAGE_BYTES`] first; every count is checked against its limit and the bytes left before anything is
//! reserved; varints must be canonical; bools, enum tags, slots and ranges are checked; floats must be finite; text is
//! UTF-8 with no control or bidirectional characters, trimmed, within its length ([`Text`]). Nothing panics: garbage
//! gives a [`DecodeError`]. An unknown type is [`DecodeError::UnknownType`], which the caller skips and counts
//! ([`InvalidMessages`]).
//!
//! **Growth** (protocol.md section 8.2). A new field is appended to a body and the type's format goes up. A reader
//! that meets a newer format than it knows decodes its fields and skips the rest; at its own format, trailing bytes are
//! [`DecodeError::TrailingBytes`]. An older format than the reader knows is [`DecodeError::BadFormat`] until a reader
//! learns to fill the missing fields. Every type is format 1 today.
//!
//! **Type numbers** (ADR 0009): admission `0x0001`-`0x0008`, lobby and session `0x0010`-`0x0020`, chat `0x0200`-
//! `0x0203`, bulk `0x0300`-`0x0302` in the core namespace; gameplay events `0x1100`-`0x1150` in the game namespace.
//! The table is in `catalogue.rs` and in `docs/specs/multiplayer/protocol-messages.md`.

mod admission;
mod catalogue;
mod chat;
pub(crate) mod codec;
mod common;
mod events;
mod lobby;
pub(crate) mod macros;
mod settings;
mod strikes;
mod text;
pub(crate) mod wire;

#[cfg(test)]
mod tests;

pub use admission::*;
pub use catalogue::{Channel, HEADER_LEN, MAX_MESSAGE_BYTES, Message, decode, encode, peek_type};
pub use chat::*;
pub use codec::{DecodeError, EncodeError, LimitError, Reader, Writer};
pub use common::{MAX_SLOTS, Mode, Role, SessionPhase, is_slot};
pub use events::*;
pub use lobby::*;
pub use settings::*;
pub use strikes::{INVALID_LIMIT, INVALID_WINDOW_MS, InvalidMessages};
pub use text::password_secret;
pub use text::{ChatText, Name, Plate, ServerName, Text, VersionText};
pub use wire::{Bytes, List, Wire, wire_len};

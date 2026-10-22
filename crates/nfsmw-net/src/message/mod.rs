//! The codec of the reliable messages (protocol.md section 7): postcard's wire format, written by hand so that every
//! decode is bounded and the crate gains no dependency.

pub(crate) mod codec;
pub(crate) mod macros;
mod text;
pub(crate) mod wire;

pub use codec::{DecodeError, EncodeError, LimitError, Reader, Writer};
pub use text::{ChatText, Name, Plate, ServerName, Text, VersionText};
pub use wire::{Bytes, List, Wire, wire_len};

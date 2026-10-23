//! Race results and session stats (MP-70) as a fixed, versioned record, and its conversion to the `RaceResults` event
//! (protocol-messages.md section 7.6). MP-74 stores these records later; the format is fixed now so it can.
//!
//! - [`Placing`]: one player's place and race facts, as a mode's rules give them ([`crate::modes`]).
//! - [`Tally`]: the session stats the server counts from the same inputs as the modes: top speed, damage, busted count,
//!   pursuit time and resets.
//! - [`Roster`]: the slot and entity id of each player, for the wire.
//! - [`RaceRecord`] and [`RecordRow`]: the stored record. [`RaceRecord::encode`] and [`RaceRecord::decode`] follow the
//!   growth rule of the reliable messages; [`build_record`] joins placings, a tally and a roster;
//!   [`RaceRecord::to_message`] gives the `RaceResults` event.
//!
//! **Record layout, format 1.** The magic `NMRR`, a format byte, then the body in the reliable messages' wire format
//! (protocol-messages.md section 7.0): mode (one byte), track (varint), laps (u8), start and end tick (varints), then at
//! most 32 rows, each: slot (u8), entity id, place (u8, 0 in a mode without places), flags (u8), time in ms, best lap in
//! ms, top speed in cm/s, damage (u8, 0..=255 for 0..1), busted count, pursuit time in ms, resets, points (f32).
//!
//! **Growth.** A later format appends fields to the body; a field a row needs is a new list appended to the body, one
//! entry per row, because the row layout of format 1 is fixed. A reader meeting a newer format decodes the fields it
//! knows and skips the rest; at its own format, trailing bytes are an error; format 0 is an error.
//!
//! **Flags.** [`FLAG_DNF`] and [`FLAG_BUSTED`] are the bits of the wire's result row; [`FLAG_KNOCKED_OUT`] and
//! [`FLAG_LEFT`] are record-only and become the wire's DNF bit.

mod build;
mod placing;
mod record;
mod roster;
mod tally;

#[cfg(test)]
mod tests;

pub use build::{RecordMeta, build_record};
pub use placing::{FLAG_BUSTED, FLAG_DNF, FLAG_KNOCKED_OUT, FLAG_LEFT, NOT_FINISHED, Placing};
pub use record::{MAX_RECORD_ROWS, RECORD_FORMAT, RECORD_MAGIC, RaceRecord, RecordError, RecordRow};
pub use roster::{Roster, Seat};
pub use tally::{Stats, Tally, ticks_to_ms};

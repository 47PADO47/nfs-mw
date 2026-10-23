//! The stored per-race record: its rows, its body and the magic-and-format envelope (module docs of [`super`]).

use crate::message::macros::wire_struct;
use crate::message::{
    DecodeError, EventHead, List, MAX_SLOTS, Mode, RaceResults, Reader, ResultRow, Wire, Writer, is_slot,
};

use super::placing::{FLAG_BUSTED, NOT_FINISHED};

/// The first bytes of an encoded record.
pub const RECORD_MAGIC: [u8; 4] = *b"NMRR";
/// The record format this build writes.
pub const RECORD_FORMAT: u8 = 1;
/// Most rows in one record: one per slot.
pub const MAX_RECORD_ROWS: usize = MAX_SLOTS;

/// Why a stored record could not be read.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum RecordError {
    #[error("not a result record")]
    BadMagic,
    #[error("unknown result record format {0}")]
    BadFormat(u8),
    #[error("bad result record body: {0}")]
    Decode(#[from] DecodeError),
}

fn check(ok: bool) -> Result<(), DecodeError> {
    match ok {
        true => Ok(()),
        false => Err(DecodeError::BadValue),
    }
}

wire_struct! {
    /// One player's line of the stored record.
    #[derive(Copy, Default)]
    pub struct RecordRow {
        pub slot: u8,
        pub entity_id: u16,
        /// From 1; 0 in a mode without places.
        pub place: u8,
        /// The flags of [`super::Placing`].
        pub flags: u8,
        /// The race time with penalties (0 when none).
        pub time_ms: u32,
        /// The best lap (0 when none).
        pub best_lap_ms: u32,
        /// The top speed in cm/s.
        pub top_speed_cms: u32,
        /// The highest damage, 0..=255 for 0..1.
        pub damage: u8,
        pub busted: u16,
        pub pursuit_ms: u32,
        pub resets: u16,
        /// Speed-trap points.
        pub points: f32,
    }
    check(row) { check(is_slot(row.slot)) }
}

wire_struct! {
    /// The result of one race or one session of a mode without races.
    pub struct RaceRecord {
        pub mode: Mode,
        /// The track or region id.
        pub track: u32,
        pub laps: u8,
        pub start_tick: u32,
        pub end_tick: u32,
        /// In place order; unranked rows last.
        pub rows: List<RecordRow, MAX_RECORD_ROWS>,
    }
    check(record) { record.check() }
}

impl RaceRecord {
    /// The rules across fields: the end is not before the start, slots are unique, and places other than 0 are unique.
    pub fn check(&self) -> Result<(), DecodeError> {
        check(self.end_tick >= self.start_tick)?;
        let mut slots = 0u64;
        let mut places = 0u64;
        for row in self.rows.iter() {
            check(is_slot(row.slot))?;
            let slot_bit = 1u64 << row.slot;
            check(slots & slot_bit == 0)?;
            slots |= slot_bit;
            if row.place == 0 {
                continue;
            }
            check(usize::from(row.place) <= MAX_RECORD_ROWS)?;
            let place_bit = 1u64 << row.place;
            check(places & place_bit == 0)?;
            places |= place_bit;
        }
        Ok(())
    }

    /// Appends the magic, the format byte and the body to `out`.
    pub fn encode(&self, out: &mut Vec<u8>) {
        out.extend_from_slice(&RECORD_MAGIC);
        out.push(RECORD_FORMAT);
        self.put(&mut Writer::new(out));
    }

    /// Reads a record. A newer format's extra fields are skipped; at this build's format, trailing bytes are an error.
    pub fn decode(bytes: &[u8]) -> Result<Self, RecordError> {
        let mut reader = Reader::new(bytes);
        let magic = reader.take(RECORD_MAGIC.len()).map_err(|_| RecordError::BadMagic)?;
        if magic != RECORD_MAGIC {
            return Err(RecordError::BadMagic);
        }
        let format = reader.u8()?;
        if format == 0 {
            return Err(RecordError::BadFormat(format));
        }
        let record = Self::take(&mut reader)?;
        if format == RECORD_FORMAT && !reader.is_empty() {
            return Err(RecordError::Decode(DecodeError::TrailingBytes));
        }
        Ok(record)
    }

    /// The `RaceResults` event of this record. Knocked-out and departed players carry the wire's DNF bit; the other
    /// record-only flags are dropped. `head` is the caller's (the sequence number is per connection).
    pub fn to_message(&self, head: EventHead) -> RaceResults {
        let mut rows = List::empty();
        for row in self.rows.iter() {
            let mut flags = row.flags & FLAG_BUSTED;
            if row.flags & NOT_FINISHED != 0 {
                flags |= crate::message::RESULT_DNF;
            }
            let line = ResultRow {
                slot: row.slot,
                entity_id: row.entity_id,
                place: row.place,
                time_ms: row.time_ms,
                best_lap_ms: row.best_lap_ms,
                flags,
            };
            // A record holds at most as many rows as the message, so the push cannot fail.
            let _ = rows.push(line);
        }
        RaceResults { head, rows }
    }
}

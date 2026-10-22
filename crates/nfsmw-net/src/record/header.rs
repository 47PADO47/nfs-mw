//! The replay header (module docs of [`super`]): who recorded, with which protocol, schema, sim build and game data.

use nfsmw_sim::{PlayerId, TICK_HZ, Tick};

use crate::version::{DataId, SimBuild};

use super::crc::crc32;

/// The first bytes of a replay file.
pub const REPLAY_MAGIC: [u8; 8] = *b"NFSMWRPL";
/// The format this build writes.
pub const REPLAY_FORMAT: u16 = 1;
/// Bytes of a format-1 header, its CRC included.
pub const HEADER_LEN: usize = 57;
/// The longest header a reader accepts (a later format may append fields).
const MAX_HEADER_LEN: usize = 1024;

/// Who wrote the recording.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RecordKind {
    /// The server's stream: snapshots and events (MP-71).
    Server,
    /// A client's view: received snapshots and events, own inputs and predicted states (MP-72).
    Client,
}

/// Why a replay cannot be read at all.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, thiserror::Error)]
pub enum ReplayError {
    #[error("not a replay file")]
    BadMagic,
    #[error("unknown replay format {0}")]
    BadFormat(u16),
    #[error("the replay header is damaged")]
    BadHeader,
    #[error("the file ends inside the replay header")]
    Truncated,
}

/// The facts a replay starts with (telemetry F-33: schema id, protocol minor, sim build, data id).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ReplayHeader {
    pub kind: RecordKind,
    /// The recording client's own player; player 0 on a server.
    pub player: PlayerId,
    /// The entity schema id ([`crate::schema::GameSchema::schema_id`]).
    pub schema_id: u64,
    pub proto_minor: u16,
    pub sim_build: SimBuild,
    pub data_id: DataId,
    /// The tick of the first frame or earlier.
    pub start_tick: Tick,
    /// Ticks per second.
    pub tick_hz: u8,
}

impl ReplayHeader {
    /// A header of `kind` at 60 Hz.
    pub fn new(kind: RecordKind, schema_id: u64, sim_build: SimBuild, data_id: DataId, start_tick: Tick) -> Self {
        Self {
            kind,
            player: PlayerId(0),
            schema_id,
            proto_minor: crate::version::PROTO_MINOR,
            sim_build,
            data_id,
            start_tick,
            tick_hz: TICK_HZ as u8,
        }
    }

    /// Appends the format-1 header.
    pub fn encode(&self, out: &mut Vec<u8>) {
        let start = out.len();
        out.extend_from_slice(&REPLAY_MAGIC);
        out.extend_from_slice(&REPLAY_FORMAT.to_le_bytes());
        out.extend_from_slice(&(HEADER_LEN as u16).to_le_bytes());
        out.push(match self.kind {
            RecordKind::Server => 0,
            RecordKind::Client => 1,
        });
        out.push(self.player.0);
        out.extend_from_slice(&self.schema_id.to_le_bytes());
        out.extend_from_slice(&self.proto_minor.to_le_bytes());
        out.extend_from_slice(&self.sim_build.bytes);
        out.extend_from_slice(&self.data_id.bytes);
        out.extend_from_slice(&self.start_tick.to_le_bytes());
        out.push(self.tick_hz);
        let crc = crc32(&out[start..]);
        out.extend_from_slice(&crc.to_le_bytes());
    }

    /// Reads the header at the start of `bytes`; returns it and its length.
    pub fn decode(bytes: &[u8]) -> Result<(Self, usize), ReplayError> {
        if bytes.len() < 12 {
            return Err(ReplayError::Truncated);
        }
        if bytes[..8] != REPLAY_MAGIC {
            return Err(ReplayError::BadMagic);
        }
        let format = u16::from_le_bytes([bytes[8], bytes[9]]);
        if format == 0 {
            return Err(ReplayError::BadFormat(format));
        }
        let len = usize::from(u16::from_le_bytes([bytes[10], bytes[11]]));
        if !(HEADER_LEN..=MAX_HEADER_LEN).contains(&len) {
            return Err(ReplayError::BadHeader);
        }
        if format == REPLAY_FORMAT && len != HEADER_LEN {
            return Err(ReplayError::BadHeader);
        }
        let head = bytes.get(..len).ok_or(ReplayError::Truncated)?;
        let stored = u32::from_le_bytes([head[len - 4], head[len - 3], head[len - 2], head[len - 1]]);
        if crc32(&head[..len - 4]) != stored {
            return Err(ReplayError::BadHeader);
        }
        let kind = match head[12] {
            0 => RecordKind::Server,
            1 => RecordKind::Client,
            _ => return Err(ReplayError::BadHeader),
        };
        let mut sim_build = [0u8; 8];
        sim_build.copy_from_slice(&head[24..32]);
        let mut data_id = [0u8; 16];
        data_id.copy_from_slice(&head[32..48]);
        let header = Self {
            kind,
            player: PlayerId(head[13]),
            schema_id: u64::from_le_bytes(array(&head[14..22])),
            proto_minor: u16::from_le_bytes([head[22], head[23]]),
            sim_build: SimBuild { bytes: sim_build },
            data_id: DataId { bytes: data_id },
            start_tick: u32::from_le_bytes([head[48], head[49], head[50], head[51]]),
            tick_hz: head[52],
        };
        Ok((header, len))
    }
}

/// The 8 bytes of `slice` (which the caller sizes) as an array.
fn array(slice: &[u8]) -> [u8; 8] {
    let mut out = [0u8; 8];
    out.copy_from_slice(slice);
    out
}

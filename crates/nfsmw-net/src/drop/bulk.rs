//! Bulk transfers on the ordered `bulk` channel (protocol-messages.md 7.8): the sender splits a blob into `BulkStart`,
//! `BulkChunk`s in offset order and `BulkEnd`, paced by a byte budget the caller gives per call; the receiver checks
//! the length against its own limit before taking any chunk, takes chunks only in order, grows its buffer only as
//! chunks arrive, and checks the length and the SHA-256 at the end.

use sha2::{Digest, Sha256};

use crate::message::{BulkChunk, BulkEnd, BulkKind, BulkStart, Bytes, MAX_BULK_CHUNK, MAX_BULK_TRANSFER, Message};

/// Why a transfer was refused or abandoned. A receiver that refuses a message drops the whole transfer.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BulkError {
    /// Longer than the limit.
    TooLarge,
    /// A chunk or end without a start.
    NotStarted,
    /// A start while another transfer is open.
    Busy,
    /// A chunk or end of another transfer id.
    WrongId,
    /// A chunk whose offset is not where the last one ended.
    OutOfOrder,
    /// A chunk past the announced length.
    Overrun,
    /// The end came before the announced length.
    Short,
    /// The bytes do not hash to the announced SHA-256.
    HashMismatch,
}

/// A transfer the receiver finished and verified.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Complete {
    pub kind: BulkKind,
    pub id: u16,
    pub data: Vec<u8>,
}

/// The sending side of one transfer.
#[derive(Debug, Clone)]
pub struct BulkSender {
    start: BulkStart,
    data: Vec<u8>,
    sent: usize,
    started: bool,
    ended: bool,
}

impl BulkSender {
    /// A transfer of `data` as `kind` with transfer id `id`. [`BulkError::TooLarge`] over [`MAX_BULK_TRANSFER`].
    pub fn new(kind: BulkKind, id: u16, data: Vec<u8>) -> Result<Self, BulkError> {
        let length = u32::try_from(data.len()).map_err(|_| BulkError::TooLarge)?;
        if length > MAX_BULK_TRANSFER {
            return Err(BulkError::TooLarge);
        }
        let mut hash = [0u8; 32];
        hash.copy_from_slice(&Sha256::digest(&data));
        let start = BulkStart { kind, id, length, hash };
        Ok(Self { start, data, sent: 0, started: false, ended: false })
    }

    /// The next messages within `budget` chunk bytes: the start on the first call, chunks of at most
    /// [`MAX_BULK_CHUNK`] bytes (smaller when the budget is smaller), and the end after the last chunk. Empty once done.
    pub fn next(&mut self, budget: usize) -> Vec<Message> {
        let mut out = Vec::new();
        if self.ended {
            return out;
        }
        if !self.started {
            self.started = true;
            out.push(Message::BulkStart(self.start.clone()));
        }
        let mut left = budget;
        while self.sent < self.data.len() && left > 0 {
            let take = (self.data.len() - self.sent).min(MAX_BULK_CHUNK).min(left);
            let bytes = self.data[self.sent..self.sent + take].to_vec();
            let offset = u32::try_from(self.sent).unwrap_or(u32::MAX);
            let Ok(bytes) = Bytes::new(bytes) else {
                break;
            };
            out.push(Message::BulkChunk(BulkChunk { id: self.start.id, offset, bytes }));
            self.sent += take;
            left -= take;
        }
        if self.sent == self.data.len() {
            self.ended = true;
            out.push(Message::BulkEnd(BulkEnd { id: self.start.id }));
        }
        out
    }

    /// Whether the end was sent.
    pub fn is_done(&self) -> bool {
        self.ended
    }
}

/// The receiving side: one open transfer at a time.
#[derive(Debug, Clone)]
pub struct BulkReceiver {
    limit: u32,
    open: Option<(BulkStart, Vec<u8>)>,
}

impl BulkReceiver {
    /// A receiver that accepts transfers of at most `limit` bytes (never more than [`MAX_BULK_TRANSFER`]).
    pub fn new(limit: u32) -> Self {
        Self { limit: limit.min(MAX_BULK_TRANSFER), open: None }
    }

    /// Takes one message of the bulk channel. Returns the finished transfer at its end; other messages are ignored.
    /// On an error the open transfer is dropped, and the caller asks again or closes the connection.
    pub fn accept(&mut self, message: &Message) -> Result<Option<Complete>, BulkError> {
        let result = self.step(message);
        if result.is_err() {
            self.open = None;
        }
        result
    }

    /// Whether a transfer is open.
    pub fn is_open(&self) -> bool {
        self.open.is_some()
    }

    fn step(&mut self, message: &Message) -> Result<Option<Complete>, BulkError> {
        match message {
            Message::BulkStart(start) => self.begin(start),
            Message::BulkChunk(chunk) => self.chunk(chunk),
            Message::BulkEnd(end) => self.end(end.id),
            _ => Ok(None),
        }
    }

    fn begin(&mut self, start: &BulkStart) -> Result<Option<Complete>, BulkError> {
        if self.open.is_some() {
            return Err(BulkError::Busy);
        }
        if start.length > self.limit {
            return Err(BulkError::TooLarge);
        }
        self.open = Some((start.clone(), Vec::new()));
        Ok(None)
    }

    fn chunk(&mut self, chunk: &BulkChunk) -> Result<Option<Complete>, BulkError> {
        let (start, data) = self.open.as_mut().ok_or(BulkError::NotStarted)?;
        if chunk.id != start.id {
            return Err(BulkError::WrongId);
        }
        if chunk.offset as usize != data.len() {
            return Err(BulkError::OutOfOrder);
        }
        if data.len() + chunk.bytes.len() > start.length as usize {
            return Err(BulkError::Overrun);
        }
        data.extend_from_slice(&chunk.bytes);
        Ok(None)
    }

    fn end(&mut self, id: u16) -> Result<Option<Complete>, BulkError> {
        let (start, data) = self.open.as_ref().ok_or(BulkError::NotStarted)?;
        if id != start.id {
            return Err(BulkError::WrongId);
        }
        if data.len() != start.length as usize {
            return Err(BulkError::Short);
        }
        if Sha256::digest(data).as_slice() != start.hash.as_slice() {
            return Err(BulkError::HashMismatch);
        }
        let (start, data) = self.open.take().ok_or(BulkError::NotStarted)?;
        Ok(Some(Complete { kind: start.kind, id: start.id, data }))
    }
}

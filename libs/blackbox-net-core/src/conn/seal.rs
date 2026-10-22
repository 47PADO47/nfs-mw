//! The seal slot of a connection: what turns a plaintext packet into the datagram on the wire and back (ADR 0009
//! section 3, protocol.md section 5.1). The connection stays sans-IO and knows no cipher; it calls a [`Sealer`].
//!
//! Two sealers exist: [`Plain`] for the in-memory link of a listen host's own client (nothing outside the process can
//! reach it, so there is no handshake and no tag), and the Noise sealer of the `secure` module (feature `netcode`).

use super::ConnError;

/// Bytes at the start of every packet that stay in the clear and are authenticated: flags (1), connection id (2) and
/// packet number (4). The acknowledgement block and the frames after them are sealed.
pub const CLEAR_HEADER_LEN: usize = 7;

/// Flag bits 6 and 5: the key phase of a sealed packet. A sealer sets them on [`Sealer::seal`] and clears them on
/// [`Sealer::open`], so the connection's own header parser never sees them.
pub const KEY_PHASE_MASK: u8 = 0b0110_0000;

/// Seals outgoing packets and opens incoming ones in place.
pub trait Sealer {
    /// Bytes [`Sealer::seal`] appends to a packet (the authentication tag). At most [`super::SEAL_OVERHEAD`].
    fn overhead(&self) -> usize;

    /// Seals `packet[..len]` in place and returns the sealed length. The first [`CLEAR_HEADER_LEN`] bytes stay readable
    /// (the sealer may set the [`KEY_PHASE_MASK`] bits of byte 0 first) and are the associated data; the rest is
    /// encrypted and the tag is appended. `packet` has room for `len + overhead()` bytes.
    fn seal(&mut self, now_ms: u64, packet: &mut [u8], len: usize) -> Result<usize, ConnError>;

    /// Opens the sealed `datagram` in place and returns the plaintext length. On success the key phase bits of byte 0
    /// are clear again. A failure ([`ConnError::BadTag`] for a forged, corrupted or replayed-under-an-old-key packet)
    /// changes no sealer state, so a forged packet costs one tag check and nothing else.
    fn open(&mut self, now_ms: u64, datagram: &mut [u8]) -> Result<usize, ConnError>;
}

/// The sealer of the in-memory link: no tag, no encryption. Packets with key phase bits set are refused later by the
/// header parser, as before sealing existed.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Plain;

impl Sealer for Plain {
    fn overhead(&self) -> usize {
        0
    }

    fn seal(&mut self, _now_ms: u64, _packet: &mut [u8], len: usize) -> Result<usize, ConnError> {
        Ok(len)
    }

    fn open(&mut self, _now_ms: u64, datagram: &mut [u8]) -> Result<usize, ConnError> {
        Ok(datagram.len())
    }
}

/// A boxed sealer seals like the sealer inside, so a server can keep plain and Noise connections in one list as
/// `Connection<Box<dyn Sealer + Send>>`.
impl<S: Sealer + ?Sized> Sealer for Box<S> {
    fn overhead(&self) -> usize {
        (**self).overhead()
    }

    fn seal(&mut self, now_ms: u64, packet: &mut [u8], len: usize) -> Result<usize, ConnError> {
        (**self).seal(now_ms, packet, len)
    }

    fn open(&mut self, now_ms: u64, datagram: &mut [u8]) -> Result<usize, ConnError> {
        (**self).open(now_ms, datagram)
    }
}

/// The clear part of a packet header: flags, connection id and packet number. Parsed before any decryption so that a
/// packet for another connection, or one the receive window refuses, costs no cipher work.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ClearHeader {
    /// The flags byte as received (key phase bits included).
    pub flags: u8,
    /// Connection id of the receiver.
    pub conn_id: u16,
    /// Low 32 bits of the packet number.
    pub number: u32,
}

/// Parses the first [`CLEAR_HEADER_LEN`] bytes. Refuses short datagrams and handshake packets (flag bit 7).
pub fn parse_clear_header(data: &[u8]) -> Result<ClearHeader, ConnError> {
    let Some(bytes) = data.first_chunk::<CLEAR_HEADER_LEN>() else {
        return Err(ConnError::Truncated);
    };
    let [flags, c0, c1, n0, n1, n2, n3] = *bytes;
    if flags & 0x80 != 0 {
        return Err(ConnError::BadHeader);
    }
    Ok(ClearHeader { flags, conn_id: u16::from_le_bytes([c0, c1]), number: u32::from_le_bytes([n0, n1, n2, n3]) })
}

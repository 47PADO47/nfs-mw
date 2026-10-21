//! The 15-byte packet header and the frame type and length prefix (the layout tables are in the module doc of
//! [`super`]). Everything here works on caller-owned byte slices and never allocates.

use super::{ConnError, HEADER_LEN};

/// Flag bit 0: the acknowledgement block is valid (the sender has received at least one packet).
pub const FLAG_ACK_VALID: u8 = 0x01;

/// The fields of the packet header.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Header {
    /// Flags byte; only [`FLAG_ACK_VALID`] may be set.
    pub flags: u8,
    /// Connection id of the receiver.
    pub conn_id: u16,
    /// Number of this packet.
    pub number: u32,
    /// Largest packet number the sender has received from the peer.
    pub ack: u32,
    /// Bit `i` set: packet `ack - 1 - i` was received.
    pub ack_bits: u32,
}

/// Writes `header` into the first [`HEADER_LEN`] bytes of `out`.
pub fn write_header(out: &mut [u8], header: &Header) -> Result<(), ConnError> {
    let mut bytes = [0u8; HEADER_LEN];
    bytes[0] = header.flags;
    bytes[1..3].copy_from_slice(&header.conn_id.to_le_bytes());
    bytes[3..7].copy_from_slice(&header.number.to_le_bytes());
    bytes[7..11].copy_from_slice(&header.ack.to_le_bytes());
    bytes[11..15].copy_from_slice(&header.ack_bits.to_le_bytes());
    let Some(dst) = out.first_chunk_mut::<HEADER_LEN>() else {
        return Err(ConnError::BufferTooSmall);
    };
    *dst = bytes;
    Ok(())
}

/// Parses the header at the start of `data`. Refuses short datagrams and any flag bit other than [`FLAG_ACK_VALID`].
pub fn parse_header(data: &[u8]) -> Result<Header, ConnError> {
    let Some(bytes) = data.first_chunk::<HEADER_LEN>() else {
        return Err(ConnError::Truncated);
    };
    let [flags, c0, c1, n0, n1, n2, n3, a0, a1, a2, a3, b0, b1, b2, b3] = *bytes;
    // Bit 7 (handshake), bits 6..4 (reserved for keys) and bits 3..1 (format 0) must be zero, and so must bit 0 when
    // the ack block is ignored; only FLAG_ACK_VALID may be set.
    if flags & !FLAG_ACK_VALID != 0 {
        return Err(ConnError::BadHeader);
    }
    Ok(Header {
        flags,
        conn_id: u16::from_le_bytes([c0, c1]),
        number: u32::from_le_bytes([n0, n1, n2, n3]),
        ack: u32::from_le_bytes([a0, a1, a2, a3]),
        ack_bits: u32::from_le_bytes([b0, b1, b2, b3]),
    })
}

/// Bytes the canonical varint of `value` takes: 1 up to 127, 2 up to 16,383, 3 above.
pub fn varint_len(value: usize) -> usize {
    match value {
        0..=0x7F => 1,
        0x80..=0x3FFF => 2,
        _ => 3,
    }
}

/// Writes `value` as a canonical LEB128 varint at the start of `out` and returns the bytes written. `value` must be
/// below 2^21 (three bytes).
pub fn put_varint(out: &mut [u8], value: usize) -> Option<usize> {
    let len = varint_len(value);
    let dst = out.get_mut(..len)?;
    for (i, byte) in dst.iter_mut().enumerate() {
        let group = ((value >> (7 * i)) & 0x7F) as u8;
        let more = u8::from(i + 1 < len) << 7;
        *byte = group | more;
    }
    Some(len)
}

/// Reads a canonical varint from the start of `data`; returns the value and the bytes it used. A trailing zero group
/// (a longer encoding of a shorter value) and a fourth byte are refused.
pub fn read_varint(data: &[u8]) -> Result<(usize, usize), ConnError> {
    let mut value = 0usize;
    for i in 0..3 {
        let Some(&byte) = data.get(i) else {
            return Err(ConnError::BadFrame);
        };
        value |= usize::from(byte & 0x7F) << (7 * i);
        if byte & 0x80 == 0 {
            if i > 0 && byte == 0 {
                return Err(ConnError::BadFrame);
            }
            return Ok((value, i + 1));
        }
    }
    Err(ConnError::BadFrame)
}

/// Bytes one frame with a payload of `payload_len` bytes takes on the wire: type, length prefix and payload.
pub fn frame_size(payload_len: usize) -> usize {
    1 + varint_len(payload_len) + payload_len
}

/// Writes one frame at `at` in `out`: the type, the length of `parts` in total, then every part in order. Returns the
/// offset after the frame, or `None` if it does not fit in `out`.
pub fn write_frame(out: &mut [u8], at: usize, ftype: u8, parts: &[&[u8]]) -> Option<usize> {
    let payload_len: usize = parts.iter().map(|part| part.len()).sum();
    let end = at.checked_add(frame_size(payload_len))?;
    let dst = out.get_mut(at..end)?;
    let (head, body) = dst.split_first_mut()?;
    *head = ftype;
    let (len_bytes, mut rest) = body.split_at_mut_checked(varint_len(payload_len))?;
    put_varint(len_bytes, payload_len)?;
    for &part in parts {
        let (chunk, tail) = rest.split_at_mut_checked(part.len())?;
        chunk.copy_from_slice(part);
        rest = tail;
    }
    Some(end)
}

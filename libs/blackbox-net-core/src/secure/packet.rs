//! Handshake datagrams (protocol.md section 4). Every decoder is a pure function on `&[u8]`: the exact length first,
//! then the fields, then the padding; nothing is allocated, indexed out of range or trusted. Multi-byte integers are
//! little-endian, like the connection header.
//!
//! **Byte 0** of a handshake packet: bit 7 = 1, bits 6..4 = [`HsType`], bits 3..0 = packet format 0.
//!
//! | Packet | Layout after byte 0 | Size |
//! |---|---|---|
//! | `Hello` | `proto_major`, `format` ([`HELLO_FORMAT`]), `client_nonce` 16, `client_conn_id` u16, zeros | exactly 512 |
//! | `Challenge` | `server_conn_id` u16, `bucket` u8, `cookie` 32 | exactly 36 |
//! | `HelloCookie` | `proto_major`, `format`, `pattern`, `client_nonce` 16, `client_conn_id` u16, `server_conn_id` u16, `bucket`, `cookie` 32, `noise_len` u16, Noise message 1, zeros | exactly 512 |
//! | `NoiseMsg2` | `client_conn_id` u16, `len` u16, Noise message 2 | 5 + len |
//! | `NoiseMsg3` | `server_conn_id` u16, `len` u16, Noise message 3 | 5 + len |
//! | `Reject` | `reason` u8 (1 full, 2 protocol major unsupported, 3 shutting down) | exactly 2 |

use std::fmt;

use super::noise::Pattern;
use crate::conn::CLEAR_HEADER_LEN;

/// Bit 7 of byte 0: a handshake packet.
pub const HANDSHAKE_BIT: u8 = 0x80;
/// Size of `Hello` and `HelloCookie`: the padding that makes a reply never larger than its request.
pub const HELLO_LEN: usize = 512;
/// Size of `Challenge`.
pub const CHALLENGE_LEN: usize = 36;
/// Size of `Reject`.
pub const REJECT_LEN: usize = 2;
/// The `format` byte of `Hello` and `HelloCookie`.
pub const HELLO_FORMAT: u8 = 1;
/// Longest Noise message carried (`XX` message 2 is 96 bytes).
pub const MAX_NOISE_MSG: usize = 256;
/// Bytes of `HelloCookie` before Noise message 1.
const COOKIE_HEAD: usize = 59;
/// Bytes of `NoiseMsg2` and `NoiseMsg3` before the Noise message.
const NOISE_HEAD: usize = 5;

/// Why a handshake datagram was refused.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PacketError {
    /// The datagram has the wrong length for its type.
    BadLength,
    /// Byte 0 is not this packet type, or the format bits are not 0.
    BadType,
    /// The `format` byte or the pattern byte is unknown.
    BadVersion,
    /// Padding bytes are not zero.
    BadPadding,
    /// A length field is zero, too large, or runs past the datagram.
    BadField,
}

impl fmt::Display for PacketError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::BadLength => "wrong length",
            Self::BadType => "wrong packet type",
            Self::BadVersion => "unknown format",
            Self::BadPadding => "padding not zero",
            Self::BadField => "bad length field",
        })
    }
}

impl std::error::Error for PacketError {}

/// The handshake packet types (bits 6..4 of byte 0).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HsType {
    /// Client to server: the first contact.
    Hello = 0,
    /// Server to client: the cookie.
    Challenge = 1,
    /// Client to server: the cookie echoed, with Noise message 1.
    HelloCookie = 2,
    /// Server to client: Noise message 2.
    NoiseMsg2 = 3,
    /// Client to server: Noise message 3.
    NoiseMsg3 = 4,
    /// Server to client: a refusal, only to an address that presented a valid cookie.
    Reject = 5,
}

impl HsType {
    fn from_bits(bits: u8) -> Option<Self> {
        match bits {
            0 => Some(Self::Hello),
            1 => Some(Self::Challenge),
            2 => Some(Self::HelloCookie),
            3 => Some(Self::NoiseMsg2),
            4 => Some(Self::NoiseMsg3),
            5 => Some(Self::Reject),
            _ => None,
        }
    }

    /// Byte 0 of a packet of this type.
    pub fn byte0(self) -> u8 {
        HANDSHAKE_BIT | ((self as u8) << 4)
    }
}

/// What a datagram is, from its first byte and length, before anything is decoded.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PacketKind {
    /// A handshake packet of this type (still to be decoded).
    Handshake(HsType),
    /// A data packet: route it by its connection id to a [`crate::conn::Connection`].
    Data,
    /// Neither: drop it.
    Invalid,
}

/// Sorts a datagram by its first byte.
pub fn classify(datagram: &[u8]) -> PacketKind {
    let Some(&first) = datagram.first() else {
        return PacketKind::Invalid;
    };
    if first & HANDSHAKE_BIT == 0 {
        return match datagram.len() >= CLEAR_HEADER_LEN {
            true => PacketKind::Data,
            false => PacketKind::Invalid,
        };
    }
    if first & 0x0F != 0 {
        return PacketKind::Invalid;
    }
    match HsType::from_bits((first >> 4) & 0x07) {
        Some(kind) => PacketKind::Handshake(kind),
        None => PacketKind::Invalid,
    }
}

fn u16_at(bytes: &[u8], at: usize) -> Option<u16> {
    let pair = bytes.get(at..at + 2)?;
    Some(u16::from_le_bytes([*pair.first()?, *pair.get(1)?]))
}

fn array_at<const N: usize>(bytes: &[u8], at: usize) -> Option<[u8; N]> {
    bytes.get(at..at + N)?.try_into().ok()
}

fn check_type(bytes: &[u8], kind: HsType) -> Result<(), PacketError> {
    match bytes.first() {
        Some(&first) if first == kind.byte0() => Ok(()),
        _ => Err(PacketError::BadType),
    }
}

fn check_zero(bytes: &[u8], from: usize) -> Result<(), PacketError> {
    let tail = bytes.get(from..).ok_or(PacketError::BadLength)?;
    match tail.iter().all(|&b| b == 0) {
        true => Ok(()),
        false => Err(PacketError::BadPadding),
    }
}

/// `Hello`: the first contact.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Hello {
    /// The client's protocol major version.
    pub proto_major: u8,
    /// A fresh random value per attempt; bound into the cookie.
    pub client_nonce: [u8; 16],
    /// The connection id the client answers to.
    pub client_conn_id: u16,
}

impl Hello {
    /// The 512-byte datagram.
    pub fn encode(&self) -> [u8; HELLO_LEN] {
        let mut out = [0u8; HELLO_LEN];
        let head = [HsType::Hello.byte0(), self.proto_major, HELLO_FORMAT];
        out[..3].copy_from_slice(&head);
        out[3..19].copy_from_slice(&self.client_nonce);
        out[19..21].copy_from_slice(&self.client_conn_id.to_le_bytes());
        out
    }

    /// Decodes a `Hello`. The length must be exactly 512 and the padding zero.
    pub fn decode(bytes: &[u8]) -> Result<Self, PacketError> {
        if bytes.len() != HELLO_LEN {
            return Err(PacketError::BadLength);
        }
        check_type(bytes, HsType::Hello)?;
        let (Some(&proto_major), Some(&format)) = (bytes.get(1), bytes.get(2)) else {
            return Err(PacketError::BadLength);
        };
        if format != HELLO_FORMAT {
            return Err(PacketError::BadVersion);
        }
        let client_nonce = array_at::<16>(bytes, 3).ok_or(PacketError::BadLength)?;
        let client_conn_id = u16_at(bytes, 19).ok_or(PacketError::BadLength)?;
        check_zero(bytes, 21)?;
        Ok(Self { proto_major, client_nonce, client_conn_id })
    }
}

/// `Challenge`: the server's stateless cookie.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Challenge {
    /// The connection id the server will answer to.
    pub server_conn_id: u16,
    /// Low 8 bits of the cookie's time bucket.
    pub bucket: u8,
    /// The cookie.
    pub cookie: [u8; 32],
}

impl Challenge {
    /// The 36-byte datagram.
    pub fn encode(&self) -> [u8; CHALLENGE_LEN] {
        let mut out = [0u8; CHALLENGE_LEN];
        out[0] = HsType::Challenge.byte0();
        out[1..3].copy_from_slice(&self.server_conn_id.to_le_bytes());
        out[3] = self.bucket;
        out[4..36].copy_from_slice(&self.cookie);
        out
    }

    /// Decodes a `Challenge` of exactly 36 bytes.
    pub fn decode(bytes: &[u8]) -> Result<Self, PacketError> {
        if bytes.len() != CHALLENGE_LEN {
            return Err(PacketError::BadLength);
        }
        check_type(bytes, HsType::Challenge)?;
        let server_conn_id = u16_at(bytes, 1).ok_or(PacketError::BadLength)?;
        let bucket = *bytes.get(3).ok_or(PacketError::BadLength)?;
        let cookie = array_at::<32>(bytes, 4).ok_or(PacketError::BadLength)?;
        Ok(Self { server_conn_id, bucket, cookie })
    }
}

/// `HelloCookie`: the cookie echoed, with Noise message 1.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HelloCookie<'a> {
    /// The client's protocol major version.
    pub proto_major: u8,
    /// The Noise pattern the client runs.
    pub pattern: Pattern,
    /// The nonce of the `Hello` this answers.
    pub client_nonce: [u8; 16],
    /// The connection id the client answers to.
    pub client_conn_id: u16,
    /// The connection id from the `Challenge`.
    pub server_conn_id: u16,
    /// The bucket from the `Challenge`.
    pub bucket: u8,
    /// The cookie from the `Challenge`.
    pub cookie: [u8; 32],
    /// Noise message 1.
    pub noise: &'a [u8],
}

impl HelloCookie<'_> {
    /// The 512-byte datagram; `BadField` if the Noise message does not fit.
    pub fn encode(&self) -> Result<[u8; HELLO_LEN], PacketError> {
        let len = self.noise.len();
        if len == 0 || len > MAX_NOISE_MSG {
            return Err(PacketError::BadField);
        }
        let mut out = [0u8; HELLO_LEN];
        out[..4].copy_from_slice(&[HsType::HelloCookie.byte0(), self.proto_major, HELLO_FORMAT, self.pattern as u8]);
        out[4..20].copy_from_slice(&self.client_nonce);
        out[20..22].copy_from_slice(&self.client_conn_id.to_le_bytes());
        out[22..24].copy_from_slice(&self.server_conn_id.to_le_bytes());
        out[24] = self.bucket;
        out[25..57].copy_from_slice(&self.cookie);
        out[57..59].copy_from_slice(&(len as u16).to_le_bytes());
        out[COOKIE_HEAD..COOKIE_HEAD + len].copy_from_slice(self.noise);
        Ok(out)
    }
}

impl<'a> HelloCookie<'a> {
    /// Decodes a `HelloCookie` of exactly 512 bytes with zero padding after the Noise message.
    pub fn decode(bytes: &'a [u8]) -> Result<Self, PacketError> {
        if bytes.len() != HELLO_LEN {
            return Err(PacketError::BadLength);
        }
        check_type(bytes, HsType::HelloCookie)?;
        let head = array_at::<4>(bytes, 0).ok_or(PacketError::BadLength)?;
        let [_, proto_major, format, pattern] = head;
        if format != HELLO_FORMAT {
            return Err(PacketError::BadVersion);
        }
        let pattern = Pattern::from_u8(pattern).ok_or(PacketError::BadVersion)?;
        let client_nonce = array_at::<16>(bytes, 4).ok_or(PacketError::BadLength)?;
        let client_conn_id = u16_at(bytes, 20).ok_or(PacketError::BadLength)?;
        let server_conn_id = u16_at(bytes, 22).ok_or(PacketError::BadLength)?;
        let bucket = *bytes.get(24).ok_or(PacketError::BadLength)?;
        let cookie = array_at::<32>(bytes, 25).ok_or(PacketError::BadLength)?;
        let len = usize::from(u16_at(bytes, 57).ok_or(PacketError::BadLength)?);
        if len == 0 || len > MAX_NOISE_MSG {
            return Err(PacketError::BadField);
        }
        let noise = bytes.get(COOKIE_HEAD..COOKIE_HEAD + len).ok_or(PacketError::BadField)?;
        check_zero(bytes, COOKIE_HEAD + len)?;
        Ok(Self { proto_major, pattern, client_nonce, client_conn_id, server_conn_id, bucket, cookie, noise })
    }
}

/// `NoiseMsg2` or `NoiseMsg3`: a connection id and one Noise message.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NoiseMsg<'a> {
    /// `NoiseMsg2`: the client's connection id. `NoiseMsg3`: the server's.
    pub conn_id: u16,
    /// The Noise message.
    pub msg: &'a [u8],
}

impl<'a> NoiseMsg<'a> {
    /// Writes the packet of type `kind` into `out`; returns its length, or `BadField` if the message is empty, too long
    /// or does not fit.
    pub fn encode(&self, kind: HsType, out: &mut [u8]) -> Result<usize, PacketError> {
        let len = self.msg.len();
        if len == 0 || len > MAX_NOISE_MSG {
            return Err(PacketError::BadField);
        }
        let dst = out.get_mut(..NOISE_HEAD + len).ok_or(PacketError::BadField)?;
        let (head, body) = dst.split_at_mut(NOISE_HEAD);
        let id = self.conn_id.to_le_bytes();
        let size = (len as u16).to_le_bytes();
        head.copy_from_slice(&[kind.byte0(), id[0], id[1], size[0], size[1]]);
        body.copy_from_slice(self.msg);
        Ok(NOISE_HEAD + len)
    }

    /// Decodes a packet of type `kind` (`NoiseMsg2` or `NoiseMsg3`). The datagram must end with the message.
    pub fn decode(bytes: &'a [u8], kind: HsType) -> Result<Self, PacketError> {
        check_type(bytes, kind)?;
        let conn_id = u16_at(bytes, 1).ok_or(PacketError::BadLength)?;
        let len = usize::from(u16_at(bytes, 3).ok_or(PacketError::BadLength)?);
        if len == 0 || len > MAX_NOISE_MSG {
            return Err(PacketError::BadField);
        }
        if bytes.len() != NOISE_HEAD + len {
            return Err(PacketError::BadLength);
        }
        let msg = bytes.get(NOISE_HEAD..).ok_or(PacketError::BadLength)?;
        Ok(Self { conn_id, msg })
    }
}

/// Why a server refused a client that presented a valid cookie.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RejectReason {
    /// The session is full.
    Full = 1,
    /// The client's protocol major version is not supported.
    Version = 2,
    /// The server is shutting down.
    ShuttingDown = 3,
}

impl RejectReason {
    /// The 2-byte `Reject` datagram.
    pub fn encode(self) -> [u8; REJECT_LEN] {
        [HsType::Reject.byte0(), self as u8]
    }

    /// Decodes a `Reject` of exactly 2 bytes with a known reason.
    pub fn decode(bytes: &[u8]) -> Result<Self, PacketError> {
        if bytes.len() != REJECT_LEN {
            return Err(PacketError::BadLength);
        }
        check_type(bytes, HsType::Reject)?;
        match bytes.get(1) {
            Some(1) => Ok(Self::Full),
            Some(2) => Ok(Self::Version),
            Some(3) => Ok(Self::ShuttingDown),
            _ => Err(PacketError::BadField),
        }
    }
}

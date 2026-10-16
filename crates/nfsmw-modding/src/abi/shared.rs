//! Plain records that several `nfsmw.*` tables use: an inline name and a 3-vector. Both are copied by value, so no pointer
//! the host keeps outlives a call (ADR 0005 section 4).

use blackbox_modding::abi::MAX_NAME_BYTES;

/// A name copied inline: `len` bytes of UTF-8 at the start of `bytes`. `len` is at most `MAX_NAME_BYTES` (128).
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BbName {
    /// Bytes in use.
    pub len: u32,
    /// The bytes. The rest is zero.
    pub bytes: [u8; MAX_NAME_BYTES],
}

impl BbName {
    /// The empty name.
    pub const EMPTY: BbName = BbName { len: 0, bytes: [0; MAX_NAME_BYTES] };

    /// A name holding `text`. `None` when `text` is longer than `MAX_NAME_BYTES`.
    pub fn new(text: &str) -> Option<Self> {
        if text.len() > MAX_NAME_BYTES {
            return None;
        }
        let mut bytes = [0_u8; MAX_NAME_BYTES];
        bytes[..text.len()].copy_from_slice(text.as_bytes());
        Some(BbName { len: text.len() as u32, bytes })
    }

    /// The text. `None` when `len` is past the array or the bytes are not UTF-8, which a host never writes.
    pub fn text(&self) -> Option<&str> {
        let len = usize::try_from(self.len).ok()?;
        core::str::from_utf8(self.bytes.get(..len)?).ok()
    }
}

/// A point in render space (x forward, y left, z up), in metres.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BbVec3 {
    /// Forward.
    pub x: f32,
    /// Left.
    pub y: f32,
    /// Up.
    pub z: f32,
}

impl BbVec3 {
    /// From `[x, y, z]`.
    pub const fn from_array(v: [f32; 3]) -> Self {
        BbVec3 { x: v[0], y: v[1], z: v[2] }
    }

    /// As `[x, y, z]`.
    pub const fn to_array(self) -> [f32; 3] {
        [self.x, self.y, self.z]
    }
}

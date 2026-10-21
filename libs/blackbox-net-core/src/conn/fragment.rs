//! Splitting a message into fragments and checking what a receiver is given. Every limit is checked before a buffer grows.

use super::{ConnError, MAX_FRAGMENT, MAX_FRAGMENTS, MAX_MESSAGE};

/// Message bytes in the first fragment of a multi-fragment message: the fragment holds [`MAX_FRAGMENT`] bytes, two of
/// them the total length.
pub const FIRST_CHUNK: usize = MAX_FRAGMENT - 2;

/// Fragments a message of `len` bytes takes, or `None` when it is above [`MAX_MESSAGE`] or needs more than
/// [`MAX_FRAGMENTS`] fragments. A message of at most [`MAX_FRAGMENT`] bytes is one fragment without a length prefix.
pub fn fragment_count(len: usize) -> Option<usize> {
    if len > MAX_MESSAGE {
        return None;
    }
    if len <= MAX_FRAGMENT {
        return Some(1);
    }
    let count = 1 + (len - FIRST_CHUNK).div_ceil(MAX_FRAGMENT);
    (count <= MAX_FRAGMENTS).then_some(count)
}

/// The message bytes fragment `index` of a message of `len` bytes carries, as a `start..end` range.
pub fn fragment_span(len: usize, index: usize) -> (usize, usize) {
    if len <= MAX_FRAGMENT {
        return (0, len);
    }
    let (start, size) = match index {
        0 => (0, FIRST_CHUNK),
        _ => (FIRST_CHUNK + (index - 1) * MAX_FRAGMENT, MAX_FRAGMENT),
    };
    (start.min(len), (start + size).min(len))
}

/// Reads and checks the total message length at the start of a first fragment's data. The total must be above
/// [`MAX_FRAGMENT`] (a shorter message is never split) and at most [`MAX_MESSAGE`].
pub fn declared_total(data: &[u8]) -> Result<usize, ConnError> {
    let Some(prefix) = data.first_chunk::<2>() else {
        return Err(ConnError::BadFrame);
    };
    let total = usize::from(u16::from_le_bytes(*prefix));
    if total <= MAX_FRAGMENT || total > MAX_MESSAGE {
        return Err(ConnError::BadFrame);
    }
    Ok(total)
}

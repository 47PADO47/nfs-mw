//! Detecting and removing a wrapper.

use std::borrow::Cow;

use crate::{Error, HEADER_LEN, HUFF_MAGIC, Header, JDLZ_MAGIC, RAWW_MAGIC, Result, huff_decompress, jdlz_decompress};

/// Which wrapper (if any) a buffer starts with.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Wrapper {
    Jdlz,
    Huff,
    Raww,
    None,
}

impl Wrapper {
    pub fn detect(data: &[u8]) -> Self {
        match data.first_chunk::<4>().map(|m| u32::from_le_bytes(*m)) {
            Some(JDLZ_MAGIC) => Self::Jdlz,
            Some(HUFF_MAGIC) => Self::Huff,
            Some(RAWW_MAGIC) => Self::Raww,
            _ => Self::None,
        }
    }
}

/// Decompress any supported wrapper. Returns the input unchanged (borrowed) when there is none.
pub fn unwrap(data: &[u8]) -> Result<Cow<'_, [u8]>> {
    match Wrapper::detect(data) {
        Wrapper::Jdlz => jdlz_decompress(data).map(Cow::Owned),
        Wrapper::Huff => huff_decompress(data).map(Cow::Owned),
        Wrapper::Raww => {
            let h = Header::parse(data, "RAWW")?;
            let end = HEADER_LEN + h.decompressed_size as usize;
            data.get(HEADER_LEN..end).map(Cow::Borrowed).ok_or(Error::Truncated("RAWW"))
        }
        Wrapper::None => Ok(Cow::Borrowed(data)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detect_and_unwrap_raww() {
        let mut blob = b"RAWW".to_vec();
        blob.extend_from_slice(&[1, 0x10, 0, 0]);
        blob.extend_from_slice(&3u32.to_le_bytes());
        blob.extend_from_slice(&19u32.to_le_bytes());
        blob.extend_from_slice(b"abc");
        assert_eq!(Wrapper::detect(&blob), Wrapper::Raww);
        assert_eq!(&*unwrap(&blob).unwrap(), b"abc");
    }

    #[test]
    fn no_wrapper_is_borrowed() {
        let data = [0u8; 12];
        assert!(matches!(unwrap(&data).unwrap(), Cow::Borrowed(_)));
    }
}

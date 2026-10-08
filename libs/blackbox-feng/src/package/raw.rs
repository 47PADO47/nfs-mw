//! The two levels of syntax inside a package: chunks (`u32 id, u32 size, data`, bit 31 = has children) and,
//! inside object, script and response chunks, tags (`u8 a, u8 b, u16 size, data`). Neither is padded.

use crate::error::{Error, Result};

/// One chunk of a package.
#[derive(Clone, Copy, Debug)]
pub struct Chunk<'a> {
    /// The four id characters with the container bit cleared.
    pub id: [u8; 4],
    pub nested: bool,
    pub data: &'a [u8],
}

impl Chunk<'_> {
    pub fn is(&self, id: &[u8; 4]) -> bool {
        &self.id == id
    }
}

/// Splits `data` into chunks. Stops with an error if one runs past the end.
pub fn chunks<'a>(data: &'a [u8], what: &'static str) -> Result<Vec<Chunk<'a>>> {
    let mut out = Vec::new();
    let mut pos = 0;
    while pos < data.len() {
        let head = data.get(pos..pos + 8).ok_or(Error::Truncated(what))?;
        let size = u32::from_le_bytes([head[4], head[5], head[6], head[7]]) as usize;
        let body = data.get(pos + 8..pos + 8 + size).ok_or(Error::Truncated(what))?;
        out.push(Chunk { id: [head[0], head[1], head[2], head[3] & 0x7F], nested: head[3] & 0x80 != 0, data: body });
        pos += 8 + size;
    }
    Ok(out)
}

/// One property tag.
#[derive(Clone, Copy, Debug)]
pub struct Tag<'a> {
    pub id: [u8; 2],
    pub data: &'a [u8],
}

impl<'a> Tag<'a> {
    pub fn is(&self, id: &[u8; 2]) -> bool {
        &self.id == id
    }

    pub fn u32(&self) -> Option<u32> {
        Some(u32::from_le_bytes(self.data.get(..4)?.try_into().ok()?))
    }

    pub fn i32(&self) -> Option<i32> {
        self.u32().map(|v| v as i32)
    }
}

/// Splits `data` into tags.
pub fn tags<'a>(data: &'a [u8], what: &'static str) -> Result<Vec<Tag<'a>>> {
    let mut out = Vec::new();
    let mut pos = 0;
    while pos < data.len() {
        let head = data.get(pos..pos + 4).ok_or(Error::Truncated(what))?;
        let size = u16::from_le_bytes([head[2], head[3]]) as usize;
        let body = data.get(pos + 4..pos + 4 + size).ok_or(Error::Truncated(what))?;
        out.push(Tag { id: [head[0], head[1]], data: body });
        pos += 4 + size;
    }
    Ok(out)
}

/// All complete little-endian words of `data`.
pub fn words(data: &[u8]) -> impl Iterator<Item = u32> + '_ {
    data.as_chunks::<4>().0.iter().map(|w| u32::from_le_bytes(*w))
}

/// All complete little-endian 16-bit units of `data`.
pub fn units(data: &[u8]) -> impl Iterator<Item = u16> + '_ {
    data.as_chunks::<2>().0.iter().map(|w| u16::from_le_bytes(*w))
}

pub fn read_u32(data: &[u8], offset: usize) -> Option<u32> {
    Some(u32::from_le_bytes(data.get(offset..offset + 4)?.try_into().ok()?))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chunks_and_tags_split() {
        let mut c = Vec::new();
        c.extend_from_slice(b"ObjL");
        c.extend_from_slice(&3u32.to_le_bytes());
        c.extend_from_slice(&[1, 2, 3]);
        let mut id = *b"FEng";
        id[3] |= 0x80;
        c.extend_from_slice(&id);
        c.extend_from_slice(&0u32.to_le_bytes());
        let ch = chunks(&c, "test").unwrap();
        assert_eq!(ch.len(), 2);
        assert!(ch[0].is(b"ObjL") && !ch[0].nested && ch[0].data == [1, 2, 3]);
        assert!(ch[1].is(b"FEng") && ch[1].nested);

        let t = [b'O', b't', 4, 0, 5, 0, 0, 0, b'P', b'A', 0, 0];
        let tg = tags(&t, "test").unwrap();
        assert_eq!(tg.len(), 2);
        assert_eq!(tg[0].u32(), Some(5));
        assert!(tg[1].is(b"PA") && tg[1].data.is_empty());
    }

    #[test]
    fn truncation_is_an_error() {
        assert!(chunks(&[1, 2, 3], "x").is_err());
        let mut c = b"ObjL".to_vec();
        c.extend_from_slice(&9u32.to_le_bytes());
        assert!(chunks(&c, "x").is_err());
        assert!(tags(&[b'O', b't', 8, 0, 1], "x").is_err());
    }
}

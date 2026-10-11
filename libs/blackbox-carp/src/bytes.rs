use crate::{Error, Result};

#[derive(Debug, Clone, Copy)]
pub(crate) struct Reader<'a> {
    data: &'a [u8],
    what: &'static str,
}

impl<'a> Reader<'a> {
    pub fn new(data: &'a [u8], what: &'static str) -> Self {
        Self { data, what }
    }

    pub fn slice(&self, offset: usize, len: usize) -> Result<&'a [u8]> {
        offset.checked_add(len).and_then(|end| self.data.get(offset..end)).ok_or(Error::Truncated {
            what: self.what,
            offset,
            need: len,
            len: self.data.len(),
        })
    }

    pub fn u32(&self, offset: usize) -> Result<u32> {
        Ok(u32::from_le_bytes(self.slice(offset, 4)?.try_into().unwrap()))
    }
}

pub(crate) fn malformed(what: &'static str, detail: impl Into<String>) -> Error {
    Error::Malformed { what, detail: detail.into() }
}

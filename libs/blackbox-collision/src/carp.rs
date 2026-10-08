//! The `UGroup` tree inside a CARP blob: tagged groups holding tagged data records.
//! Spec: `docs/formats/collision.md` ("The CARP blob").
//!
//! A blob starts with a 16-byte group header (the root, tag `CARP`). A group's child groups and
//! data records form one array (child groups first) that starts `offset * 16` bytes after the
//! group header. A data record's bytes start `offset` bytes after its own header. Only the
//! embedded form is supported; the files of this game contain nothing else.

use crate::bytes::{Reader, malformed};
use crate::{Error, Result};

/// Size of a group header and of a data record header.
pub const HEADER_LEN: usize = 16;

/// Builds a tag from its type (two characters) and index (two characters or a number), the way the
/// engine's `MAKE_UDATA_TYPE(type) | index` does.
pub const fn tag(kind: [u8; 2], index: u16) -> u32 {
    ((kind[0] as u32) << 24) | ((kind[1] as u32) << 16) | index as u32
}

/// A tag made of four characters, e.g. `Arti`.
pub const fn tag4(name: &[u8; 4]) -> u32 {
    u32::from_be_bytes(*name)
}

/// A whole CARP blob (without the 16-byte chunk header in front of it).
#[derive(Debug, Clone, Copy)]
pub struct Carp<'a> {
    reader: Reader<'a>,
}

impl<'a> Carp<'a> {
    pub fn new(blob: &'a [u8]) -> Self {
        Self { reader: Reader::new(blob, "CARP blob") }
    }

    /// The root group (tag `CARP`).
    pub fn root(&self) -> Result<Group<'a>> {
        Group::at(self.reader, 0)
    }
}

/// One `UGroup`.
#[derive(Debug, Clone, Copy)]
pub struct Group<'a> {
    reader: Reader<'a>,
    pub tag: u32,
    group_count: usize,
    data_count: usize,
    array: usize,
}

impl<'a> Group<'a> {
    fn at(reader: Reader<'a>, offset: usize) -> Result<Self> {
        let tag = reader.u32(offset)?;
        let flags = reader.u32(offset + 4)?;
        if flags & 0b10 == 0 {
            return Err(malformed("CARP group", format!("group at 0x{offset:X} is not embedded (flags 0x{flags:X})")));
        }
        let group_count = (flags >> 5) as usize;
        let data_count = reader.u32(offset + 8)? as usize;
        let units = reader.u32(offset + 12)? as usize;
        let array = units
            .checked_mul(HEADER_LEN)
            .and_then(|o| o.checked_add(offset))
            .ok_or_else(|| malformed("CARP group", format!("group at 0x{offset:X}: offset {units} overflows")))?;
        // Fail early when the whole array is outside the blob.
        reader.slice(array, (group_count + data_count) * HEADER_LEN)?;
        Ok(Self { reader, tag, group_count, data_count, array })
    }

    pub fn group_count(&self) -> usize {
        self.group_count
    }

    pub fn record_count(&self) -> usize {
        self.data_count
    }

    /// The child groups, in file order.
    pub fn groups(&self) -> impl Iterator<Item = Result<Group<'a>>> + '_ {
        (0..self.group_count).map(|i| Group::at(self.reader, self.array + i * HEADER_LEN))
    }

    /// The first child group with `tag`.
    pub fn group(&self, tag: u32) -> Result<Option<Group<'a>>> {
        for g in self.groups() {
            let g = g?;
            if g.tag == tag {
                return Ok(Some(g));
            }
        }
        Ok(None)
    }

    /// The data records, in file order.
    pub fn records(&self) -> impl Iterator<Item = Result<Record<'a>>> + '_ {
        let first = self.array + self.group_count * HEADER_LEN;
        (0..self.data_count).map(move |i| Record::at(self.reader, first + i * HEADER_LEN))
    }

    /// The first record with `tag`.
    pub fn record(&self, tag: u32) -> Result<Option<Record<'a>>> {
        for r in self.records() {
            let r = r?;
            if r.tag == tag {
                return Ok(Some(r));
            }
        }
        Ok(None)
    }

    /// Like [`Group::record`], but a missing record is an error named `what`.
    pub fn require(&self, tag: u32, what: &'static str) -> Result<Record<'a>> {
        self.record(tag)?.ok_or(Error::Missing(what))
    }
}

/// One `UData` record: a tagged run of bytes.
#[derive(Debug, Clone, Copy)]
pub struct Record<'a> {
    pub tag: u32,
    /// Number of elements the record holds (0 for records that are just a blob).
    pub count: u32,
    /// The record's bytes (`size` of them).
    pub data: &'a [u8],
}

impl<'a> Record<'a> {
    fn at(reader: Reader<'a>, offset: usize) -> Result<Self> {
        let tag = reader.u32(offset)?;
        let flags = reader.u32(offset + 4)?;
        let count = reader.u32(offset + 8)?;
        let rel = reader.u32(offset + 12)? as usize;
        if flags & 0b10 == 0 {
            return Err(malformed(
                "CARP record",
                format!("record at 0x{offset:X} is not embedded (flags 0x{flags:X})"),
            ));
        }
        let size = (flags >> 8) as usize;
        let data = reader.slice(offset + rel, size)?;
        Ok(Self { tag, count, data })
    }

    /// The bytes of one element when the record is an array of `stride`-byte elements.
    pub fn elements(&self, stride: usize, what: &'static str) -> Result<impl Iterator<Item = &'a [u8]> + use<'a>> {
        let total = (self.count as usize).checked_mul(stride);
        if total != Some(self.data.len()) {
            return Err(malformed(what, format!("{} bytes for {} elements of {stride}", self.data.len(), self.count)));
        }
        Ok(self.data.chunks_exact(stride))
    }
}

use std::io::Read;

use flate2::read::ZlibDecoder;

use crate::entry::{Entry, Import};
use crate::error::{Error, Result};
use crate::header::{BLOCKS, ENTRY_SIZE, FLAG_HAS_DEBUG_DATA, HEADER_SIZE, Header, u32_at, u64_at};

/// A `bnd2` bundle held in memory: the file's bytes and the parsed resource table.
#[derive(Debug, Clone)]
pub struct Bundle<'a> {
    data: &'a [u8],
    header: Header,
    entries: Vec<Entry>,
}

/// One resource with its blocks inflated.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Resource {
    pub entry: Entry,
    /// Main-memory block (the resource's header and small data) and the others; an unused block is empty.
    pub blocks: [Vec<u8>; BLOCKS],
    pub imports: Vec<Import>,
}

impl<'a> Bundle<'a> {
    pub fn parse(data: &'a [u8]) -> Result<Self> {
        let header = Header::parse(data)?;
        let start = header.entries_offset as usize;
        let end = start.saturating_add(header.resource_count as usize * ENTRY_SIZE);
        if start < HEADER_SIZE || end > data.len() {
            return Err(Error::Truncated { what: "resource table", start, end, len: data.len() });
        }
        let table = &data[start..end];
        let entries = (0..header.resource_count as usize).map(|index| Entry::parse(table, index)).collect();
        Ok(Self { data, header, entries })
    }

    pub fn header(&self) -> &Header {
        &self.header
    }

    pub fn entries(&self) -> &[Entry] {
        &self.entries
    }

    /// The position of the resource with id `id`.
    pub fn find(&self, id: u64) -> Option<usize> {
        self.entries.iter().position(|entry| entry.id == id)
    }

    /// The resource at `index`, inflated, with its import table.
    pub fn resource(&self, index: usize) -> Result<Resource> {
        let entry = self.entries[index];
        let mut blocks: [Vec<u8>; BLOCKS] = Default::default();
        for (block, slot) in blocks.iter_mut().enumerate() {
            *slot = self.read_block(index, &entry, block)?;
        }
        let imports = read_imports(index, &entry, &blocks[0])?;
        Ok(Resource { entry, blocks, imports })
    }

    /// The resource with id `id`.
    pub fn resource_by_id(&self, id: u64) -> Result<Resource> {
        let index = self.find(id).ok_or(Error::NoSuchResource { id })?;
        self.resource(index)
    }

    /// The `ResourceStringTable` XML, when the bundle carries one.
    pub fn debug_xml(&self) -> Option<String> {
        if self.header.flags & FLAG_HAS_DEBUG_DATA == 0 {
            return None;
        }
        let raw = self.data.get(self.header.debug_offset as usize..)?;
        let text = match raw.first() {
            Some(&0x78) => inflate(raw).ok()?,
            _ => raw.to_vec(),
        };
        let end = text.iter().position(|&byte| byte == 0).unwrap_or(text.len());
        String::from_utf8(text[..end].to_vec()).ok()
    }

    fn read_block(&self, index: usize, entry: &Entry, block: usize) -> Result<Vec<u8>> {
        let expected = entry.block_size(block);
        if expected == 0 {
            return Ok(Vec::new());
        }
        let start = (self.header.block_offsets[block] as usize).saturating_add(entry.disk_offset[block] as usize);
        let disk = (entry.disk_size[block] & 0x0FFF_FFFF) as usize;
        let end = start.saturating_add(disk);
        let Some(raw) = self.data.get(start..end) else {
            return Err(Error::Truncated { what: "resource block", start, end, len: self.data.len() });
        };
        if !self.header.compressed() {
            return Ok(raw.to_vec());
        }
        let bytes = inflate(raw).map_err(|detail| Error::Inflate { index, block, detail })?;
        if bytes.len() != expected {
            return Err(Error::BlockSize { index, block, expected, actual: bytes.len() });
        }
        Ok(bytes)
    }
}

fn inflate(raw: &[u8]) -> std::result::Result<Vec<u8>, String> {
    let mut out = Vec::new();
    ZlibDecoder::new(raw).read_to_end(&mut out).map_err(|error| error.to_string())?;
    Ok(out)
}

fn read_imports(index: usize, entry: &Entry, main: &[u8]) -> Result<Vec<Import>> {
    let count = entry.import_count as usize;
    if count == 0 {
        return Ok(Vec::new());
    }
    let offset = entry.import_offset as usize;
    if offset.saturating_add(count * 16) > main.len() {
        return Err(Error::ImportTable { index, offset, count, len: main.len() });
    }
    Ok((0..count)
        .map(|n| Import { id: u64_at(main, offset + 16 * n), offset: u32_at(main, offset + 16 * n + 8) })
        .collect())
}

//! One vault: the `.vlt` structure blob and the `.bin` payload blob, with its chunks parsed and
//! its pointer fix-ups indexed.

mod chunks;
mod exports;
mod pointers;

use std::fmt;

pub use chunks::Dependency;
pub use exports::Export;
use pointers::Pointers;

use crate::bytes;
use crate::layout::{self, Layout};
use crate::{Error, Result};

/// Which of a vault's two blobs a location is in.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Stream {
    Vlt,
    Bin,
}

impl fmt::Display for Stream {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Vlt => "vlt",
            Self::Bin => "bin",
        })
    }
}

/// A byte offset inside one of a vault's blobs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Location {
    pub stream: Stream,
    pub offset: usize,
}

impl Location {
    pub fn vlt(offset: usize) -> Self {
        Self { stream: Stream::Vlt, offset }
    }

    pub fn bin(offset: usize) -> Self {
        Self { stream: Stream::Bin, offset }
    }

    pub fn advance(self, delta: usize) -> Self {
        Self { offset: self.offset + delta, ..self }
    }
}

impl fmt::Display for Location {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}+0x{:X}", self.stream, self.offset)
    }
}

#[derive(Debug, Clone)]
pub struct Vault {
    name: String,
    vlt: Vec<u8>,
    bin: Vec<u8>,
    layout: &'static Layout,
    dependencies: Vec<Dependency>,
    strings: Vec<String>,
    exports: Vec<Export>,
    pointers: Pointers,
}

impl Vault {
    /// Parses a vault's chunks: `DepN`, `ExpN` and `PtrN` in the `.vlt`, `StrE` at the start of
    /// the `.bin`. The layout is detected from the export table.
    pub fn parse(name: &str, vlt: &[u8], bin: &[u8]) -> Result<Self> {
        let err = |detail: String| Error::Vault { vault: name.to_owned(), detail };
        let mut dependencies = Vec::new();
        let (mut export_table, mut pointer_table) = (None, None);
        for chunk in chunks::chunks(vlt) {
            let chunk = chunk.map_err(err)?;
            match chunk.id {
                chunks::DEP_N => dependencies = chunks::read_dependencies(chunk.payload).map_err(err)?,
                chunks::EXP_N => export_table = Some(chunk.payload),
                chunks::PTR_N => pointer_table = Some(chunk.payload),
                _ => {}
            }
        }
        let export_table = export_table.ok_or_else(|| err("no ExpN (export table) chunk".into()))?;
        let layout = layout::detect(export_table).ok_or_else(|| Error::UnsupportedLayout {
            vault: name.to_owned(),
            detail: format!("the {}-byte export table matches no known export entry size", export_table.len()),
        })?;
        let exports = exports::read_exports(export_table, &layout.export);
        let pointers = match pointer_table {
            Some(table) => pointers::read_pointers(table, &dependencies).map_err(err)?,
            None => Pointers::default(),
        };
        Ok(Self {
            name: name.to_owned(),
            vlt: vlt.to_vec(),
            bin: bin.to_vec(),
            layout,
            dependencies,
            strings: chunks::read_strings(bin),
            exports,
            pointers,
        })
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn layout(&self) -> &'static Layout {
        self.layout
    }

    /// `DepN`: the blobs pointers can be relative to (`<vault>.vlt`, `<vault>.bin`).
    pub fn dependencies(&self) -> &[Dependency] {
        &self.dependencies
    }

    /// `StrE`: the string table at the start of the `.bin`.
    pub fn strings(&self) -> &[String] {
        &self.strings
    }

    pub fn exports(&self) -> &[Export] {
        &self.exports
    }

    /// Number of `PtrN` fix-ups (both blobs).
    pub fn pointer_count(&self) -> usize {
        self.pointers.len()
    }

    pub fn data(&self, stream: Stream) -> &[u8] {
        match stream {
            Stream::Vlt => &self.vlt,
            Stream::Bin => &self.bin,
        }
    }

    pub(crate) fn error(&self, detail: impl Into<String>) -> Error {
        Error::Vault { vault: self.name.clone(), detail: detail.into() }
    }

    pub(crate) fn bytes(&self, at: Location, len: usize) -> Result<&[u8]> {
        bytes::slice(self.data(at.stream), at.offset, len)
            .ok_or_else(|| self.error(format!("{len} bytes at {at} are out of bounds")))
    }

    pub(crate) fn array<const N: usize>(&self, at: Location) -> Result<[u8; N]> {
        Ok(self.bytes(at, N)?.try_into().expect("slice of N bytes"))
    }

    pub(crate) fn u8(&self, at: Location) -> Result<u8> {
        Ok(self.array::<1>(at)?[0])
    }

    pub(crate) fn u16(&self, at: Location) -> Result<u16> {
        self.array(at).map(u16::from_le_bytes)
    }

    pub(crate) fn u32(&self, at: Location) -> Result<u32> {
        self.array(at).map(u32::from_le_bytes)
    }

    /// Follows the pointer stored at `at`. `None` is a null pointer: no fix-up (or a `PtrNull`
    /// one) and a stored value of 0. A non-zero value without a fix-up is an error, because it
    /// cannot be told which blob it points into.
    pub(crate) fn pointer(&self, at: Location) -> Result<Option<Location>> {
        if let Some(target) = self.pointers.get(at) {
            return Ok(target);
        }
        match self.u32(at)? {
            0 => Ok(None),
            raw => Err(self.error(format!("pointer at {at} holds 0x{raw:X} but has no PtrN fix-up"))),
        }
    }

    /// Like [`Self::pointer`], but null is an error.
    pub(crate) fn required_pointer(&self, at: Location, what: &str) -> Result<Location> {
        self.pointer(at)?.ok_or_else(|| self.error(format!("{what} pointer at {at} is null")))
    }

    /// The bytes of the NUL-terminated string at `at`, without the NUL.
    pub(crate) fn cstr_bytes(&self, at: Location) -> Result<&[u8]> {
        bytes::cstr_at(self.data(at.stream), at.offset)
            .ok_or_else(|| self.error(format!("unterminated string at {at}")))
    }

    /// The NUL-terminated string at `at` (bytes outside UTF-8 are replaced).
    pub(crate) fn cstr(&self, at: Location) -> Result<String> {
        self.cstr_bytes(at).map(|s| String::from_utf8_lossy(s).into_owned())
    }
}

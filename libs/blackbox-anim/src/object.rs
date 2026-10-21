//! An EAGL object: the ELF32 relocatable object that holds a skeleton or an animation bank.
//!
//! [`Object::read`] takes the chunk payload (or the bare ELF), checks the container, applies the `R_MIPS_32`
//! relocations to `.data` and records which words were relocated. After that every pointer is an offset into
//! `.data`, and [`Object::ptr_at`] reads one.

mod elf;

use crate::{Error, Result, bytes};
use elf::Relocation;

/// The 8 bytes of `0x11` alignment padding that the chunk payload carries in front of the ELF.
const PAD: [u8; 8] = [0x11; 8];
const ELF_MAGIC: [u8; 4] = [0x7F, b'E', b'L', b'F'];
/// The only relocation type the reader accepts: a 32-bit absolute pointer.
const R_MIPS_32: u8 = 2;
const WORD: usize = 4;
const WHAT: &str = "EAGL object data";

/// A symbol from `.symtab`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Symbol {
    /// The symbol name, for example `__Bone:::rig.root`.
    pub name: String,
    /// The symbol value. In these objects it is an offset into `.data`.
    pub value: u32,
    /// The symbol size in bytes.
    pub size: u32,
    /// The index of the section the symbol belongs to (0 when undefined).
    pub section: u16,
    /// The `st_info` byte: the binding in the high nibble and the type in the low nibble.
    pub info: u8,
}

/// An EAGL object with its `.data` section relocated.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Object {
    /// The `.data` section with the relocations applied.
    pub data: Vec<u8>,
    /// Every `.symtab` entry in table order, including the null entry at index 0.
    pub symbols: Vec<Symbol>,
    /// The `.data` offsets of the relocated words, sorted. These words are the pointers.
    pub pointers: Vec<u32>,
}

impl Object {
    /// Reads an object from a chunk payload (8 bytes of `0x11` then the ELF) or from a bare ELF.
    pub fn read(payload: &[u8]) -> Result<Self> {
        let file = strip_pad(payload)?;
        let parsed = elf::parse(file)?;
        let mut data = parsed.data.to_vec();
        let pointers = apply_relocations(&mut data, &parsed.symbols, parsed.relocations)?;
        Ok(Self { data, symbols: parsed.symbols, pointers })
    }

    /// The first symbol called `name`.
    pub fn symbol(&self, name: &str) -> Option<&Symbol> {
        self.symbols.iter().find(|symbol| symbol.name == name)
    }

    /// The symbols whose names start with `prefix`, in table order.
    pub fn symbols_with_prefix<'a>(&'a self, prefix: &'a str) -> impl Iterator<Item = &'a Symbol> + 'a {
        self.symbols.iter().filter(move |symbol| symbol.name.starts_with(prefix))
    }

    /// The byte at `offset` in `.data`.
    pub fn u8_at(&self, offset: usize) -> Result<u8> {
        bytes::u8_at(&self.data, offset, WHAT)
    }

    /// The little-endian u16 at `offset` in `.data`.
    pub fn u16_at(&self, offset: usize) -> Result<u16> {
        bytes::u16_at(&self.data, offset, WHAT)
    }

    /// The little-endian u32 at `offset` in `.data`.
    pub fn u32_at(&self, offset: usize) -> Result<u32> {
        bytes::u32_at(&self.data, offset, WHAT)
    }

    /// The little-endian f32 at `offset` in `.data`.
    pub fn f32_at(&self, offset: usize) -> Result<f32> {
        bytes::f32_at(&self.data, offset, WHAT)
    }

    /// Whether the word at `offset` was relocated, so that it holds a pointer.
    pub fn is_pointer(&self, offset: usize) -> bool {
        u32::try_from(offset).is_ok_and(|offset| self.pointers.binary_search(&offset).is_ok())
    }

    /// The pointer at `offset`. Returns `None` for a zero word that was not relocated. Otherwise returns the
    /// target offset in `.data`. A target past the end of `.data` is an error; a target equal to the end is
    /// accepted.
    pub fn ptr_at(&self, offset: usize) -> Result<Option<u32>> {
        let word = self.u32_at(offset)?;
        if word == 0 && !self.is_pointer(offset) {
            return Ok(None);
        }
        if word as usize > self.data.len() {
            return Err(Error::Invalid(format!(
                "pointer at {offset:#x} targets {word:#x}, past the {:#x} bytes of .data",
                self.data.len()
            )));
        }
        Ok(Some(word))
    }

    /// The NUL-terminated ASCII string at `offset` in `.data`, without the NUL.
    pub fn cstr_at(&self, offset: usize) -> Result<&str> {
        bytes::cstr_at(&self.data, offset, "string")
    }
}

/// Drops the 8-byte pad when the payload has one, and checks that the ELF magic follows.
fn strip_pad(payload: &[u8]) -> Result<&[u8]> {
    let file = payload.strip_prefix(&PAD).unwrap_or(payload);
    if !file.starts_with(&ELF_MAGIC) {
        return Err(Error::BadMagic);
    }
    Ok(file)
}

/// Adds each relocation's symbol value to the word it points at. Returns the sorted offsets of those words.
fn apply_relocations(data: &mut [u8], symbols: &[Symbol], mut relocations: Vec<Relocation>) -> Result<Vec<u32>> {
    relocations.sort_by_key(|reloc| reloc.offset);
    if let Some(pair) = relocations.windows(2).find(|pair| pair[0].offset == pair[1].offset) {
        return Err(Error::Invalid(format!("two relocations at {:#x}", pair[0].offset)));
    }
    let mut pointers = Vec::with_capacity(relocations.len());
    for reloc in relocations {
        apply_one(data, symbols, reloc)?;
        pointers.push(reloc.offset);
    }
    Ok(pointers)
}

fn apply_one(data: &mut [u8], symbols: &[Symbol], reloc: Relocation) -> Result<()> {
    if reloc.kind != R_MIPS_32 {
        return Err(Error::Unsupported(format!("relocation type {} (only R_MIPS_32)", reloc.kind)));
    }
    let offset = reloc.offset as usize;
    let end = offset
        .checked_add(WORD)
        .filter(|&end| end <= data.len())
        .ok_or_else(|| Error::Invalid(format!("relocation at {offset:#x} is past the end of .data")))?;
    let symbol = symbols
        .get(reloc.symbol as usize)
        .ok_or_else(|| Error::Invalid(format!("relocation symbol {} is out of range", reloc.symbol)))?;
    let target = bytes::u32_at(data, offset, WHAT)?;
    let patched = target
        .checked_add(symbol.value)
        .ok_or_else(|| Error::Invalid(format!("relocation at {offset:#x} overflows the word")))?;
    data[offset..end].copy_from_slice(&patched.to_le_bytes());
    Ok(())
}

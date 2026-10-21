//! The ELF32 container: the file header, the section table, `.symtab` and the relocation sections.
//!
//! Only the little-endian `ET_REL` form is read. The field offsets are the standard ELF32 layout.

use super::Symbol;
use crate::{Error, Result, bytes};

const HEADER_LEN: usize = 52;
const SECTION_LEN: usize = 40;
const SYMBOL_LEN: usize = 16;
const REL_LEN: usize = 8;

const ELFCLASS32: u8 = 1;
const ELFDATA2LSB: u8 = 1;
const ET_REL: u16 = 1;

const SHT_SYMTAB: u32 = 2;
const SHT_RELA: u32 = 4;
const SHT_REL: u32 = 9;
const SHT_NOBITS: u32 = 8;

const HEADER: &str = "ELF header";
const SECTION_TABLE: &str = "section header table";
const SYMBOL: &str = "symbol";
const RELOCATION: &str = "relocation";

/// One `Elf32_Rel` entry, split into its fields.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct Relocation {
    /// The offset within `.data` of the word to patch.
    pub offset: u32,
    /// The index into `.symtab`.
    pub symbol: u32,
    /// The relocation type (the low byte of `r_info`).
    pub kind: u8,
}

/// What the object reader needs from the container.
pub(super) struct Parsed<'a> {
    /// The bytes of `.data`, not yet relocated.
    pub data: &'a [u8],
    /// Every `.symtab` entry, in table order.
    pub symbols: Vec<Symbol>,
    /// Every relocation that targets `.data`, in table order.
    pub relocations: Vec<Relocation>,
}

struct Header {
    shoff: usize,
    shentsize: usize,
    shnum: usize,
    shstrndx: usize,
}

struct SectionHeader {
    name: usize,
    kind: u32,
    offset: usize,
    size: usize,
    link: usize,
    info: usize,
}

/// Parses an ELF32 relocatable object. `file` starts at the ELF magic.
pub(super) fn parse(file: &[u8]) -> Result<Parsed<'_>> {
    let header = read_header(file)?;
    let sections = read_sections(file, &header)?;
    let names = section_names(file, &sections, header.shstrndx)?;
    let data_index = data_section(&names)?;
    let data = section_bytes(file, &sections[data_index])?;
    let symbols = read_symbols(file, &sections)?;
    let relocations = read_relocations(file, &sections, data_index)?;
    Ok(Parsed { data, symbols, relocations })
}

fn read_header(file: &[u8]) -> Result<Header> {
    if file.len() < HEADER_LEN {
        return Err(Error::Truncated { what: HEADER, offset: file.len() });
    }
    let class = bytes::u8_at(file, 4, HEADER)?;
    let encoding = bytes::u8_at(file, 5, HEADER)?;
    let kind = bytes::u16_at(file, 16, HEADER)?;
    if class != ELFCLASS32 {
        return Err(Error::Unsupported(format!("ELF class {class} (only ELF32)")));
    }
    if encoding != ELFDATA2LSB {
        return Err(Error::Unsupported("big-endian ELF".to_string()));
    }
    if kind != ET_REL {
        return Err(Error::Unsupported(format!("ELF type {kind} (only relocatable objects)")));
    }
    Ok(Header {
        shoff: bytes::u32_at(file, 32, HEADER)? as usize,
        shentsize: bytes::u16_at(file, 46, HEADER)? as usize,
        shnum: bytes::u16_at(file, 48, HEADER)? as usize,
        shstrndx: bytes::u16_at(file, 50, HEADER)? as usize,
    })
}

fn read_sections(file: &[u8], header: &Header) -> Result<Vec<SectionHeader>> {
    if header.shentsize < SECTION_LEN {
        return Err(Error::Invalid(format!("section header entry size {} is below {SECTION_LEN}", header.shentsize)));
    }
    let table = bytes::slice(file, header.shoff, header.shentsize * header.shnum, SECTION_TABLE)?;
    table.chunks_exact(header.shentsize).map(read_section).collect()
}

fn read_section(entry: &[u8]) -> Result<SectionHeader> {
    Ok(SectionHeader {
        name: bytes::u32_at(entry, 0, SECTION_TABLE)? as usize,
        kind: bytes::u32_at(entry, 4, SECTION_TABLE)?,
        offset: bytes::u32_at(entry, 16, SECTION_TABLE)? as usize,
        size: bytes::u32_at(entry, 20, SECTION_TABLE)? as usize,
        link: bytes::u32_at(entry, 24, SECTION_TABLE)? as usize,
        info: bytes::u32_at(entry, 28, SECTION_TABLE)? as usize,
    })
}

/// The contents of a section. `SHT_NOBITS` sections have no bytes in the file.
fn section_bytes<'a>(file: &'a [u8], section: &SectionHeader) -> Result<&'a [u8]> {
    if section.kind == SHT_NOBITS {
        return Ok(&[]);
    }
    bytes::slice(file, section.offset, section.size, "section contents")
}

fn section_names<'a>(file: &'a [u8], sections: &[SectionHeader], shstrndx: usize) -> Result<Vec<&'a str>> {
    let names = sections
        .get(shstrndx)
        .ok_or_else(|| Error::Invalid(format!("section name table index {shstrndx} is out of range")))?;
    let table = section_bytes(file, names)?;
    sections.iter().map(|section| bytes::cstr_at(table, section.name, "section name")).collect()
}

fn data_section(names: &[&str]) -> Result<usize> {
    names.iter().position(|name| *name == ".data").ok_or_else(|| Error::Invalid("no .data section".to_string()))
}

fn read_symbols(file: &[u8], sections: &[SectionHeader]) -> Result<Vec<Symbol>> {
    let symtab = match sections.iter().find(|section| section.kind == SHT_SYMTAB) {
        Some(section) => section,
        None => return Ok(Vec::new()),
    };
    let strtab = sections
        .get(symtab.link)
        .ok_or_else(|| Error::Invalid(format!("symbol string table index {} is out of range", symtab.link)))?;
    let strings = section_bytes(file, strtab)?;
    let table = section_bytes(file, symtab)?;
    check_whole_entries(table, SYMBOL_LEN, "symbol table")?;
    table.as_chunks::<SYMBOL_LEN>().0.iter().map(|entry| read_symbol(entry, strings)).collect()
}

fn read_symbol(entry: &[u8], strings: &[u8]) -> Result<Symbol> {
    let name = bytes::u32_at(entry, 0, SYMBOL)? as usize;
    Ok(Symbol {
        name: bytes::cstr_at(strings, name, "symbol name")?.to_string(),
        value: bytes::u32_at(entry, 4, SYMBOL)?,
        size: bytes::u32_at(entry, 8, SYMBOL)?,
        info: bytes::u8_at(entry, 12, SYMBOL)?,
        section: bytes::u16_at(entry, 14, SYMBOL)?,
    })
}

/// The `SHT_REL` sections whose target is `.data`. Their entries are returned unapplied.
fn read_relocations(file: &[u8], sections: &[SectionHeader], data_index: usize) -> Result<Vec<Relocation>> {
    let mut relocations = Vec::new();
    for section in sections.iter().filter(|section| section.info == data_index) {
        if section.kind == SHT_RELA {
            return Err(Error::Unsupported("RELA relocations (only REL)".to_string()));
        }
        if section.kind != SHT_REL {
            continue;
        }
        let table = section_bytes(file, section)?;
        check_whole_entries(table, REL_LEN, "relocation table")?;
        for entry in table.as_chunks::<REL_LEN>().0 {
            let info = bytes::u32_at(entry, 4, RELOCATION)?;
            relocations.push(Relocation {
                offset: bytes::u32_at(entry, 0, RELOCATION)?,
                symbol: info >> 8,
                kind: (info & 0xFF) as u8,
            });
        }
    }
    Ok(relocations)
}

/// Rejects a table whose length is not a whole number of `entry_len` records.
fn check_whole_entries(table: &[u8], entry_len: usize, what: &'static str) -> Result<()> {
    let whole = table.len() - table.len() % entry_len;
    if whole != table.len() {
        return Err(Error::Truncated { what, offset: whole });
    }
    Ok(())
}

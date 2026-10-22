//! Builds small ELF32 little-endian relocatable objects in the EAGL layout, for the unit tests.
//!
//! The sections are `.data`, `.rel.data`, `.symtab`, `.strtab` and `.shstrtab`, followed by the section header
//! table. Symbol 0 is the null symbol and symbol 1 is a section symbol for `.data` at value 0; `reloc` points at
//! it, as the relocations of the install do. User symbols follow from index 2, in the order they were added.

const PAD: [u8; 8] = [0x11; 8];
const HEADER_LEN: usize = 52;
const SECTION_LEN: usize = 40;
const SYMBOL_LEN: usize = 16;
const R_MIPS_32: u8 = 2;
const SECTION_SYMBOL: u32 = 1;
const FIRST_USER_SYMBOL: u32 = 2;
const DATA_SECTION: u16 = 1;

/// The contents of an object before it is serialised.
pub struct Builder {
    data: Vec<u8>,
    symbols: Vec<(String, u32)>,
    relocations: Vec<(u32, u32, u8)>,
}

impl Builder {
    /// An object whose `.data` is `data`.
    pub fn new(data: &[u8]) -> Self {
        Self { data: data.to_vec(), symbols: Vec::new(), relocations: Vec::new() }
    }

    /// Adds a global symbol `name` with the value `value` (an offset into `.data`).
    pub fn symbol(mut self, name: &str, value: u32) -> Self {
        self.symbols.push((name.to_string(), value));
        self
    }

    /// Adds an `R_MIPS_32` relocation at `offset`, against the `.data` section symbol (value 0).
    pub fn reloc(self, offset: u32) -> Self {
        self.reloc_kind(offset, R_MIPS_32)
    }

    /// Adds a relocation of type `kind` at `offset`, against the `.data` section symbol.
    pub fn reloc_kind(mut self, offset: u32, kind: u8) -> Self {
        self.relocations.push((offset, SECTION_SYMBOL, kind));
        self
    }

    /// Adds an `R_MIPS_32` relocation at `offset`, against the symbol called `name`, which must have been added
    /// with `symbol` first.
    pub fn reloc_to(mut self, offset: u32, name: &str) -> Self {
        let position = self
            .symbols
            .iter()
            .position(|(symbol, _)| symbol == name)
            .expect("reloc_to names a symbol added with symbol()");
        self.relocations.push((offset, FIRST_USER_SYMBOL + position as u32, R_MIPS_32));
        self
    }

    /// The object behind the 8 bytes of `0x11` padding, as a chunk payload stores it.
    pub fn padded(&self) -> Vec<u8> {
        let mut out = PAD.to_vec();
        out.extend(self.bare());
        out
    }

    /// The bare ELF object, without the padding.
    pub fn bare(&self) -> Vec<u8> {
        let mut shstrtab = vec![0u8];
        let data_name = intern(&mut shstrtab, ".data");
        let rel_name = intern(&mut shstrtab, ".rel.data");
        let symtab_name = intern(&mut shstrtab, ".symtab");
        let strtab_name = intern(&mut shstrtab, ".strtab");
        let shstrtab_name = intern(&mut shstrtab, ".shstrtab");

        let mut strtab = vec![0u8];
        let mut symtab = Vec::new();
        push_symbol(&mut symtab, 0, 0, 0, 0, 0);
        push_symbol(&mut symtab, 0, 0, 0, 3, DATA_SECTION);
        for (name, value) in &self.symbols {
            let at = intern(&mut strtab, name);
            push_symbol(&mut symtab, at, *value, 4, 0x11, DATA_SECTION);
        }
        let mut rel = Vec::new();
        for (offset, symbol, kind) in &self.relocations {
            push_u32(&mut rel, *offset);
            push_u32(&mut rel, (symbol << 8) | u32::from(*kind));
        }

        let mut body = Vec::new();
        let data_at = place(&mut body, &self.data);
        let rel_at = place(&mut body, &rel);
        let symtab_at = place(&mut body, &symtab);
        let strtab_at = place(&mut body, &strtab);
        let shstrtab_at = place(&mut body, &shstrtab);
        let table_at = HEADER_LEN + body.len();

        let mut table = vec![0u8; SECTION_LEN];
        table.extend(section(data_name, 1, data_at, self.data.len(), 0, 0, 0));
        table.extend(section(rel_name, 9, rel_at, rel.len(), 3, 1, 8));
        table.extend(section(symtab_name, 2, symtab_at, symtab.len(), 4, 2, SYMBOL_LEN as u32));
        table.extend(section(strtab_name, 3, strtab_at, strtab.len(), 0, 0, 0));
        table.extend(section(shstrtab_name, 3, shstrtab_at, shstrtab.len(), 0, 0, 0));

        let mut out = header(table_at);
        out.extend(body);
        out.extend(table);
        out
    }
}

/// Appends `name` and its NUL to a string table and returns its offset.
fn intern(table: &mut Vec<u8>, name: &str) -> usize {
    let at = table.len();
    table.extend_from_slice(name.as_bytes());
    table.push(0);
    at
}

/// Appends `bytes` to the body, which starts at file offset `HEADER_LEN`, and returns the file offset.
fn place(body: &mut Vec<u8>, bytes: &[u8]) -> usize {
    let at = HEADER_LEN + body.len();
    body.extend_from_slice(bytes);
    at
}

fn header(table_at: usize) -> Vec<u8> {
    let mut out = vec![0x7F, b'E', b'L', b'F', 1, 1, 1, 0];
    out.extend([0u8; 8]);
    push_u16(&mut out, 1); // e_type: ET_REL
    push_u16(&mut out, 8); // e_machine: EM_MIPS
    push_u32(&mut out, 1); // e_version
    push_u32(&mut out, 0); // e_entry
    push_u32(&mut out, 0); // e_phoff
    push_u32(&mut out, table_at as u32); // e_shoff
    push_u32(&mut out, 0); // e_flags
    push_u16(&mut out, HEADER_LEN as u16);
    push_u16(&mut out, 0); // e_phentsize
    push_u16(&mut out, 0); // e_phnum
    push_u16(&mut out, SECTION_LEN as u16);
    push_u16(&mut out, 6); // e_shnum: null, .data, .rel.data, .symtab, .strtab, .shstrtab
    push_u16(&mut out, 5); // e_shstrndx
    out
}

fn section(name: usize, kind: u32, offset: usize, size: usize, link: u32, info: u32, entsize: u32) -> Vec<u8> {
    let mut out = Vec::with_capacity(SECTION_LEN);
    push_u32(&mut out, name as u32);
    push_u32(&mut out, kind);
    push_u32(&mut out, 0); // sh_flags
    push_u32(&mut out, 0); // sh_addr
    push_u32(&mut out, offset as u32);
    push_u32(&mut out, size as u32);
    push_u32(&mut out, link);
    push_u32(&mut out, info);
    push_u32(&mut out, 1); // sh_addralign
    push_u32(&mut out, entsize);
    out
}

fn push_symbol(out: &mut Vec<u8>, name: usize, value: u32, size: u32, info: u8, shndx: u16) {
    push_u32(out, name as u32);
    push_u32(out, value);
    push_u32(out, size);
    out.push(info);
    out.push(0); // st_other
    push_u16(out, shndx);
}

fn push_u16(out: &mut Vec<u8>, value: u16) {
    out.extend_from_slice(&value.to_le_bytes());
}

fn push_u32(out: &mut Vec<u8>, value: u32) {
    out.extend_from_slice(&value.to_le_bytes());
}

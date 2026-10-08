//! Errors of the bank reader and the interpreter.

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum Error {
    /// The data ends before a header, table or module it announces.
    #[error("module bank is truncated: need {needed} bytes at offset {at}, have {have}")]
    Truncated { at: usize, needed: usize, have: usize },
    /// The first four bytes are not `ABKC`.
    #[error("not a module bank (bad magic)")]
    BadMagic,
    /// A header field points outside the file or contradicts another.
    #[error("bad module bank: {0}")]
    Bad(&'static str),
    /// The module has no such number.
    #[error("no module {0} in the bank")]
    NoSuchModule(usize),
    /// The code uses an instruction the interpreter does not know.
    #[error("unknown instruction 0x{opcode:02X} in module code at offset {at}")]
    BadCode { at: usize, opcode: u8 },
    /// The code calls a node function number that does not exist.
    #[error("module code calls the unknown node function {0}")]
    UnknownFunction(u32),
    /// The code reads or writes outside the module image.
    #[error("module code or data reaches outside its image at offset {0}")]
    Fault(usize),
}

pub type Result<T> = std::result::Result<T, Error>;

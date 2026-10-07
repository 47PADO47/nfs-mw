#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum Error {
    #[error("input too short for a {0} header")]
    Truncated(&'static str),
    #[error("bad {kind} header: {detail}")]
    BadHeader { kind: &'static str, detail: String },
    #[error("corrupt {kind} stream at input 0x{input:X} / output 0x{output:X}: {detail}")]
    Corrupt { kind: &'static str, input: usize, output: usize, detail: &'static str },
}

pub type Result<T> = std::result::Result<T, Error>;

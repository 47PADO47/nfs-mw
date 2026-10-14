//! Authored FXEDIT graphs. Spec: `fxedit-processor.md`.
//! Reads bytes and processes a mono send into mono/stereo output.

mod kernel;
mod math;
mod parse;
mod processor;

#[cfg(test)]
mod tests;

pub use processor::Processor;

pub const MAX_BLOCK: usize = 256;
pub(crate) type Block = [f32; MAX_BLOCK];

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum Error {
    #[error("invalid FXEDIT patch: {0}")]
    Invalid(&'static str),
    #[error("unsupported FXEDIT processor {0}")]
    Unsupported(u8),
}

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, Clone, Copy)]
pub struct Node {
    pub opcode: u8,
    pub output_selector: u8,
    pub parameters: [i16; 3],
}

#[derive(Debug, Clone, Copy)]
pub struct Connection {
    pub source: usize,
    pub destination: usize,
    pub source_port: u8,
    pub destination_port: u8,
}

#[derive(Debug, Clone, Default)]
pub struct Layout {
    pub nodes: Vec<Node>,
    pub connections: Vec<Connection>,
}

#[derive(Debug, Clone)]
pub struct Patch {
    pub layouts: [Layout; 6],
}

impl Patch {
    pub fn parse(bytes: &[u8]) -> Result<Self> {
        parse::parse(bytes)
    }
}

pub(crate) fn invalid(reason: &'static str) -> Error {
    Error::Invalid(reason)
}

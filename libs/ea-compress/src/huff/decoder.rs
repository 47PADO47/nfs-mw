//! The stream header and the literal / run / escape loop (spec §3, §7).

use super::bits::BitReader;
use super::code::Code;
use super::filter::Filter;
use super::{KIND, bad_header};
use crate::{Error, HEADER_LEN, Result};

pub(super) struct Decoder<'a> {
    bits: BitReader<'a>,
    out: Vec<u8>,
}

impl<'a> Decoder<'a> {
    pub(super) fn new(input: &'a [u8]) -> Self {
        Self { bits: BitReader::new(input), out: Vec::new() }
    }

    pub(super) fn run(mut self, expected: usize) -> Result<Vec<u8>> {
        let (filter, size) = self.stream_header()?;
        if size != expected {
            return Err(bad_header(format!("stream size {size} does not match wrapper size {expected}")));
        }
        let clue = self.read(|b| b.bits(8))? as u8;
        let code = self.read(Code::read)?;
        self.out.reserve_exact(size);

        loop {
            let byte = self.read(|b| code.decode(b))?;
            if byte != clue {
                self.emit(byte, size)?;
                continue;
            }
            // The clue symbol introduces a run, an escaped byte or the end of the stream.
            let run = self.read(BitReader::number)? as usize;
            if run > 0 {
                let &last = self.out.last().ok_or_else(|| self.corrupt("run before the first byte"))?;
                if run > size - self.out.len() {
                    return Err(self.corrupt("run past the decompressed size"));
                }
                self.out.resize(self.out.len() + run, last);
            } else if self.read(|b| b.bits(1))? == 1 {
                break;
            } else {
                let byte = self.read(|b| b.bits(8))? as u8;
                self.emit(byte, size)?;
            }
        }

        if self.out.len() != size {
            return Err(self.corrupt("end of stream before the decompressed size"));
        }
        filter.apply(&mut self.out);
        Ok(self.out)
    }

    /// Type id and sizes. Returns the filter and this stream's decompressed size.
    fn stream_header(&mut self) -> Result<(Filter, usize)> {
        let id = self.read(|b| b.bits(16))?;
        let filter = match id & !0x8100 {
            0x30FB => Filter::Plain,
            0x32FB => Filter::Delta,
            0x34FB => Filter::DeltaDelta,
            _ => {
                return Err(bad_header(format!("stream type {id:04X} is not a Huffman type (30FB-35FB, B0FB-B5FB)")));
            }
        };
        let width = if id & 0x8000 != 0 { 32 } else { 24 };
        if id & 0x0100 != 0 {
            self.read(|b| b.bits(width))?; // total size of a multi-stream ("composite") object
        }
        Ok((filter, self.read(|b| b.bits(width))? as usize))
    }

    /// Run a bit-level read, turning its error into [`Error::Corrupt`] with positions.
    fn read<T>(&mut self, f: impl FnOnce(&mut BitReader<'a>) -> std::result::Result<T, &'static str>) -> Result<T> {
        f(&mut self.bits).map_err(|detail| self.corrupt(detail))
    }

    fn emit(&mut self, byte: u8, size: usize) -> Result<()> {
        if self.out.len() == size {
            return Err(self.corrupt("output past the decompressed size"));
        }
        self.out.push(byte);
        Ok(())
    }

    fn corrupt(&self, detail: &'static str) -> Error {
        Error::Corrupt { kind: KIND, input: HEADER_LEN + self.bits.bit_pos() / 8, output: self.out.len(), detail }
    }
}

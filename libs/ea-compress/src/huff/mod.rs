//! HUFF: EA's canonical-Huffman + run-length codec (EA compression-library
//! stream type `0x30FB`), used by EA Black Box games for compressed textures,
//! UI packages and AttribSys data.
//!
//! Implemented from our own spec, `docs/formats/huff.md`. No third-party code
//! was copied; the sources read to write that spec are listed there.
//!
//! Layout of this module:
//! - [`bits`]: the MSB-first bit reader and the variable-length number code;
//! - [`code`]: reading the canonical Huffman table and decoding symbols;
//! - [`filter`]: the delta post-filters of the `32FB` / `34FB` variants;
//! - [`decoder`]: the stream header and the literal / run / escape loop.

mod bits;
mod code;
mod decoder;
mod filter;

#[cfg(test)]
mod tests;

use crate::{Error, HEADER_LEN, HUFF_MAGIC, Header, Result};

const KIND: &str = "HUFF";

fn bad_header(detail: String) -> Error {
    Error::BadHeader { kind: KIND, detail }
}

/// Decompress a HUFF blob (16-byte `HUFF` wrapper header followed by the EA stream).
pub fn decompress(data: &[u8]) -> Result<Vec<u8>> {
    let h = Header::parse(data, KIND)?;
    if h.magic != HUFF_MAGIC || h.version != 0x01 || h.flags != 0x10 {
        return Err(bad_header(format!("magic/version/flags {:08X}/{:02X}/{:02X}", h.magic, h.version, h.flags)));
    }
    // Unlike JDLZ's, this size field does not include the 16-byte header.
    let end = HEADER_LEN.saturating_add(h.compressed_size as usize).min(data.len());
    decoder::Decoder::new(&data[HEADER_LEN..end]).run(h.decompressed_size as usize)
}

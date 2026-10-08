//! Builders for synthetic EA streams.

/// `tag, len, big-endian value` with the shortest length that holds `value`.
pub fn tag(id: u8, value: u32) -> Vec<u8> {
    let bytes = value.to_be_bytes();
    let skip = bytes.iter().take_while(|&&b| b == 0).count().min(3);
    let mut out = vec![id, (4 - skip) as u8];
    out.extend_from_slice(&bytes[skip..]);
    out
}

/// A `PT` header (platform marker plus tags plus the end tag).
pub fn pt_header(platform: u16, tags: &[Vec<u8>]) -> Vec<u8> {
    let mut out = vec![b'P', b'T'];
    out.extend_from_slice(&platform.to_le_bytes());
    out.extend(tags.iter().flatten());
    out.push(0xFF);
    out
}

/// A `GSTR` header.
pub fn gstr_header(tags: &[Vec<u8>]) -> Vec<u8> {
    let mut out = b"GSTR".to_vec();
    out.extend_from_slice(&[1, 0, 0, 0x30]);
    out.extend(tags.iter().flatten());
    out.push(0xFF);
    out
}

/// A block: tag, little-endian size including the 8 prefix bytes, payload.
pub fn block(id: &[u8; 4], payload: &[u8]) -> Vec<u8> {
    let mut out = id.to_vec();
    out.extend_from_slice(&(payload.len() as u32 + 8).to_le_bytes());
    out.extend_from_slice(payload);
    out
}

/// An `SCDl` block: sample count, one offset per channel (relative to the end of the table), data.
pub fn scdl(samples: u32, offsets: &[u32], data: &[u8], big_endian: bool) -> Vec<u8> {
    let word = |v: u32| if big_endian { v.to_be_bytes() } else { v.to_le_bytes() };
    let mut payload = word(samples).to_vec();
    for &o in offsets {
        payload.extend_from_slice(&word(o));
    }
    payload.extend_from_slice(data);
    block(b"SCDl", &payload)
}

/// A complete stream: `SCHl`, `SCCl`, the given blocks, `SCEl`.
pub fn stream(header: &[u8], blocks: &[Vec<u8>]) -> Vec<u8> {
    let mut out = block(b"SCHl", header);
    out.extend(block(b"SCCl", &(blocks.len() as u32).to_le_bytes()));
    out.extend(blocks.iter().flatten());
    out.extend(block(b"SCEl", &[]));
    out
}

/// A revision-2 EA-XA PCM frame: marker, two history words, 28 big-endian samples.
pub fn xa_pcm_frame(h1: i16, h2: i16, samples: &[i16; 28]) -> Vec<u8> {
    let mut out = vec![0xEE];
    out.extend_from_slice(&h1.to_be_bytes());
    out.extend_from_slice(&h2.to_be_bytes());
    for s in samples {
        out.extend_from_slice(&s.to_be_bytes());
    }
    out
}

/// A 15-byte ADPCM frame whose 28 nibbles are all `nibble`.
pub fn xa_adpcm_frame(info: u8, nibble: u8) -> Vec<u8> {
    let mut out = vec![info];
    out.extend(std::iter::repeat_n(nibble << 4 | nibble, 14));
    out
}

/// A ramp of 28 samples starting at `start`.
pub fn ramp(start: i16) -> [i16; 28] {
    std::array::from_fn(|i| start + i as i16 * 10)
}

//! Helpers that build synthetic movies.

/// One block: tag, little-endian size, payload.
pub fn block(tag: &[u8; 4], payload: &[u8]) -> Vec<u8> {
    let mut out = tag.to_vec();
    out.extend_from_slice(&(payload.len() as u32 + 8).to_le_bytes());
    out.extend_from_slice(payload);
    out
}

/// An `MVhd` block for a 64x32 movie.
pub fn mvhd(frames: u32, rate: u32, scale: u32) -> Vec<u8> {
    let mut p = vec![0x30, 0x36, 0x50, 0x56];
    p.extend_from_slice(&64u16.to_le_bytes());
    p.extend_from_slice(&32u16.to_le_bytes());
    p.extend_from_slice(&frames.to_le_bytes());
    p.extend_from_slice(&100u32.to_le_bytes());
    p.extend_from_slice(&rate.to_le_bytes());
    p.extend_from_slice(&scale.to_le_bytes());
    block(b"MVhd", &p)
}

/// An `SCDl` block with the given sample count and filler data.
pub fn scdl(samples: u32, filler: u8) -> Vec<u8> {
    let mut p = samples.to_be_bytes().to_vec();
    p.extend_from_slice(&[filler; 8]);
    block(b"SCDl", &p)
}

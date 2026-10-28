//! Splitting a Wwise sound bank (`.bnk`) into the media it embeds.
//!
//! A user bank such as a bike's engine bank holds its recordings inside: a `DIDX` chunk lists `(media id, offset, size)`
//! and a `DATA` chunk holds the bytes (`docs/formats/ride6-audio.md`). This module reads only those two chunks and the
//! bank header; the object hierarchy (`HIRC`) is not parsed. Each media is a `.wem` file that `vgmstream-cli` decodes to
//! WAV.

use anyhow::{Result, bail, ensure};

/// One embedded recording.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EmbeddedMedia {
    pub id: u32,
    /// The Wwise media file (RIFF `.wem`).
    pub bytes: Vec<u8>,
}

/// What the bank header and index say.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Bank {
    /// The bank format version (the Wwise release it was built for).
    pub version: u32,
    pub bank_id: u32,
    pub media: Vec<EmbeddedMedia>,
}

fn word(bytes: &[u8], at: usize) -> Result<u32> {
    let Some(slice) = bytes.get(at..at + 4) else { bail!("bank ends inside a chunk") };
    Ok(u32::from_le_bytes([slice[0], slice[1], slice[2], slice[3]]))
}

/// Read a bank's header and embedded media.
pub fn parse(bytes: &[u8]) -> Result<Bank> {
    let mut at = 0;
    let (mut version, mut bank_id) = (None, None);
    let mut index = None;
    let mut data = None;
    while at + 8 <= bytes.len() {
        let size = word(bytes, at + 4)? as usize;
        let start = at + 8;
        let Some(body) = bytes.get(start..start + size) else { bail!("a chunk runs past the end of the bank") };
        match &bytes[at..at + 4] {
            b"BKHD" => {
                version = Some(word(body, 0)?);
                bank_id = Some(word(body, 4)?);
            }
            b"DIDX" => index = Some(body),
            b"DATA" => data = Some(body),
            _ => {}
        }
        at = start + size;
    }
    let (Some(version), Some(bank_id)) = (version, bank_id) else { bail!("not a Wwise bank: no BKHD chunk") };
    let mut media = Vec::new();
    if let (Some(index), Some(data)) = (index, data) {
        ensure!(index.len().is_multiple_of(12), "DIDX is not a whole number of entries");
        for entry in index.as_chunks::<12>().0 {
            let (id, offset, size) = (word(entry, 0)?, word(entry, 4)? as usize, word(entry, 8)? as usize);
            let Some(blob) = data.get(offset..offset + size) else { bail!("media {id} lies outside the DATA chunk") };
            media.push(EmbeddedMedia { id, bytes: blob.to_vec() });
        }
    }
    Ok(Bank { version, bank_id, media })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn chunk(tag: &[u8; 4], body: &[u8]) -> Vec<u8> {
        let mut out = tag.to_vec();
        out.extend_from_slice(&(body.len() as u32).to_le_bytes());
        out.extend_from_slice(body);
        out
    }

    fn bank() -> Vec<u8> {
        let mut header = Vec::new();
        for value in [145u32, 77, 0, 0] {
            header.extend_from_slice(&value.to_le_bytes());
        }
        let mut index = Vec::new();
        for value in [11u32, 0, 4, 12, 9, 3] {
            index.extend_from_slice(&value.to_le_bytes());
        }
        let mut file = chunk(b"BKHD", &header);
        file.extend(chunk(b"DIDX", &index));
        file.extend(chunk(b"DATA", b"AAAA....BBB"));
        file.extend(chunk(b"HIRC", &[0, 0, 0, 0]));
        file
    }

    #[test]
    fn media_are_cut_out_of_the_data_chunk() {
        // The second entry (offset 9, size 3) ends past the 11-byte DATA chunk and is refused.
        assert!(parse(&bank()).is_err());
    }

    #[test]
    fn a_well_formed_bank_is_split() {
        let mut header = Vec::new();
        for value in [145u32, 77, 0, 0] {
            header.extend_from_slice(&value.to_le_bytes());
        }
        let mut index = Vec::new();
        for value in [11u32, 0, 4, 12, 8, 3] {
            index.extend_from_slice(&value.to_le_bytes());
        }
        let mut file = chunk(b"BKHD", &header);
        file.extend(chunk(b"DIDX", &index));
        file.extend(chunk(b"DATA", b"AAAA....BBB"));
        let bank = parse(&file).unwrap();
        assert_eq!((bank.version, bank.bank_id), (145, 77));
        assert_eq!(bank.media.len(), 2);
        assert_eq!(bank.media[0], EmbeddedMedia { id: 11, bytes: b"AAAA".to_vec() });
        assert_eq!(bank.media[1], EmbeddedMedia { id: 12, bytes: b"BBB".to_vec() });
    }

    #[test]
    fn a_file_without_a_header_is_refused() {
        assert!(parse(&chunk(b"DATA", b"x")).is_err());
    }

    #[test]
    fn a_truncated_chunk_an_odd_index_and_a_short_header_are_refused() {
        let mut header = Vec::new();
        for value in [145u32, 77] {
            header.extend_from_slice(&value.to_le_bytes());
        }
        // The chunk says 16 bytes and the file ends after 8.
        let mut cut = chunk(b"BKHD", &header);
        cut.truncate(12);
        assert!(parse(&cut).is_err());
        // An index that is not a whole number of 12-byte entries.
        let mut odd = chunk(b"BKHD", &header);
        odd.extend(chunk(b"DIDX", &[0u8; 13]));
        odd.extend(chunk(b"DATA", &[0u8; 8]));
        assert!(parse(&odd).is_err());
        // A header too short to hold the bank id.
        assert!(parse(&chunk(b"BKHD", &[1, 0, 0, 0])).is_err());
    }

    #[test]
    fn a_bank_without_media_has_none_and_unknown_chunks_are_skipped() {
        let mut header = Vec::new();
        for value in [145u32, 5] {
            header.extend_from_slice(&value.to_le_bytes());
        }
        let mut file = chunk(b"BKHD", &header);
        file.extend(chunk(b"STID", &[1, 2, 3]));
        file.extend(chunk(b"HIRC", &[0, 0, 0, 0]));
        let bank = parse(&file).unwrap();
        assert_eq!((bank.version, bank.bank_id, bank.media.len()), (145, 5, 0));
    }

    #[test]
    fn an_entry_with_an_offset_past_the_data_is_refused_not_a_panic() {
        let mut header = Vec::new();
        for value in [145u32, 77] {
            header.extend_from_slice(&value.to_le_bytes());
        }
        let mut index = Vec::new();
        for value in [1u32, u32::MAX, 4] {
            index.extend_from_slice(&value.to_le_bytes());
        }
        let mut file = chunk(b"BKHD", &header);
        file.extend(chunk(b"DIDX", &index));
        file.extend(chunk(b"DATA", b"abcd"));
        assert!(parse(&file).is_err());
    }
}
